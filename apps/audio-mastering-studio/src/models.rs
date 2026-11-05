#![forbid(unsafe_code)]
//! Core Domain Models for Offline AI Stem Splitter & Audio Mastering Studio

use serde::{Deserialize, Serialize};

/// 4-Stem Audio Demixing Category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StemType {
    #[default]
    Vocals,
    Drums,
    Bass,
    Other,
}

impl StemType {
    pub fn display_name(&self) -> &'static str {
        match self {
            StemType::Vocals => "Vocals / Formants",
            StemType::Drums => "Drums & Percussion",
            StemType::Bass => "Bass & Low-End",
            StemType::Other => "Accompaniment & Guitars",
        }
    }

    pub fn color_hex(&self) -> &'static str {
        match self {
            StemType::Vocals => "#ff70a6",
            StemType::Drums => "#ffaa00",
            StemType::Bass => "#79c0ff",
            StemType::Other => "#7ee787",
        }
    }
}

/// Extracted or mixed individual stem track
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StemTrack {
    pub stem_type: StemType,
    pub title: String,
    pub samples: Vec<f32>,
    pub volume_gain_db: f32,
    pub is_muted: bool,
    pub is_solo: bool,
    pub peak_db: f32,
    pub rms_db: f32,
}

impl Default for StemTrack {
    fn default() -> Self {
        Self {
            stem_type: StemType::Vocals,
            title: "Vocals".to_string(),
            samples: Vec::new(),
            volume_gain_db: 0.0,
            is_muted: false,
            is_solo: false,
            peak_db: -90.0,
            rms_db: -90.0,
        }
    }
}

/// Single band within 5-band parametric equalizer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqBand {
    pub band_index: usize,
    pub name: String,
    pub filter_type: String, // "LowShelf", "Peaking", "HighShelf"
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q_factor: f32,
}

/// Comprehensive mastering DSP chain parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasteringConfig {
    pub bands: Vec<EqBand>,
    pub compressor_threshold_db: f32,
    pub compressor_ratio: f32,
    pub compressor_attack_ms: f32,
    pub compressor_release_ms: f32,
    pub de_reverb_amount: f32,
    pub target_lufs: f32,
    pub limiter_ceiling_db: f32,
}

impl Default for MasteringConfig {
    fn default() -> Self {
        Self {
            bands: vec![
                EqBand {
                    band_index: 0,
                    name: "Sub / Bass Shelf".to_string(),
                    filter_type: "LowShelf".to_string(),
                    frequency_hz: 80.0,
                    gain_db: 1.0,
                    q_factor: 0.707,
                },
                EqBand {
                    band_index: 1,
                    name: "Low-Mid Warmth".to_string(),
                    filter_type: "Peaking".to_string(),
                    frequency_hz: 300.0,
                    gain_db: -0.5,
                    q_factor: 1.0,
                },
                EqBand {
                    band_index: 2,
                    name: "Mid Presence".to_string(),
                    filter_type: "Peaking".to_string(),
                    frequency_hz: 1200.0,
                    gain_db: 0.5,
                    q_factor: 1.2,
                },
                EqBand {
                    band_index: 3,
                    name: "High-Mid Clarity".to_string(),
                    filter_type: "Peaking".to_string(),
                    frequency_hz: 4500.0,
                    gain_db: 1.5,
                    q_factor: 1.0,
                },
                EqBand {
                    band_index: 4,
                    name: "Air High Shelf".to_string(),
                    filter_type: "HighShelf".to_string(),
                    frequency_hz: 12000.0,
                    gain_db: 2.0,
                    q_factor: 0.707,
                },
            ],
            compressor_threshold_db: -18.0,
            compressor_ratio: 2.5,
            compressor_attack_ms: 25.0,
            compressor_release_ms: 120.0,
            de_reverb_amount: 0.15,
            target_lufs: -14.0,
            limiter_ceiling_db: -0.5,
        }
    }
}

/// EBU R128 international broadcast & streaming loudness measurements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LoudnessMetrics {
    pub integrated_lufs: f32,
    pub short_term_lufs: f32,
    pub momentary_lufs: f32,
    pub loudness_range_lu: f32,
    pub true_peak_dbfs: f32,
}

/// Persistent audio studio project
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasteringProject {
    pub project_id: String,
    pub title: String,
    pub created_at: i64,
    pub sample_rate: u32,
    pub duration_seconds: f32,
    pub original_samples: Vec<f32>,
    pub stems: Vec<StemTrack>,
    pub mastering_config: MasteringConfig,
    pub mastered_samples: Vec<f32>,
    pub original_loudness: LoudnessMetrics,
    pub mastered_loudness: LoudnessMetrics,
}

/// Aggregated studio telemetry and usage analytics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StudioStats {
    pub total_projects: usize,
    pub total_stems_extracted: usize,
    pub total_audio_minutes_processed: f64,
    pub avg_integrated_lufs: f32,
}
