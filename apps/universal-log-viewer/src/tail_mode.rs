#![forbid(unsafe_code)]
//! Real-Time Live Tail Mode Subsystem
//!
//! Implements ADR-0010: Non-blocking tail follower with 16ms coalescing debounce
//! buffers, file append tracking, and rotation/truncation detection.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::broadcast;

/// Tail mode error definitions
#[derive(Debug, Error)]
pub enum TailError {
    #[error("I/O error observing tail file '{path}': {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Tail channel send error")]
    ChannelClosed,
}

/// Events emitted by the tail follower
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TailEvent {
    /// Newly appended bytes and lines detected
    Appended {
        bytes_added: u64,
        new_file_size: u64,
    },
    /// File size shrank (truncated or rotated)
    Truncated {
        previous_size: u64,
        current_size: u64,
    },
}

/// Follower tracking live file appends
pub struct TailFollower {
    path: PathBuf,
    is_running: Arc<AtomicBool>,
    event_tx: broadcast::Sender<TailEvent>,
}

impl TailFollower {
    /// Start following a log file with a 16ms coalescing debounce interval.
    pub fn start<P: AsRef<Path>>(path: P) -> Result<Self, TailError> {
        let path_buf = path.as_ref().to_path_buf();
        let (tx, _rx) = broadcast::channel(128);
        let is_running = Arc::new(AtomicBool::new(true));

        let initial_size = match std::fs::metadata(&path_buf) {
            Ok(m) => m.len(),
            Err(_) => 0,
        };

        let path_clone = path_buf.clone();
        let tx_clone = tx.clone();
        let running_clone = Arc::clone(&is_running);

        tokio::spawn(async move {
            let mut last_size = initial_size;

            // 16ms interval matches 60 FPS frame rate (ADR-0010)
            let poll_interval = Duration::from_millis(16);

            while running_clone.load(Ordering::Relaxed) {
                tokio::time::sleep(poll_interval).await;

                let current_metadata = match std::fs::metadata(&path_clone) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let current_size = current_metadata.len();

                if current_size > last_size {
                    let bytes_added = current_size - last_size;
                    last_size = current_size;

                    let _ = tx_clone.send(TailEvent::Appended {
                        bytes_added,
                        new_file_size: current_size,
                    });
                } else if current_size < last_size {
                    let _ = tx_clone.send(TailEvent::Truncated {
                        previous_size: last_size,
                        current_size,
                    });
                    last_size = current_size;
                }
            }
        });

        Ok(Self {
            path: path_buf,
            is_running,
            event_tx: tx,
        })
    }

    /// Subscribe to live tail update events.
    pub fn subscribe(&self) -> broadcast::Receiver<TailEvent> {
        self.event_tx.subscribe()
    }

    /// Stop the tail follower thread.
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::Relaxed);
    }

    /// Check if tail follower is currently active.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Path to followed file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TailFollower {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_tail_follower_detects_appends() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("tail_test.log");
        std::fs::write(&file_path, b"Initial Line\n").unwrap();

        let follower = TailFollower::start(&file_path).unwrap();
        let mut rx = follower.subscribe();

        // Write append to file
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&file_path)
                .unwrap();
            f.write_all(b"Appended New Line\n").unwrap();
            f.sync_all().unwrap();
        }

        // Expect tail event
        let event = tokio::time::timeout(Duration::from_millis(1000), rx.recv())
            .await
            .expect("Should receive event within timeout")
            .expect("Channel should be open");

        match event {
            TailEvent::Appended {
                bytes_added,
                new_file_size,
            } => {
                assert_eq!(bytes_added, 18);
                assert_eq!(new_file_size, 13 + 18);
            }
            TailEvent::Truncated { .. } => panic!("Expected Appended, got Truncated"),
        }

        follower.stop();
        assert!(!follower.is_running());
    }
}
