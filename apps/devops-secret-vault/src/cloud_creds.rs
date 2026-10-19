#![forbid(unsafe_code)]
//! DevOps Secret Vault - Specialized Cloud Credentials and Configuration Parsers
//!
//! Provides schema definitions, domain validations, and format parsers for:
//! - AWS credentials (INI files, environment variable blocks, session tokens)
//! - Azure service principal credentials (JSON objects, environment blocks)
//! - Terraform state keys, variable files (.tfvars), and provider tokens

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::VaultError;

/// AWS Credential representation with memory zeroization on drop
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct AwsCredential {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
    pub region: Option<String>,
    pub profile_name: Option<String>,
}

impl AwsCredential {
    pub fn new(
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
        region: Option<String>,
        profile_name: Option<String>,
    ) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            session_token,
            region,
            profile_name,
        }
    }

    /// Validate AWS credential format and minimum key length
    pub fn validate(&self) -> Result<(), VaultError> {
        let key_id = self.access_key_id.trim();
        if key_id.is_empty() {
            return Err(VaultError::ValidationError(
                "AWS Access Key ID cannot be empty".to_string(),
            ));
        }

        // Standard AWS key IDs start with AKIA, ASIA, etc., and are 20 characters
        if key_id.len() < 16 || key_id.len() > 128 {
            return Err(VaultError::ValidationError(format!(
                "AWS Access Key ID length is invalid ({})",
                key_id.len()
            )));
        }

        if self.secret_access_key.trim().is_empty() {
            return Err(VaultError::ValidationError(
                "AWS Secret Access Key cannot be empty".to_string(),
            ));
        }

        if self.secret_access_key.len() < 20 {
            return Err(VaultError::ValidationError(
                "AWS Secret Access Key is suspiciously short (< 20 characters)".to_string(),
            ));
        }

        Ok(())
    }

    /// Parse AWS credentials from standard INI ~/.aws/credentials or ~/.aws/config syntax
    pub fn parse_ini(content: &str, target_profile: Option<&str>) -> Result<Vec<Self>, VaultError> {
        let mut profiles: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut current_profile = "default".to_string();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let section_name = &trimmed[1..trimmed.len() - 1].trim();
                let profile = if let Some(stripped) = section_name.strip_prefix("profile ") {
                    stripped.trim().to_string()
                } else {
                    section_name.to_string()
                };
                current_profile = profile;
                profiles.entry(current_profile.clone()).or_default();
            } else if let Some((key, val)) = trimmed.split_once('=') {
                let key_norm = key.trim().to_lowercase();
                let val_clean = val.trim().trim_matches('"').trim_matches('\'').to_string();
                profiles
                    .entry(current_profile.clone())
                    .or_default()
                    .insert(key_norm, val_clean);
            }
        }

        let mut results = Vec::new();
        for (prof_name, map) in profiles {
            if let Some(target) = target_profile {
                if prof_name != target {
                    continue;
                }
            }

            let access_key = map
                .get("aws_access_key_id")
                .or_else(|| map.get("access_key_id"))
                .cloned();

            let secret_key = map
                .get("aws_secret_access_key")
                .or_else(|| map.get("secret_access_key"))
                .cloned();

            if let (Some(ak), Some(sk)) = (access_key, secret_key) {
                let token = map
                    .get("aws_session_token")
                    .or_else(|| map.get("session_token"))
                    .cloned();
                let region = map.get("region").cloned();

                let cred = Self {
                    access_key_id: ak,
                    secret_access_key: sk,
                    session_token: token,
                    region,
                    profile_name: Some(prof_name),
                };

                if cred.validate().is_ok() {
                    results.push(cred);
                }
            }
        }

        if results.is_empty() {
            return Err(VaultError::ValidationError(
                "No valid AWS credentials found in input content".to_string(),
            ));
        }

        Ok(results)
    }
}

/// Azure Service Principal and Cloud Credential representation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct AzureCredential {
    pub subscription_id: String,
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub cloud_environment: Option<String>,
}

impl AzureCredential {
    pub fn new(
        subscription_id: String,
        tenant_id: String,
        client_id: String,
        client_secret: String,
        cloud_environment: Option<String>,
    ) -> Self {
        Self {
            subscription_id,
            tenant_id,
            client_id,
            client_secret,
            cloud_environment: cloud_environment.or_else(|| Some("AzureCloud".to_string())),
        }
    }

    /// Check UUID/GUID format (8-4-4-4-12 hex characters)
    fn is_valid_guid(guid: &str) -> bool {
        let parts: Vec<&str> = guid.split('-').collect();
        if parts.len() != 5 {
            return false;
        }
        let lengths = [8, 4, 4, 4, 12];
        for (part, expected_len) in parts.iter().zip(lengths.iter()) {
            if part.len() != *expected_len || !part.chars().all(|c| c.is_ascii_hexdigit()) {
                return false;
            }
        }
        true
    }

