#![forbid(unsafe_code)]
//! High-Level Offline Signal Telemetry & Oscilloscope Capture Service

use crate::dsp_engine::SignalDspEngine;
use crate::models::{
    CaptureSession, MathOperation, MeasurementCursors, OscilloscopeStats, SignalFileFormat,
    TriggerConfig, WaveformType,
};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OscilloscopeError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Capture session not found: {0}")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Signal parsing error: {0}")]
    Parse(String),
}

/// Core service managing multi-channel oscilloscope acquisitions, DSP pipeline, and captures
pub struct OscilloscopeService {
    conn: Arc<Mutex<Connection>>,
}

impl OscilloscopeService {
    /// Opens persistent SQLite WAL oscilloscope capture store
    pub fn open(base_dir: impl AsRef<Path>) -> Result<Self, OscilloscopeError> {
        let dir = base_dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let db_path = dir.join("oscilloscope.db");
        let conn = Connection::open(db_path)?;
        Self::init_db(conn)
    }

    /// Opens in-memory database for unit testing and instant zero-disk usage
    pub fn open_in_memory() -> Result<Self, OscilloscopeError> {
        let conn = Connection::open_in_memory()?;
        Self::init_db(conn)
    }

    fn init_db(conn: Connection) -> Result<Self, OscilloscopeError> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;

             CREATE TABLE IF NOT EXISTS captures (
                 session_id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 created_at INTEGER NOT NULL,
                 sample_rate_hz REAL NOT NULL,
                 total_samples INTEGER NOT NULL,
                 dominant_freq_hz REAL NOT NULL,
                 snr_db REAL NOT NULL,
                 packets_count INTEGER NOT NULL,
                 session_json TEXT NOT NULL
             );",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Saves or updates a signal capture session
    pub fn save_capture(&self, session: &CaptureSession) -> Result<(), OscilloscopeError> {
        let json_str = serde_json::to_string(session)?;
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO captures (
                session_id, title, created_at, sample_rate_hz, total_samples,
                dominant_freq_hz, snr_db, packets_count, session_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(session_id) DO UPDATE SET
                title = excluded.title,
                dominant_freq_hz = excluded.dominant_freq_hz,
                snr_db = excluded.snr_db,
                packets_count = excluded.packets_count,
                session_json = excluded.session_json;",
            params![
                session.session_id,
                session.title,
                session.created_at,
                session.sample_rate_hz as f64,
                session.total_samples as i64,
                session.spectrum.dominant_freq_hz as f64,
                session.spectrum.snr_db as f64,
                session.decoded_packets.len() as i64,
                json_str,
            ],
        )?;

        Ok(())
    }

    /// Retrieves capture session by ID
    pub fn get_capture(&self, session_id: &str) -> Result<CaptureSession, OscilloscopeError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT session_json FROM captures WHERE session_id = ?1;")?;
        let json_str: String = stmt
            .query_row([session_id], |r| r.get(0))
            .map_err(|_| OscilloscopeError::NotFound(session_id.to_string()))?;

