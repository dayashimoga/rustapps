#![forbid(unsafe_code)]
//! High-Level Offline Screen Recorder & Tutorial Studio Service

use crate::engine::TutorialEngine;
use crate::models::{
    ClickType, CursorEvent, ExportConfig, ExportFormat, RecordingSession, StudioStats, SubtitleCue,
};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StudioError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Recording session not found: {0}")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Core Studio service managing screen recordings, timeline processing, and exports
pub struct RecorderService {
    conn: Arc<Mutex<Connection>>,
}

impl RecorderService {
    /// Opens persistent SQLite WAL studio store
    pub fn open(base_dir: impl AsRef<Path>) -> Result<Self, StudioError> {
        let dir = base_dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let db_path = dir.join("screen_recorder.db");
        let conn = Connection::open(db_path)?;
        Self::init_db(conn)
    }

    /// Opens an in-memory studio store for unit tests and instant zero-footprint operation
    pub fn open_in_memory() -> Result<Self, StudioError> {
        let conn = Connection::open_in_memory()?;
        Self::init_db(conn)
    }

    fn init_db(conn: Connection) -> Result<Self, StudioError> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;

             CREATE TABLE IF NOT EXISTS recordings (
                 session_id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 created_at INTEGER NOT NULL,
                 duration_ms INTEGER NOT NULL,
                 width INTEGER NOT NULL,
                 height INTEGER NOT NULL,
                 fps INTEGER NOT NULL,
                 total_clicks INTEGER NOT NULL,
                 auto_zooms_count INTEGER NOT NULL,
                 silence_trimmed_ms INTEGER NOT NULL,
                 session_json TEXT NOT NULL
             );",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Saves or updates a recording session project
    pub fn save_session(&self, session: &RecordingSession) -> Result<(), StudioError> {
        let json_str = serde_json::to_string(session)?;
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO recordings (
                session_id, title, created_at, duration_ms, width, height, fps,
                total_clicks, auto_zooms_count, silence_trimmed_ms, session_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(session_id) DO UPDATE SET
                title = excluded.title,
                duration_ms = excluded.duration_ms,
                total_clicks = excluded.total_clicks,
                auto_zooms_count = excluded.auto_zooms_count,
                silence_trimmed_ms = excluded.silence_trimmed_ms,
                session_json = excluded.session_json;",
            params![
                session.session_id,
                session.title,
                session.created_at,
                session.duration_ms as i64,
                session.width,
                session.height,
                session.fps,
                session.total_clicks as i64,
                session.auto_zooms_count as i64,
                session.silence_trimmed_ms as i64,
                json_str,
            ],
        )?;

        Ok(())
    }

    /// Retrieves recording session by ID
    pub fn get_session(&self, session_id: &str) -> Result<RecordingSession, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT session_json FROM recordings WHERE session_id = ?1;")?;
        let json_str: String = stmt
            .query_row([session_id], |r| r.get(0))
            .map_err(|_| StudioError::NotFound(session_id.to_string()))?;