    pub fn validate(&self) -> Result<(), VaultError> {
        if !Self::is_valid_guid(self.subscription_id.trim()) {
            return Err(VaultError::ValidationError(
                "Azure Subscription ID must be a valid UUID".to_string(),
            ));
        }
        if !Self::is_valid_guid(self.tenant_id.trim()) {
            return Err(VaultError::ValidationError(
                "Azure Tenant ID must be a valid UUID".to_string(),
            ));
        }
        if !Self::is_valid_guid(self.client_id.trim()) {
            return Err(VaultError::ValidationError(
                "Azure Client/App ID must be a valid UUID".to_string(),
            ));
        }
        if self.client_secret.trim().is_empty() {
            return Err(VaultError::ValidationError(
                "Azure Client Secret cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Parse Azure credentials from Azure CLI `az ad sp create-for-rbac` JSON output
    pub fn parse_az_cli_json(json_str: &str, subscription_id: &str) -> Result<Self, VaultError> {
        let parsed: serde_json::Value = serde_json::from_str(json_str)?;

        let client_id = parsed
            .get("appId")
            .or_else(|| parsed.get("clientId"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                VaultError::ValidationError("Missing 'appId' in Azure JSON".to_string())
            })?;

        let client_secret = parsed
            .get("password")
            .or_else(|| parsed.get("clientSecret"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                VaultError::ValidationError("Missing 'password' in Azure JSON".to_string())
            })?;

        let tenant_id = parsed
            .get("tenant")
            .or_else(|| parsed.get("tenantId"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                VaultError::ValidationError("Missing 'tenant' in Azure JSON".to_string())
            })?;

        let cred = Self::new(
            subscription_id.to_string(),
            tenant_id.to_string(),
            client_id.to_string(),
            client_secret.to_string(),
            None,
        );
        cred.validate()?;
        Ok(cred)
    }
}

/// Terraform Secret representation (State Key, Variables, Provider Tokens)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct TerraformSecret {
    pub state_key: Option<String>,
    pub tf_vars: Option<String>,
    pub provider_token: Option<String>,
    pub backend_config: Option<String>,
}

impl TerraformSecret {
    pub fn new(
        state_key: Option<String>,
        tf_vars: Option<String>,
        provider_token: Option<String>,
        backend_config: Option<String>,
    ) -> Self {
        Self {
            state_key,
            tf_vars,
            provider_token,
            backend_config,
        }
    }

    pub fn validate(&self) -> Result<(), VaultError> {
        if self.state_key.is_none()
            && self.tf_vars.is_none()
            && self.provider_token.is_none()
            && self.backend_config.is_none()
        {
            return Err(VaultError::ValidationError(
                "Terraform secret must contain at least one state key, variable set, token, or config"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Parse .tfvars string into structured variable key-value pairs
    pub fn parse_tfvars(tfvars_content: &str) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        for line in tfvars_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim().to_string();
                let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
                vars.insert(key, val);
            }
        }
        vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_credential_validation() {
        let valid = AwsCredential::new(
            "AKIAIOSFODNN7EXAMPLE".to_string(),
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            None,
            Some("us-east-1".to_string()),
            Some("default".to_string()),
        );
        assert!(valid.validate().is_ok());

        let invalid_key = AwsCredential::new(
            "SHORT".to_string(),
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            None,
            None,
            None,
        );
        assert!(invalid_key.validate().is_err());
    }

    #[test]
    fn test_aws_ini_parser() {
        let ini = r#"
        # Sample AWS config file
        [default]
        aws_access_key_id = AKIAIOSFODNN7EXAMPLE
        aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
        region = us-west-2

        [profile staging]
        aws_access_key_id = AKIAI44QH8DHBEXAMPLE
        aws_secret_access_key = je7MtGbClwBF/2Zp9Utk/h3yCo8nvbEXAMPLEKEY
        aws_session_token = AQoDYXdzEJr111111111111111111111
        region = eu-central-1
        "#;

        let creds = AwsCredential::parse_ini(ini, None).expect("Should parse multiple profiles");
        assert_eq!(creds.len(), 2);

        let staging = AwsCredential::parse_ini(ini, Some("staging")).expect("Should parse staging");
        assert_eq!(staging.len(), 1);
        assert_eq!(staging[0].access_key_id, "AKIAI44QH8DHBEXAMPLE");
        assert!(staging[0].session_token.is_some());
    }

    #[test]
    fn test_azure_credential_validation_and_json_parsing() {
        let json = r#"{
            "appId": "11111111-2222-3333-4444-555555555555",
            "displayName": "sp-devops-vault",
            "password": "azure-client-secret-strong-token-123",
            "tenant": "66666666-7777-8888-9999-000000000000"
        }"#;

        let sub_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        let cred =
            AzureCredential::parse_az_cli_json(json, sub_id).expect("Should parse valid JSON");
        assert_eq!(cred.subscription_id, sub_id);
        assert_eq!(cred.client_id, "11111111-2222-3333-4444-555555555555");
        assert_eq!(cred.tenant_id, "66666666-7777-8888-9999-000000000000");
        assert_eq!(cred.client_secret, "azure-client-secret-strong-token-123");
        assert!(cred.validate().is_ok());

        let invalid_guid = AzureCredential::new(
            "not-a-valid-uuid".to_string(),
            "66666666-7777-8888-9999-000000000000".to_string(),
            "11111111-2222-3333-4444-555555555555".to_string(),
            "secret".to_string(),
            None,
        );
        assert!(invalid_guid.validate().is_err());
    }

    #[test]
    fn test_terraform_secret_tfvars_parsing() {
        let tfvars = r#"
        # Terraform config
        db_username = "admin_user"
        db_password = "SuperSecretDatabasePassword!"
        cluster_nodes = "3"
        // Comment line
        api_endpoint = "https://internal.vault.local"
        "#;

        let vars = TerraformSecret::parse_tfvars(tfvars);
        assert_eq!(vars.get("db_username").unwrap(), "admin_user");
        assert_eq!(
            vars.get("db_password").unwrap(),
            "SuperSecretDatabasePassword!"
        );
        assert_eq!(vars.get("cluster_nodes").unwrap(), "3");
        assert_eq!(
            vars.get("api_endpoint").unwrap(),
            "https://internal.vault.local"
        );
    }
}
