#![forbid(unsafe_code)]
//! DevOps Secret Vault - Domain Models, Entities, and Validations

use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::VaultError;

/// Supported secret types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
#[serde(rename_all = "snake_case")]
pub enum SecretType {
    Password,
    SshKey,
    Certificate,
    ApiKey,
    DatabaseCredential,
    AwsCredential,
    AzureCredential,
    TerraformSecret,
    Totp,
}

impl SecretType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::SshKey => "ssh_key",
            Self::Certificate => "certificate",
            Self::ApiKey => "api_key",
            Self::DatabaseCredential => "database_credential",
            Self::AwsCredential => "aws_credential",
            Self::AzureCredential => "azure_credential",
            Self::TerraformSecret => "terraform_secret",
            Self::Totp => "totp",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ssh_key" | "ssh" => Self::SshKey,
            "certificate" | "cert" | "tls" => Self::Certificate,
            "api_key" | "api" => Self::ApiKey,
            "database_credential" | "database" | "db" => Self::DatabaseCredential,
            "aws_credential" | "aws" => Self::AwsCredential,
            "azure_credential" | "azure" => Self::AzureCredential,
            "terraform_secret" | "terraform" | "tf" => Self::TerraformSecret,
            "totp" | "2fa" | "mfa" => Self::Totp,
            _ => Self::Password,
        }
    }
}

/// Non-sensitive secret metadata stored and indexed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
pub struct SecretMetadata {
    pub id: String,
    pub title: String,
    pub secret_type: SecretType,
    pub tags: Vec<String>,
    pub username: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub version: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

impl SecretMetadata {
    pub fn validate(&self) -> Result<(), VaultError> {
        if self.title.trim().is_empty() {
            return Err(VaultError::ValidationError(
                "Secret title cannot be empty".to_string(),
            ));
        }
        if self.title.len() > 256 {
            return Err(VaultError::ValidationError(
                "Secret title exceeds 256 characters".to_string(),
            ));
        }
        Ok(())
    }
}

/// Zeroizing sensitive payload variants
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SecretPayload {
    Password {
        password: String,
    },
    SshKey {
        private_key: String,
        public_key: Option<String>,
        passphrase: Option<String>,
    },
    Certificate {
        private_key: String,
        certificate_pem: String,
        chain_pem: Option<String>,
    },
    ApiKey {
        api_key: String,
        api_secret: Option<String>,
        endpoint_url: Option<String>,
    },
    DatabaseCredential {
        engine: String,
        host: String,
        port: u16,
        database: String,
        username: String,
        password: String,
        connection_string: Option<String>,
    },
    AwsCredential {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
        region: Option<String>,
        profile_name: Option<String>,
    },
    AzureCredential {
        subscription_id: String,
        tenant_id: String,
        client_id: String,
        client_secret: String,
        cloud_environment: Option<String>,
    },
    TerraformSecret {
        state_key: Option<String>,
        tf_vars: Option<String>,
        provider_token: Option<String>,
        backend_config: Option<String>,
    },
    Totp {
        secret_base32: String,
        issuer: Option<String>,
        account_name: Option<String>,
        algorithm: Option<String>,
        digits: Option<u32>,
        period_seconds: Option<u64>,
    },
}

impl SecretPayload {
    pub fn secret_type(&self) -> SecretType {
        match self {
            Self::Password { .. } => SecretType::Password,
            Self::SshKey { .. } => SecretType::SshKey,
            Self::Certificate { .. } => SecretType::Certificate,
            Self::ApiKey { .. } => SecretType::ApiKey,
            Self::DatabaseCredential { .. } => SecretType::DatabaseCredential,
            Self::AwsCredential { .. } => SecretType::AwsCredential,
            Self::AzureCredential { .. } => SecretType::AzureCredential,
            Self::TerraformSecret { .. } => SecretType::TerraformSecret,
            Self::Totp { .. } => SecretType::Totp,
        }
    }

    pub fn primary_secret_value(&self) -> &str {
        match self {
            Self::Password { password } => password.as_str(),
            Self::SshKey { private_key, .. } => private_key.as_str(),
            Self::Certificate { private_key, .. } => private_key.as_str(),
            Self::ApiKey { api_key, .. } => api_key.as_str(),
            Self::DatabaseCredential { password, .. } => password.as_str(),
            Self::AwsCredential {
                secret_access_key, ..
            } => secret_access_key.as_str(),
            Self::AzureCredential { client_secret, .. } => client_secret.as_str(),
            Self::TerraformSecret {
                state_key,
                tf_vars,
                provider_token,
                ..
            } => {
                if let Some(t) = provider_token {
                    t.as_str()
                } else if let Some(s) = state_key {
                    s.as_str()
                } else if let Some(v) = tf_vars {
                    v.as_str()
                } else {
                    ""
                }
            }
            Self::Totp { secret_base32, .. } => secret_base32.as_str(),
        }
    }

