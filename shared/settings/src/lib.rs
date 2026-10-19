#![forbid(unsafe_code)]
//! Shared Settings Subsystem
//!
//! Provides schema-validated persistent configuration management
//! with default fallbacks and reactive broadcast notifications.

use serde::de::DeserializeOwned;
use serde::Serialize;
use shared_storage::atomic_write_file;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};

/// Settings error definitions
#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("Serialization / Deserialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Storage error: {0}")]
    Storage(#[from] shared_storage::StorageError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Global standard application settings
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub theme: String,
    pub telemetry_enabled: bool,
    pub auto_lock_timeout_secs: u32,
    pub font_size: u32,
    pub tail_buffer_lines: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            // Nonnegotiable: telemetry strictly disabled by default
            telemetry_enabled: false,
            auto_lock_timeout_secs: 300,
            font_size: 14,
            tail_buffer_lines: 5000,
        }
    }
}

/// Thread-safe, reactive settings manager
pub struct SettingsManager<
    T: Serialize + DeserializeOwned + Default + Clone + Send + Sync + 'static,
> {
    path: PathBuf,
    current: Arc<RwLock<T>>,
    change_tx: broadcast::Sender<T>,
}

impl<T: Serialize + DeserializeOwned + Default + Clone + Send + Sync + 'static> SettingsManager<T> {
    /// Load existing settings from disk or initialize with default values.
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Result<Self, SettingsError> {
        let path_buf = path.as_ref().to_path_buf();
        let (tx, _rx) = broadcast::channel(32);

        let settings = if path_buf.exists() {
            match std::fs::read_to_string(&path_buf) {
                Ok(content) => serde_json::from_str::<T>(&content).unwrap_or_default(),
                Err(_) => T::default(),
            }
        } else {
            let default_val = T::default();
            let json = serde_json::to_vec_pretty(&default_val)?;
            atomic_write_file(&path_buf, &json)?;
            default_val
        };

        Ok(Self {
            path: path_buf,
            current: Arc::new(RwLock::new(settings)),
            change_tx: tx,
        })
    }

    /// Retrieve a snapshot of the current settings.
    pub async fn get(&self) -> T {
        self.current.read().await.clone()
    }

    /// Update settings atomically, persisting changes to disk and notifying subscribers.
    pub async fn update(&self, new_settings: T) -> Result<(), SettingsError> {
        let json = serde_json::to_vec_pretty(&new_settings)?;
        atomic_write_file(&self.path, &json)?;

        {
            let mut writer = self.current.write().await;
            *writer = new_settings.clone();
        }

        // Broadcast to reactive listeners
        let _ = self.change_tx.send(new_settings);
        Ok(())
    }

    /// Subscribe to live settings change events.
    pub fn subscribe(&self) -> broadcast::Receiver<T> {
        self.change_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_settings_load_or_default_creates_file() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("settings.json");

        let manager = SettingsManager::<AppSettings>::load_or_default(&config_path)
            .expect("Should load or create defaults");

        let settings = manager.get().await;
        assert_eq!(settings.theme, "dark");
        assert!(!settings.telemetry_enabled); // Must be false by default
        assert!(config_path.exists());
    }

    #[tokio::test]
    async fn test_settings_update_and_subscribe() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("settings.json");

        let manager = SettingsManager::<AppSettings>::load_or_default(&config_path).unwrap();
        let mut rx = manager.subscribe();

        let mut updated = manager.get().await;
        updated.theme = "light".to_string();
        updated.font_size = 16;

        manager.update(updated.clone()).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.theme, "light");
        assert_eq!(received.font_size, 16);

        // Verify persisted to disk
        let content = std::fs::read_to_string(&config_path).unwrap();
        let from_disk: AppSettings = serde_json::from_str(&content).unwrap();
        assert_eq!(from_disk.theme, "light");
        assert_eq!(from_disk.font_size, 16);
    }
}