        let session: CaptureSession = serde_json::from_str(&json_str)?;
        Ok(session)
    }

    /// Lists all historical oscilloscope capture sessions
    pub fn list_captures(&self) -> Result<Vec<CaptureSession>, OscilloscopeError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT session_json FROM captures ORDER BY created_at DESC;")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;

        let mut captures = Vec::new();
        for json_str in rows.flatten() {
            if let Ok(sess) = serde_json::from_str::<CaptureSession>(&json_str) {
                captures.push(sess);
            }
        }

        Ok(captures)
    }

    /// Acquires and synthesizes a live oscilloscope sweep with DSP statistics and protocol decodes
    #[allow(clippy::too_many_arguments)]
    pub fn acquire_capture(
        &self,
        title: &str,
        sample_rate_hz: f32,
        num_samples: usize,
        ch1_type: WaveformType,
        ch1_freq: f32,
        ch1_amp: f32,
        ch2_type: WaveformType,
        ch2_freq: f32,
        ch2_amp: f32,
        trigger: &TriggerConfig,
    ) -> Result<CaptureSession, OscilloscopeError> {
        let session_id = format!("scope-{}", uuid::Uuid::new_v4().simple());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let ch1_raw = SignalDspEngine::generate_synthetic_waveform(
            ch1_type,
            num_samples,
            sample_rate_hz,
            ch1_freq,
            ch1_amp,
        );

        let ch2_raw = SignalDspEngine::generate_synthetic_waveform(
            ch2_type,
            num_samples,
            sample_rate_hz,
            ch2_freq,
            ch2_amp,
        );

        // Align view to trigger condition
        let trig_offset = SignalDspEngine::find_trigger_index(&ch1_raw, trigger).unwrap_or(0);
        let ch1_samples: Vec<f32> = ch1_raw[trig_offset..].to_vec();
        let ch2_samples: Vec<f32> = ch2_raw[trig_offset..].to_vec();

        // Digital channel stream (threshold 0.5V on CH2)
        let digital_samples: Vec<u8> = ch2_samples
            .iter()
            .map(|&v| if v > 0.5 { 1 } else { 0 })
            .collect();

        // Compute signal statistics
        let stats_ch1 = SignalDspEngine::compute_statistics(&ch1_samples, sample_rate_hz);
        let stats_ch2 = SignalDspEngine::compute_statistics(&ch2_samples, sample_rate_hz);

        // Compute FFT spectrum on CH1
        let spectrum = SignalDspEngine::compute_fft_spectrum(&ch1_samples, sample_rate_hz);

        // Decode serial packets from digital channel
        let decoded_packets = SignalDspEngine::decode_uart(&digital_samples, sample_rate_hz, 9600);

        let session = CaptureSession {
            session_id,
            title: title.to_string(),
            created_at: now,
            sample_rate_hz,
            total_samples: ch1_samples.len(),
            time_per_div_ms: 1.0,
            ch1_samples,
            ch2_samples,
            math_samples: Vec::new(),
            digital_samples,
            stats_ch1,
            stats_ch2,
            spectrum,
            decoded_packets,
            cursors: Some(MeasurementCursors {
                x1_time_ms: 0.2,
                x2_time_ms: 1.2,
                delta_t_ms: 1.0,
                frequency_hz: 1000.0,
                y1_volts: -1.65,
                y2_volts: 1.65,
                delta_v_volts: 3.3,
            }),
        };

        self.save_capture(&session)?;
        Ok(session)
    }

    /// Computes channel mathematical waveform ($A+B, A-B, A*B, -A, -B$)
    pub fn compute_math_channel(
        &self,
        session_id: &str,
        op: MathOperation,
    ) -> Result<CaptureSession, OscilloscopeError> {
        let mut session = self.get_capture(session_id)?;
        session.math_samples =
            SignalDspEngine::compute_channel_math(&session.ch1_samples, &session.ch2_samples, op);
        self.save_capture(&session)?;
        Ok(session)
    }

    /// Imports external signal data from CSV, WAV, or VCD formats
    pub fn import_signal_from_data(
        &self,
        title: &str,
        format: SignalFileFormat,
        data: &[u8],
    ) -> Result<CaptureSession, OscilloscopeError> {
        let session_id = format!("scope-imp-{}", uuid::Uuid::new_v4().simple());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let (ch1_samples, ch2_samples, digital_samples, sample_rate_hz) = match format {
            SignalFileFormat::Csv => {
                let text = std::str::from_utf8(data)
                    .map_err(|e| OscilloscopeError::Parse(e.to_string()))?;
                let (ch1, ch2, rate) =
                    SignalDspEngine::parse_csv_signal(text).map_err(OscilloscopeError::Parse)?;
                let digital: Vec<u8> = ch2.iter().map(|&v| if v > 0.5 { 1 } else { 0 }).collect();
                (ch1, ch2, digital, rate)
            }
            SignalFileFormat::Wav => {
                let (samples, rate) =
                    SignalDspEngine::parse_wav_signal(data).map_err(OscilloscopeError::Parse)?;
                let ch2 = vec![0.0f32; samples.len()];
                let digital: Vec<u8> = samples
                    .iter()
                    .map(|&v| if v > 0.0 { 1 } else { 0 })
                    .collect();
                (samples, ch2, digital, rate)
            }
            SignalFileFormat::Vcd => {
                let text = std::str::from_utf8(data)
                    .map_err(|e| OscilloscopeError::Parse(e.to_string()))?;
                let (digital, rate) =
                    SignalDspEngine::parse_vcd_signal(text).map_err(OscilloscopeError::Parse)?;
                let ch1: Vec<f32> = digital
                    .iter()
                    .map(|&b| if b == 1 { 3.3 } else { 0.0 })
                    .collect();
                let ch2 = vec![0.0f32; digital.len()];
                (ch1, ch2, digital, rate)
            }
        };

        let stats_ch1 = SignalDspEngine::compute_statistics(&ch1_samples, sample_rate_hz);
        let stats_ch2 = SignalDspEngine::compute_statistics(&ch2_samples, sample_rate_hz);
        let spectrum = SignalDspEngine::compute_fft_spectrum(&ch1_samples, sample_rate_hz);
        let decoded_packets = SignalDspEngine::decode_uart(&digital_samples, sample_rate_hz, 9600);

        let session = CaptureSession {
            session_id,
            title: title.to_string(),
            created_at: now,
            sample_rate_hz,
            total_samples: ch1_samples.len(),
            time_per_div_ms: 1.0,
            ch1_samples,
            ch2_samples,
            math_samples: Vec::new(),
            digital_samples,
            stats_ch1,
            stats_ch2,
            spectrum,
            decoded_packets,
            cursors: None,
        };

        self.save_capture(&session)?;
        Ok(session)
    }

    /// Updates interactive cursor coordinates and delta calculations
    pub fn update_measurement_cursors(
        &self,
        session_id: &str,
        mut cursors: MeasurementCursors,
    ) -> Result<CaptureSession, OscilloscopeError> {
        let mut session = self.get_capture(session_id)?;
        cursors.delta_t_ms = (cursors.x2_time_ms - cursors.x1_time_ms).abs();
        cursors.frequency_hz = if cursors.delta_t_ms > 1e-6 {
            1000.0 / cursors.delta_t_ms
        } else {
            0.0
        };
        cursors.delta_v_volts = (cursors.y2_volts - cursors.y1_volts).abs();
        session.cursors = Some(cursors);
        self.save_capture(&session)?;
        Ok(session)
    }

    /// Retrieves raw little-endian IEEE 754 f32 sample bytes for fast binary IPC streaming
    pub fn get_channel_waveform_raw(
        &self,
        session_id: &str,
        channel: &str,
    ) -> Result<Vec<u8>, OscilloscopeError> {
        let session = self.get_capture(session_id)?;
        let samples = match channel.to_lowercase().as_str() {
            "ch2" => &session.ch2_samples,
            "math" => &session.math_samples,
            _ => &session.ch1_samples,
        };

        let mut bytes = Vec::with_capacity(samples.len() * 4);
        for &s in samples {
            bytes.extend_from_slice(&s.to_le_bytes());
        }

        Ok(bytes)
    }

    /// Aggregates global telemetry and laboratory metrics
    pub fn get_stats(&self) -> Result<OscilloscopeStats, OscilloscopeError> {
        let conn = self.conn.lock().unwrap();

        let total_captures: usize =
            conn.query_row("SELECT COUNT(*) FROM captures;", [], |r| r.get(0))?;

        let total_samples: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_samples), 0) FROM captures;",
            [],
            |r| r.get(0),
        )?;

        let total_packets: i64 = conn.query_row(
            "SELECT COALESCE(SUM(packets_count), 0) FROM captures;",
            [],
            |r| r.get(0),
        )?;

        let avg_freq: f64 = conn.query_row(
            "SELECT COALESCE(AVG(dominant_freq_hz), 0.0) FROM captures;",
            [],
            |r| r.get(0),
        )?;

        let avg_snr: f64 = conn.query_row(
            "SELECT COALESCE(AVG(snr_db), 0.0) FROM captures;",
            [],
            |r| r.get(0),
        )?;

        Ok(OscilloscopeStats {
            total_captures,
            total_samples_analyzed: total_samples as usize,
            total_packets_decoded: total_packets as usize,
            avg_dominant_freq_hz: avg_freq as f32,
            avg_snr_db: avg_snr as f32,
        })
    }

    /// Seeds an initial realistic dual-channel scientific capture session for demo inspection
    pub fn seed_demo_capture(&self) -> Result<CaptureSession, OscilloscopeError> {
        let trigger = TriggerConfig::default();
        let mut session = self.acquire_capture(
            "Laboratory Rig: 1 kHz Sensor Carrier & Serial Bus",
            50_000.0,
            1000,
            WaveformType::Sine,
            1_000.0,
            3.3,
            WaveformType::Square,
            500.0,
            3.3,
            &trigger,
        )?;

        // Precompute A + B math channel
        session.math_samples = SignalDspEngine::compute_channel_math(
            &session.ch1_samples,
            &session.ch2_samples,
            MathOperation::Add,
        );
        self.save_capture(&session)?;
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oscilloscope_service_crud_flow() {
        let service = OscilloscopeService::open_in_memory().expect("in-memory db");
        let session = service.seed_demo_capture().expect("seed capture");

        assert_eq!(
            session.title,
            "Laboratory Rig: 1 kHz Sensor Carrier & Serial Bus"
        );
        assert!(!session.ch1_samples.is_empty());
        assert!(!session.ch2_samples.is_empty());
        assert!(!session.math_samples.is_empty());
        assert!(session.stats_ch1.vpp > 0.0);
        assert!(session.spectrum.dominant_freq_hz > 800.0);

        let retrieved = service.get_capture(&session.session_id).expect("get");
        assert_eq!(retrieved.title, session.title);

        let list = service.list_captures().expect("list");
        assert_eq!(list.len(), 1);

        let stats = service.get_stats().expect("stats");
        assert_eq!(stats.total_captures, 1);
        assert!(stats.total_samples_analyzed > 0);

        // Test math channel computation
        let math_sess = service
            .compute_math_channel(&session.session_id, MathOperation::Subtract)
            .unwrap();
        assert_eq!(math_sess.math_samples.len(), session.ch1_samples.len());

        // Test cursor update
        let cursors = MeasurementCursors {
            x1_time_ms: 1.0,
            x2_time_ms: 3.0,
            delta_t_ms: 0.0,
            frequency_hz: 0.0,
            y1_volts: -1.0,
            y2_volts: 2.0,
            delta_v_volts: 0.0,
        };
        let updated = service
            .update_measurement_cursors(&session.session_id, cursors)
            .unwrap();
        let cur = updated.cursors.unwrap();
        assert!((cur.delta_t_ms - 2.0).abs() < 1e-4);
        assert!((cur.frequency_hz - 500.0).abs() < 1e-4);
        assert!((cur.delta_v_volts - 3.0).abs() < 1e-4);

        // Test raw binary byte extraction
        let raw_bytes = service
            .get_channel_waveform_raw(&session.session_id, "ch1")
            .unwrap();
        assert_eq!(raw_bytes.len(), session.ch1_samples.len() * 4);
    }
}