    pub fn validate(&self) -> Result<(), VaultError> {
        match self {
            Self::Password { password } => {
                if password.is_empty() {
                    return Err(VaultError::ValidationError(
                        "Password cannot be empty".to_string(),
                    ));
                }
            }
            Self::SshKey { private_key, .. } => {
                if private_key.trim().is_empty() {
                    return Err(VaultError::ValidationError(
                        "SSH private key cannot be empty".to_string(),
                    ));
                }
            }
            Self::Certificate {
                private_key,
                certificate_pem,
                ..
            } => {
                if private_key.trim().is_empty() || certificate_pem.trim().is_empty() {
                    return Err(VaultError::ValidationError(
                        "Certificate must include both private key and certificate PEM".to_string(),
                    ));
                }
            }
            Self::ApiKey { api_key, .. } => {
                if api_key.trim().is_empty() {
                    return Err(VaultError::ValidationError(
                        "API key cannot be empty".to_string(),
                    ));
                }
            }
            Self::DatabaseCredential {
                engine,
                host,
                port,
                database,
                username,
                ..
            } => {
                if engine.trim().is_empty()
                    || host.trim().is_empty()
                    || database.trim().is_empty()
                    || username.trim().is_empty()
                {
                    return Err(VaultError::ValidationError(
                        "Database credential requires engine, host, database, and username"
                            .to_string(),
                    ));
                }
                if *port == 0 {
                    return Err(VaultError::ValidationError(
                        "Database port must be valid (>0)".to_string(),
                    ));
                }
            }
            Self::AwsCredential {
                access_key_id,
                secret_access_key,
                ..
            } => {
                if access_key_id.trim().is_empty() || secret_access_key.trim().is_empty() {
                    return Err(VaultError::ValidationError(
                        "AWS credential requires both Access Key ID and Secret Access Key"
                            .to_string(),
                    ));
                }
            }
            Self::AzureCredential {
                subscription_id,
                tenant_id,
                client_id,
                client_secret,
                ..
            } => {
                if subscription_id.trim().is_empty()
                    || tenant_id.trim().is_empty()
                    || client_id.trim().is_empty()
                    || client_secret.trim().is_empty()
                {
                    return Err(VaultError::ValidationError(
                        "Azure credential requires subscription ID, tenant ID, client ID, and client secret".to_string(),
                    ));
                }
            }
            Self::TerraformSecret {
                state_key,
                tf_vars,
                provider_token,
                backend_config,
            } => {
                if state_key.is_none()
                    && tf_vars.is_none()
                    && provider_token.is_none()
                    && backend_config.is_none()
                {
                    return Err(VaultError::ValidationError(
                        "Terraform secret must contain at least one state key, variable set, token, or backend config".to_string(),
                    ));
                }
            }
            Self::Totp { secret_base32, .. } => {
                if secret_base32.trim().is_empty() {
                    return Err(VaultError::ValidationError(
                        "TOTP secret base32 key cannot be empty".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Fully decrypted secret combining metadata and sensitive payload
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct DecryptedSecret {
    #[zeroize(skip)]
    pub metadata: SecretMetadata,
    pub payload: SecretPayload,
}

/// Encrypted secret record stored in SQLite
#[derive(Debug, Clone)]
pub struct EncryptedSecretRecord {
    pub metadata: SecretMetadata,
    pub encrypted_payload: Vec<u8>,
}

/// Creation payload passed from UI / API
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize)]
pub struct CreateSecretInput {
    pub title: String,
    pub tags: Vec<String>,
    pub username: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub payload: SecretPayload,
}

/// Update payload
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize)]
pub struct UpdateSecretInput {
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
    pub username: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub payload: Option<SecretPayload>,
}

/// Search hit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretSearchResult {
    pub id: String,
    pub title: String,
    pub secret_type: SecretType,
    pub tags: Vec<String>,
    pub username: Option<String>,
    pub score: f32,
    pub snippet: Option<String>,
}

/// Audit action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretAuditAction {
    Created,
    Read,
    Updated,
    Deleted,
    Exported,
    CopiedToClipboard,
}

impl SecretAuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Read => "read",
            Self::Updated => "updated",
            Self::Deleted => "deleted",
            Self::Exported => "exported",
            Self::CopiedToClipboard => "copied_to_clipboard",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "read" => Self::Read,
            "updated" => Self::Updated,
            "deleted" => Self::Deleted,
            "exported" => Self::Exported,
            "copied_to_clipboard" => Self::CopiedToClipboard,
            _ => Self::Created,
        }
    }
}

/// Sensitive metadata fields that are encrypted at rest with AES-256-GCM
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
pub struct EncryptedMetadataFields {
    pub tags: Vec<String>,
    pub username: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
}

/// Audit entry with tamper-evident cryptographic hash chain
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretAuditEntry {
    pub id: String,
    pub secret_id: String,
    pub action: SecretAuditAction,
    pub timestamp: i64,
    pub client: String,
    #[serde(default)]
    pub prev_hash: String,
    #[serde(default)]
    pub entry_hash: String,
}

