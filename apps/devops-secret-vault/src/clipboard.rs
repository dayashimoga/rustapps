#![forbid(unsafe_code)]
//! DevOps Secret Vault - Secure Auto-Clearing Clipboard Manager

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::VaultError;

/// Thread-safe in-memory secure clipboard holding temporary secret content with auto-wipe
#[derive(Clone)]
pub struct SecureClipboard {
    current_payload: Arc<Mutex<Option<ClipboardSlot>>>,
    ttl_seconds: Arc<AtomicU32>,
    is_active: Arc<AtomicBool>,
}

struct ClipboardSlot {
    content: ZeroizingClipboardString,
    _copied_at: Instant,
    expires_at: Instant,
}

#[derive(Zeroize, ZeroizeOnDrop)]
struct ZeroizingClipboardString(String);

impl Default for SecureClipboard {
    fn default() -> Self {
        Self::new(30)
    }
}

impl SecureClipboard {
    /// Create a new secure clipboard manager with default TTL in seconds
    pub fn new(default_ttl_secs: u32) -> Self {
        Self {
            current_payload: Arc::new(Mutex::new(None)),
            ttl_seconds: Arc::new(AtomicU32::new(default_ttl_secs)),
            is_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set default TTL
    pub fn set_ttl(&self, secs: u32) {
        self.ttl_seconds.store(secs, Ordering::SeqCst);
    }

    /// Copy secret value into secure clipboard with automated background clearing
    pub async fn copy(&self, text: &str, custom_ttl_secs: Option<u32>) -> Result<u32, VaultError> {
        let ttl = custom_ttl_secs.unwrap_or_else(|| self.ttl_seconds.load(Ordering::SeqCst));
        let now = Instant::now();
        let expires_at = now + Duration::from_secs(ttl as u64);

        {
            let mut slot_guard = self.current_payload.lock().await;
            *slot_guard = Some(ClipboardSlot {
                content: ZeroizingClipboardString(text.to_string()),
                _copied_at: now,
                expires_at,
            });
        }

        self.is_active.store(true, Ordering::SeqCst);

        // Spawn async background worker to clear memory when time expires
        let current_payload_clone = Arc::clone(&self.current_payload);
        let is_active_clone = Arc::clone(&self.is_active);

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(ttl as u64)).await;

            let mut guard = current_payload_clone.lock().await;
            if let Some(slot) = guard.as_ref() {
                if Instant::now() >= slot.expires_at {
                    *guard = None;
                    is_active_clone.store(false, Ordering::SeqCst);
                }
            }
        });

        Ok(ttl)
    }

    /// Immediately clear clipboard memory
    pub async fn clear(&self) -> Result<(), VaultError> {
        let mut guard = self.current_payload.lock().await;
        *guard = None;
        self.is_active.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Get remaining seconds until clipboard auto-clears
    pub async fn remaining_seconds(&self) -> Option<u32> {
        let guard = self.current_payload.lock().await;
        if let Some(slot) = guard.as_ref() {
            let now = Instant::now();
            if slot.expires_at > now {
                let remaining = slot.expires_at.duration_since(now).as_secs();
                Some(remaining as u32)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Peek current value (used for verification and testing)
    pub async fn peek_for_test(&self) -> Option<String> {
        let guard = self.current_payload.lock().await;
        guard.as_ref().map(|s| s.content.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_copy_and_manual_clear() {
        let clipboard = SecureClipboard::new(30);
        let ttl = clipboard.copy("s3cret-t0ken-99", None).await.unwrap();
        assert_eq!(ttl, 30);

        assert_eq!(
            clipboard.peek_for_test().await,
            Some("s3cret-t0ken-99".to_string())
        );
        let remaining = clipboard.remaining_seconds().await;
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= 30);

        clipboard.clear().await.unwrap();
        assert_eq!(clipboard.peek_for_test().await, None);
        assert_eq!(clipboard.remaining_seconds().await, None);
    }

    #[tokio::test]
    async fn test_clipboard_auto_clear_timer() {
        let clipboard = SecureClipboard::new(1);
        clipboard.copy("temporary-key", Some(1)).await.unwrap();
        assert_eq!(
            clipboard.peek_for_test().await,
            Some("temporary-key".to_string())
        );

        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert_eq!(clipboard.peek_for_test().await, None);
    }
}