        let session: RecordingSession = serde_json::from_str(&json_str)?;
        Ok(session)
    }

    /// Lists all saved recording sessions ordered by creation date descending
    pub fn list_sessions(&self) -> Result<Vec<RecordingSession>, StudioError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT session_json FROM recordings ORDER BY created_at DESC;")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;

        let mut sessions = Vec::new();
        for json_str in rows.flatten() {
            if let Ok(sess) = serde_json::from_str::<RecordingSession>(&json_str) {
                sessions.push(sess);
            }
        }

        Ok(sessions)
    }

    /// Creates and synthesizes a new screen recording project with automated AI zooms and steps
    pub fn create_recording_session(
        &self,
        title: &str,
        duration_ms: u64,
        events: Vec<CursorEvent>,
        subtitles: Vec<SubtitleCue>,
    ) -> Result<RecordingSession, StudioError> {
        let session_id = format!("rec-{}", uuid::Uuid::new_v4().simple());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let total_clicks = events.iter().filter(|e| e.click != ClickType::None).count();
        let zoom_keyframes = TutorialEngine::generate_auto_zooms(&events, duration_ms, 1.45);
        let tutorial_steps = TutorialEngine::build_tutorial_steps(&events);

        // Estimate dead-air silence from synthetic audio RMS profile
        let intervals_count = (duration_ms / 100) as usize;
        let mut mock_rms = vec![-22.0f32; intervals_count];
        // Inject periodic conversational pauses
        for chunk in mock_rms.chunks_mut(60) {
            if chunk.len() > 20 {
                for sample in chunk.iter_mut().take(20) {
                    *sample = -50.0;
                }
            }
        }
        let silences = TutorialEngine::detect_silence_intervals(&mock_rms, -45.0, 1200);
        let silence_trimmed_ms: u64 = silences.iter().map(|s| s.duration_ms).sum();

        let session = RecordingSession {
            session_id,
            title: title.to_string(),
            created_at: now,
            duration_ms,
            width: 1920,
            height: 1080,
            fps: 60,
            total_clicks,
            auto_zooms_count: zoom_keyframes.len(),
            silence_trimmed_ms,
            cursor_events: events,
            zoom_keyframes,
            subtitles,
            tutorial_steps,
        };

        self.save_session(&session)?;
        Ok(session)
    }

    /// Re-synthesizes auto-zooms with adjusted scale factor
    pub fn adjust_auto_zooms(
        &self,
        session_id: &str,
        new_scale: f32,
    ) -> Result<RecordingSession, StudioError> {
        let mut session = self.get_session(session_id)?;
        session.zoom_keyframes = TutorialEngine::generate_auto_zooms(
            &session.cursor_events,
            session.duration_ms,
            new_scale,
        );
        session.auto_zooms_count = session.zoom_keyframes.len();
        self.save_session(&session)?;
        Ok(session)
    }

    /// Exports tutorial output (HTML, SRT subtitles, or config manifest)
    pub fn export_tutorial(
        &self,
        session_id: &str,
        config: &ExportConfig,
    ) -> Result<String, StudioError> {
        let session = self.get_session(session_id)?;

        match config.format {
            ExportFormat::InteractiveHtml => Ok(TutorialEngine::render_interactive_html(
                &session.title,
                &session.tutorial_steps,
                &session.subtitles,
            )),
            ExportFormat::Mp4 | ExportFormat::WebM | ExportFormat::Gif => {
                // Generate SRT subtitles accompanying video export
                Ok(TutorialEngine::format_srt_subtitles(&session.subtitles))
            }
        }
    }

    /// Retrieves aggregated studio metrics
    pub fn get_stats(&self) -> Result<StudioStats, StudioError> {
        let conn = self.conn.lock().unwrap();

        let total_recordings: usize =
            conn.query_row("SELECT COUNT(*) FROM recordings;", [], |r| r.get(0))?;

        let total_ms: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration_ms), 0) FROM recordings;",
            [],
            |r| r.get(0),
        )?;

        let total_zooms: i64 = conn.query_row(
            "SELECT COALESCE(SUM(auto_zooms_count), 0) FROM recordings;",
            [],
            |r| r.get(0),
        )?;

        let total_silence_ms: i64 = conn.query_row(
            "SELECT COALESCE(SUM(silence_trimmed_ms), 0) FROM recordings;",
            [],
            |r| r.get(0),
        )?;

        let mut stmt = conn.prepare("SELECT session_json FROM recordings;")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut total_tutorial_steps = 0;
        for json_str in rows.flatten() {
            if let Ok(sess) = serde_json::from_str::<RecordingSession>(&json_str) {
                total_tutorial_steps += sess.tutorial_steps.len();
            }
        }

        Ok(StudioStats {
            total_recordings,
            total_duration_secs: (total_ms as f64) / 1000.0,
            total_zooms_generated: total_zooms as usize,
            total_silence_saved_secs: (total_silence_ms as f64) / 1000.0,
            total_tutorial_steps,
        })
    }

    /// Seeds an initial realistic tutorial recording session for testing and demo inspection
    pub fn seed_demo_session(&self) -> Result<RecordingSession, StudioError> {
        let events = vec![
            CursorEvent {
                timestamp_ms: 500,
                x: 0.12,
                y: 0.15,
                click: ClickType::None,
            },
            CursorEvent {
                timestamp_ms: 1800,
                x: 0.22,
                y: 0.32,
                click: ClickType::Left,
            },
            CursorEvent {
                timestamp_ms: 4200,
                x: 0.65,
                y: 0.48,
                click: ClickType::Right,
            },
            CursorEvent {
                timestamp_ms: 7100,
                x: 0.82,
                y: 0.76,
                click: ClickType::Double,
            },
            CursorEvent {
                timestamp_ms: 9500,
                x: 0.50,
                y: 0.50,
                click: ClickType::Left,
            },
        ];

        let subtitles = vec![
            SubtitleCue {
                start_ms: 600,
                end_ms: 2500,
                text: "Welcome! Today we demonstrate how to configure an offline air-gapped workspace.".to_string(),
                speaker: Some("Presenter".to_string()),
            },
            SubtitleCue {
                start_ms: 3800,
                end_ms: 5500,
                text: "Right click the target module to open the context properties panel.".to_string(),
                speaker: Some("Presenter".to_string()),
            },
            SubtitleCue {
                start_ms: 6800,
                end_ms: 8500,
                text: "Double click to deploy the production package.".to_string(),
                speaker: Some("Presenter".to_string()),
            },
        ];

        self.create_recording_session(
            "Demo: Offline Air-Gapped Deployment Walkthrough",
            12_000,
            events,
            subtitles,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_service_crud_flow() {
        let service = RecorderService::open_in_memory().expect("in-memory db");
        let session = service.seed_demo_session().expect("seed session");

        assert_eq!(session.total_clicks, 4);
        assert_eq!(session.auto_zooms_count, 4);
        assert_eq!(session.tutorial_steps.len(), 4);

        let retrieved = service.get_session(&session.session_id).expect("get");
        assert_eq!(retrieved.title, session.title);

        let list = service.list_sessions().expect("list");
        assert_eq!(list.len(), 1);

        let stats = service.get_stats().expect("stats");
        assert_eq!(stats.total_recordings, 1);
        assert_eq!(stats.total_zooms_generated, 4);
        assert_eq!(stats.total_tutorial_steps, 4);

        // Adjust zoom scale
        let adjusted = service.adjust_auto_zooms(&session.session_id, 1.8).unwrap();
        assert_eq!(adjusted.zoom_keyframes[0].scale, 1.8);

        // Export interactive HTML
        let html = service
            .export_tutorial(
                &session.session_id,
                &ExportConfig {
                    format: ExportFormat::InteractiveHtml,
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(html.contains("Offline Air-Gapped Deployment"));
    }
}
