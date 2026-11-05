#![forbid(unsafe_code)]
//! High-Level Offline Audio Mastering & Stem Separation Service

use crate::dsp_engine::AudioDspEngine;
use crate::models::{MasteringConfig, MasteringProject, StudioStats};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MasteringError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Project not found: {0}")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Core service managing multi-track audio projects, stem separation, and EBU R128 mastering
pub struct AudioMasteringService {
    conn: Arc<Mutex<Connection>>,
}

impl AudioMasteringService {
    /// Opens persistent SQLite WAL project store
    pub fn open(base_dir: impl AsRef<Path>) -> Result<Self, MasteringError> {
        let dir = base_dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let db_path = dir.join("audio_mastering.db");
        let conn = Connection::open(db_path)?;
        Self::init_db(conn)
    }

    /// Opens in-memory database for unit testing and instant zero-disk usage
    pub fn open_in_memory() -> Result<Self, MasteringError> {
        let conn = Connection::open_in_memory()?;
        Self::init_db(conn)
    }

    fn init_db(conn: Connection) -> Result<Self, MasteringError> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;

             CREATE TABLE IF NOT EXISTS projects (
                 project_id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 created_at INTEGER NOT NULL,
                 sample_rate INTEGER NOT NULL,
                 duration_seconds REAL NOT NULL,
                 integrated_lufs REAL NOT NULL,
                 stems_count INTEGER NOT NULL,
                 project_json TEXT NOT NULL
             );",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Saves or updates a mastering project
    pub fn save_project(&self, project: &MasteringProject) -> Result<(), MasteringError> {
        let json_str = serde_json::to_string(project)?;
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO projects (
                project_id, title, created_at, sample_rate, duration_seconds,
                integrated_lufs, stems_count, project_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(project_id) DO UPDATE SET
                title = excluded.title,
                integrated_lufs = excluded.integrated_lufs,
                stems_count = excluded.stems_count,
                project_json = excluded.project_json;",
            params![
                project.project_id,
                project.title,
                project.created_at,
                project.sample_rate as i64,
                project.duration_seconds as f64,
                project.mastered_loudness.integrated_lufs as f64,
                project.stems.len() as i64,
                json_str,
            ],
        )?;

        Ok(())
    }

    /// Retrieves project by ID
    pub fn get_project(&self, project_id: &str) -> Result<MasteringProject, MasteringError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT project_json FROM projects WHERE project_id = ?1;")?;
        let json_str: String = stmt
            .query_row([project_id], |r| r.get(0))
            .map_err(|_| MasteringError::NotFound(project_id.to_string()))?;

        let project: MasteringProject = serde_json::from_str(&json_str)?;
        Ok(project)
    }

    /// Lists all historical audio studio projects
    pub fn list_projects(&self) -> Result<Vec<MasteringProject>, MasteringError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT project_json FROM projects ORDER BY created_at DESC;")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;

        let mut projects = Vec::new();
        for json_str in rows.flatten() {
            if let Ok(proj) = serde_json::from_str::<MasteringProject>(&json_str) {
                projects.push(proj);
            }
        }

        Ok(projects)
    }

    /// Creates and processes a new audio mastering project with stem separation and EBU R128 mastering
    pub fn create_project(
        &self,
        title: &str,
        samples: Vec<f32>,
        sample_rate: u32,
        config: MasteringConfig,
    ) -> Result<MasteringProject, MasteringError> {
        let project_id = format!("proj-{}", uuid::Uuid::new_v4().simple());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let duration_seconds = samples.len() as f32 / sample_rate as f32;
        let stems = AudioDspEngine::separate_stems(&samples, sample_rate);
        let original_loudness = AudioDspEngine::measure_ebu_r128_loudness(&samples);
        let (mastered_samples, mastered_loudness) =
            AudioDspEngine::master_audio(&samples, sample_rate, &config);

        let project = MasteringProject {
            project_id,
            title: title.to_string(),
            created_at: now,
            sample_rate,
            duration_seconds,
            original_samples: samples,
            stems,
            mastering_config: config,
            mastered_samples,
            original_loudness,
            mastered_loudness,
        };

        self.save_project(&project)?;
        Ok(project)
    }

    /// Re-runs mastering chain with updated parameters
    pub fn remaster_project(
        &self,
        project_id: &str,
        new_config: MasteringConfig,
    ) -> Result<MasteringProject, MasteringError> {
        let mut project = self.get_project(project_id)?;
        let (mastered, loudness) = AudioDspEngine::master_audio(
            &project.original_samples,
            project.sample_rate,
            &new_config,
        );

        project.mastering_config = new_config;
        project.mastered_samples = mastered;
        project.mastered_loudness = loudness;

        self.save_project(&project)?;
        Ok(project)
    }

    /// Aggregates studio performance and analytics metrics
    pub fn get_stats(&self) -> Result<StudioStats, MasteringError> {
        let conn = self.conn.lock().unwrap();

        let total_projects: usize =
            conn.query_row("SELECT COUNT(*) FROM projects;", [], |r| r.get(0))?;

        let total_stems: i64 = conn.query_row(
            "SELECT COALESCE(SUM(stems_count), 0) FROM projects;",
            [],
            |r| r.get(0),
        )?;

        let total_seconds: f64 = conn.query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0.0) FROM projects;",
            [],
            |r| r.get(0),
        )?;

        let avg_lufs: f64 = conn.query_row(
            "SELECT COALESCE(AVG(integrated_lufs), -14.0) FROM projects;",
            [],
            |r| r.get(0),
        )?;

        Ok(StudioStats {
            total_projects,
            total_stems_extracted: total_stems as usize,
            total_audio_minutes_processed: total_seconds / 60.0,
            avg_integrated_lufs: avg_lufs as f32,
        })
    }

    /// Seeds an initial realistic full multi-track studio project
    pub fn seed_demo_project(&self) -> Result<MasteringProject, MasteringError> {
        let sample_rate = 44100;
        let num_samples = sample_rate as usize * 2; // 2.0 seconds demo
        let audio = AudioDspEngine::generate_synthetic_audio(num_samples, sample_rate);
        let config = MasteringConfig::default();

        self.create_project(
            "Electric Horizon — Studio Demix & Master",
            audio,
            sample_rate,
            config,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_mastering_service_crud_flow() {
        let service = AudioMasteringService::open_in_memory().expect("in-memory db");
        let project = service.seed_demo_project().expect("seed project");

        assert_eq!(project.title, "Electric Horizon — Studio Demix & Master");
        assert_eq!(project.stems.len(), 4);
        assert!(!project.mastered_samples.is_empty());
        assert!(project.mastered_loudness.integrated_lufs > -30.0);

        let retrieved = service.get_project(&project.project_id).expect("get");
        assert_eq!(retrieved.title, project.title);

        let list = service.list_projects().expect("list");
        assert_eq!(list.len(), 1);

        let stats = service.get_stats().expect("stats");
        assert_eq!(stats.total_projects, 1);
        assert_eq!(stats.total_stems_extracted, 4);
        assert!(stats.total_audio_minutes_processed > 0.0);
    }
}
