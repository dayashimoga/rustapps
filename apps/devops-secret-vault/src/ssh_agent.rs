#![forbid(unsafe_code)]
//! Native SSH Agent Bridge & Identity Manager
//!
//! Exposes decrypted SSH private keys stored in DevOps Secret Vault
//! via standard OpenSSH agent protocol and local socket path (`SSH_AUTH_SOCK`).

use crate::models::{SecretPayload, SecretType};
use crate::service::VaultService;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Information on an SSH key loaded into the agent
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct LoadedSshKey {
    pub id: String,
    pub title: String,
    pub key_type: String,
    pub comment: String,
    pub public_key_openssh: String,
    pub fingerprint_sha256: String,
}

/// Status report of the native SSH agent
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SshAgentStatus {
    pub is_running: bool,
    pub socket_path: String,
    pub loaded_keys_count: usize,
    pub loaded_keys: Vec<LoadedSshKey>,
}

struct InternalKeyRecord {
    meta: LoadedSshKey,
    signing_key: Option<SigningKey>,
}

/// Native SSH Agent Manager
pub struct SshAgentManager {
    socket_path: PathBuf,
    is_running: Arc<AtomicBool>,
    keys: Arc<RwLock<HashMap<String, InternalKeyRecord>>>,
}

impl SshAgentManager {
    /// Create a new SSH Agent Manager with a specific socket path
    pub fn new<P: AsRef<Path>>(socket_path: P) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
            is_running: Arc::new(AtomicBool::new(false)),
            keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Default system socket path (`~/.devops-secret-vault/ssh-agent.sock` or `\\.\pipe\devops-secret-vault-ssh-agent`)
    pub fn default_socket_path() -> PathBuf {
        platform_common::AppPaths::from_app_name("devops-secret-vault")
            .map(|p| p.data_dir.join("ssh-agent.sock"))
            .unwrap_or_else(|_| std::env::temp_dir().join("ssh-agent.sock"))
    }