impl SecretAuditEntry {
    pub fn new(
        id: String,
        secret_id: String,
        action: SecretAuditAction,
        timestamp: i64,
        client: String,
    ) -> Self {
        Self {
            id,
            secret_id,
            action,
            timestamp,
            client,
            prev_hash: String::new(),
            entry_hash: String::new(),
        }
    }

    /// Compute deterministic SHA-256 hash chaining this entry with its predecessor
    pub fn compute_hash(
        prev_hash: &str,
        id: &str,
        secret_id: &str,
        action: &str,
        timestamp: i64,
        client: &str,
    ) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"DEV_VAULT_AUDIT_V1:");
        hasher.update(prev_hash.as_bytes());
        hasher.update(id.as_bytes());
        hasher.update(secret_id.as_bytes());
        hasher.update(action.as_bytes());
        hasher.update(timestamp.to_be_bytes());
        hasher.update(client.as_bytes());
        format!("{:064x}", hasher.finalize())
    }
}

/// Password entropy classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntropyRating {
    VeryWeak,
    Weak,
    Fair,
    Strong,
    VeryStrong,
}

/// Result of entropy and password strength analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordStrength {
    pub entropy_bits: f64,
    pub rating: EntropyRating,
    pub length: usize,
    pub has_lowercase: bool,
    pub has_uppercase: bool,
    pub has_digits: bool,
    pub has_symbols: bool,
}

/// Calculate password entropy and strength characteristics
pub fn calculate_entropy(password: &str) -> PasswordStrength {
    let length = password.chars().count();
    if length == 0 {
        return PasswordStrength {
            entropy_bits: 0.0,
            rating: EntropyRating::VeryWeak,
            length: 0,
            has_lowercase: false,
            has_uppercase: false,
            has_digits: false,
            has_symbols: false,
        };
    }

    let mut has_lowercase = false;
    let mut has_uppercase = false;
    let mut has_digits = false;
    let mut has_symbols = false;

    for c in password.chars() {
        if c.is_ascii_lowercase() {
            has_lowercase = true;
        } else if c.is_ascii_uppercase() {
            has_uppercase = true;
        } else if c.is_ascii_digit() {
            has_digits = true;
        } else {
            has_symbols = true;
        }
    }

    let mut pool_size = 0usize;
    if has_lowercase {
        pool_size += 26;
    }
    if has_uppercase {
        pool_size += 26;
    }
    if has_digits {
        pool_size += 10;
    }
    if has_symbols {
        pool_size += 33;
    }

    let entropy_bits = if pool_size > 0 {
        (length as f64) * (pool_size as f64).log2()
    } else {
        0.0
    };

    let rating = if entropy_bits < 28.0 || length < 6 {
        EntropyRating::VeryWeak
    } else if entropy_bits < 45.0 || length < 8 {
        EntropyRating::Weak
    } else if entropy_bits < 60.0 || length < 12 {
        EntropyRating::Fair
    } else if entropy_bits < 80.0 || length < 16 {
        EntropyRating::Strong
    } else {
        EntropyRating::VeryStrong
    };

    PasswordStrength {
        entropy_bits,
        rating,
        length,
        has_lowercase,
        has_uppercase,
        has_digits,
        has_symbols,
    }
}

/// Generate a cryptographically secure random UUID-v4 style identifier
pub fn generate_uuid() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);

    // Set version 4 (bits 4-7 of time_hi_and_version to 0100)
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    // Set variant (bits 6-7 of clock_seq_hi_and_reserved to 10)
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_generation_format() {
        let id = generate_uuid();
        assert_eq!(id.len(), 36);
        assert_eq!(id.chars().nth(8), Some('-'));
        assert_eq!(id.chars().nth(13), Some('-'));
        assert_eq!(id.chars().nth(18), Some('-'));
        assert_eq!(id.chars().nth(23), Some('-'));
        assert_eq!(id.chars().nth(14), Some('4')); // version 4
    }

    #[test]
    fn test_entropy_calculation() {
        let weak = calculate_entropy("123456");
        assert!(matches!(
            weak.rating,
            EntropyRating::VeryWeak | EntropyRating::Weak
        ));

        let strong = calculate_entropy("C0rrect-H0rse-Battery-Staple!#2026");
        assert!(matches!(
            strong.rating,
            EntropyRating::Strong | EntropyRating::VeryStrong
        ));
        assert!(strong.has_uppercase);
        assert!(strong.has_lowercase);
        assert!(strong.has_digits);
        assert!(strong.has_symbols);
    }

    #[test]
    fn test_payload_validation() {
        let valid_pw = SecretPayload::Password {
            password: "SecurePassword123!".to_string(),
        };
        assert!(valid_pw.validate().is_ok());

        let empty_pw = SecretPayload::Password {
            password: "".to_string(),
        };
        assert!(empty_pw.validate().is_err());
    }
}
