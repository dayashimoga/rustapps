#![forbid(unsafe_code)]
//! Real-Time Filesystem Watcher with Debounced Event Coalescing

use crate::models::WatchEvent;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::broadcast;

/// Real-time directory watcher
pub struct DirectoryWatcher {
    is_running: Arc<AtomicBool>,
    event_tx: broadcast::Sender<WatchEvent>,
}

impl DirectoryWatcher {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            event_tx,
        }
    }

    /// Subscribe to real-time filesystem change events
    pub fn subscribe(&self) -> broadcast::Receiver<WatchEvent> {
        self.event_tx.subscribe()
    }

    /// Stop monitoring
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Start watching a directory path with debounced polling
    pub fn start_watching<P: AsRef<Path>>(&self, target_dir: P, interval_ms: u64) {
        let path = target_dir.as_ref().to_path_buf();
        self.is_running.store(true, Ordering::SeqCst);
        let is_running = Arc::clone(&self.is_running);
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut snapshot: HashMap<PathBuf, SystemTime> = HashMap::new();

            // Initial snapshot
            if let Ok(entries) = fs::read_dir(&path) {
                for entry_res in entries.flatten() {
                    if let Ok(meta) = entry_res.metadata() {
                        if let Ok(mod_time) = meta.modified() {
                            snapshot.insert(entry_res.path(), mod_time);
                        }
                    }
                }
            }

            while is_running.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;

                if !path.exists() {
                    break;
                }

                let mut current_snapshot: HashMap<PathBuf, SystemTime> = HashMap::new();
                if let Ok(entries) = fs::read_dir(&path) {
                    for entry_res in entries.flatten() {
                        let entry_path = entry_res.path();
                        if let Ok(meta) = entry_res.metadata() {
                            if let Ok(mod_time) = meta.modified() {
                                current_snapshot.insert(entry_path.clone(), mod_time);

                                match snapshot.get(&entry_path) {
                                    None => {
                                        let _ = event_tx.send(WatchEvent::Created(entry_path));
                                    }
                                    Some(&prev_time) if prev_time != mod_time => {
                                        let _ = event_tx.send(WatchEvent::Modified(entry_path));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                // Detect deleted files
                for old_path in snapshot.keys() {
                    if !current_snapshot.contains_key(old_path) {
                        let _ = event_tx.send(WatchEvent::Deleted(old_path.clone()));
                    }
                }

                snapshot = current_snapshot;
            }
        });
    }
}

impl Default for DirectoryWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_watcher_detects_creation_and_deletion() {
        let dir = tempdir().unwrap();
        let watcher = DirectoryWatcher::new();
        let mut rx = watcher.subscribe();

        watcher.start_watching(dir.path(), 50);

        // Allow watcher to take initial snapshot
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Create a new file
        let new_file = dir.path().join("watch_me.txt");
        {
            let mut file = File::create(&new_file).unwrap();
            file.write_all(b"watched").unwrap();
        }

        // Wait for created event
        let evt = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await;
        assert!(evt.is_ok());
        if let Ok(Ok(WatchEvent::Created(p))) = evt {
            assert_eq!(p.file_name().unwrap(), "watch_me.txt");
        } else {
            panic!("Expected WatchEvent::Created");
        }

        // Delete the file
        fs::remove_file(&new_file).unwrap();

        let del_evt = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await;
        assert!(del_evt.is_ok());
        if let Ok(Ok(WatchEvent::Deleted(p))) = del_evt {
            assert_eq!(p.file_name().unwrap(), "watch_me.txt");
        } else {
            panic!("Expected WatchEvent::Deleted");
        }

        watcher.stop();
    }
}