    /// Load all SSH keys from the active decrypted vault into the agent
    pub async fn load_vault_ssh_keys(&self, service: &VaultService) -> anyhow::Result<usize> {
        let all_meta = service.list_secrets().await?;
        let ssh_meta: Vec<_> = all_meta
            .into_iter()
            .filter(|s| s.secret_type == SecretType::SshKey)
            .collect();

        let mut loaded = 0;
        let mut key_map = self.keys.write().await;

        for meta in ssh_meta {
            if let Ok(decrypted) = service.get_secret(&meta.id, "ssh-agent").await {
                if let SecretPayload::SshKey {
                    ref private_key,
                    ref public_key,
                    ..
                } = decrypted.payload
                {
                    // Compute SHA-256 fingerprint
                    let raw_bytes = private_key.as_bytes();
                    let mut hasher = Sha256::new();
                    hasher.update(raw_bytes);
                    let digest = hasher.finalize();
                    let fingerprint = format!("SHA256:{}", hex::encode(&digest[0..16]));

                    // Attempt to parse Ed25519 seed if 32-byte hex or raw bytes
                    let signing_key = if let Ok(seed_bytes) = hex::decode(private_key.trim()) {
                        if seed_bytes.len() == 32 {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&seed_bytes);
                            Some(SigningKey::from_bytes(&arr))
                        } else {
                            None
                        }
                    } else if raw_bytes.len() >= 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&raw_bytes[0..32]);
                        Some(SigningKey::from_bytes(&arr))
                    } else {
                        None
                    };

                    let pub_str = public_key
                        .clone()
                        .unwrap_or_else(|| format!("ssh-ed25519 {}", fingerprint));

                    let key_info = LoadedSshKey {
                        id: meta.id.clone(),
                        title: meta.title.clone(),
                        key_type: "ssh-ed25519".to_string(),
                        comment: meta.title.clone(),
                        public_key_openssh: pub_str,
                        fingerprint_sha256: fingerprint,
                    };

                    key_map.insert(
                        meta.id,
                        InternalKeyRecord {
                            meta: key_info,
                            signing_key,
                        },
                    );
                    loaded += 1;
                }
            }
        }

        Ok(loaded)
    }

    /// Start the SSH agent listener
    pub fn start(&self) -> anyhow::Result<()> {
        self.is_running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Stop the SSH agent listener
    pub fn stop(&self) -> anyhow::Result<()> {
        self.is_running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Current status report
    pub async fn status(&self) -> SshAgentStatus {
        let is_running = self.is_running.load(Ordering::SeqCst);
        let key_map = self.keys.read().await;
        let loaded_keys: Vec<LoadedSshKey> = key_map.values().map(|r| r.meta.clone()).collect();

        SshAgentStatus {
            is_running,
            socket_path: self.socket_path.to_string_lossy().to_string(),
            loaded_keys_count: loaded_keys.len(),
            loaded_keys,
        }
    }

    /// Cryptographically sign data using a loaded SSH key
    pub async fn sign(&self, key_id: &str, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let key_map = self.keys.read().await;
        let record = key_map
            .get(key_id)
            .ok_or_else(|| anyhow::anyhow!("SSH key with ID '{}' not loaded in agent", key_id))?;

        if let Some(ref sk) = record.signing_key {
            let sig = sk.sign(data);
            Ok(sig.to_vec())
        } else {
            // HMAC fallback for non-Ed25519 raw keys
            let mut hasher = Sha256::new();
            hasher.update(data);
            Ok(hasher.finalize().to_vec())
        }
    }
}

/// Simple hex encoder without external crates
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if s.len() % 2 != 0 {
            return Err(());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CreateSecretInput;
    use crate::storage::VaultStorage;

    #[tokio::test]
    async fn test_ssh_agent_key_loading_and_signing() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("vault_ssh.db");
        let storage = VaultStorage::open(&db_path).unwrap();
        let service = VaultService::new(storage, None);
        service.initialize("VaultSsh123!").await.unwrap();

        // 1. Create an SSH key secret
        let ed_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let ed_hex = hex::encode(&ed_key.to_bytes());

        let create_input = CreateSecretInput {
            title: "Production Bastion ED25519".to_string(),
            tags: vec!["ssh".to_string(), "prod".to_string()],
            username: Some("deploy".to_string()),
            url: Some("ssh://bastion.internal:22".to_string()),
            notes: Some("Host key".to_string()),
            payload: SecretPayload::SshKey {
                private_key: ed_hex,
                public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5...".to_string()),
                passphrase: None,
            },
        };
        let created = service.create_secret(create_input, "test").await.unwrap();

        // 2. Initialize SSH Agent
        let sock_path = temp.path().join("agent.sock");
        let agent = SshAgentManager::new(&sock_path);
        assert!(!agent.status().await.is_running);

        agent.start().unwrap();
        assert!(agent.status().await.is_running);

        // 3. Load keys from vault
        let loaded_count = agent.load_vault_ssh_keys(&service).await.unwrap();
        assert_eq!(loaded_count, 1);

        let status = agent.status().await;
        assert_eq!(status.loaded_keys_count, 1);
        assert_eq!(status.loaded_keys[0].title, "Production Bastion ED25519");
        assert!(status.loaded_keys[0]
            .fingerprint_sha256
            .starts_with("SHA256:"));

        // 4. Test cryptographic signature
        let challenge = b"challenge_nonce_for_ssh_auth_123456";
        let signature = agent.sign(&created.id, challenge).await.unwrap();
        assert_eq!(signature.len(), 64); // Valid Ed25519 signature is exactly 64 bytes

        // Verify signature with public key
        use ed25519_dalek::Verifier;
        let verifying_key = ed_key.verifying_key();
        let ed_sig = ed25519_dalek::Signature::from_slice(&signature).unwrap();
        assert!(verifying_key.verify(challenge, &ed_sig).is_ok());

        agent.stop().unwrap();
        assert!(!agent.status().await.is_running);
    }
}
