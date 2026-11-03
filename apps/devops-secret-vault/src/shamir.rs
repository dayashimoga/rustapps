#![forbid(unsafe_code)]
//! DevOps Secret Vault - Business-Tier Shamir's Secret Sharing over GF(256)
//!
//! Provides threshold-based cryptographic secret splitting and reconstruction
//! over Galois Field GF(2^8) with irreducible polynomial P(x) = x^8 + x^4 + x^3 + x + 1.
//! Any `k` shares from a total of `n` shares reconstruct the exact 256-bit master key;
//! any `k - 1` shares reveal zero mathematical information.

use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared_licensing::{LicenseTier, TierGate};

use crate::crypto::VaultMasterKey;
use crate::error::VaultError;

/// Multiplies two elements in GF(2^8) modulo P(x) = x^8 + x^4 + x^3 + x + 1 (0x11b)
const fn gf256_mul(mut a: u8, mut b: u8) -> u8 {
    let mut p = 0u8;
    let mut i = 0;
    while i < 8 {
        if (b & 1) != 0 {
            p ^= a;
        }
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 {
            a ^= 0x1b;
        }
        b >>= 1;
        i += 1;
    }
    p
}

/// Computes the multiplicative inverse in GF(2^8) using Fermat's Little Theorem (b^254)
const fn gf256_inv(b: u8) -> u8 {
    if b == 0 {
        return 0;
    }
    let mut result = 1u8;
    let mut base = b;
    let mut exp = 254u8;
    while exp > 0 {
        if (exp & 1) != 0 {
            result = gf256_mul(result, base);
        }
        base = gf256_mul(base, base);
        exp >>= 1;
    }
    result
}

/// Divides `a` by `b` in GF(2^8)
fn gf256_div(a: u8, b: u8) -> Result<u8, VaultError> {
    if b == 0 {
        return Err(VaultError::Shamir(
            "Division by zero in GF(256)".to_string(),
        ));
    }
    Ok(gf256_mul(a, gf256_inv(b)))
}

/// Evaluates a polynomial with given coefficients at `x` in GF(2^8)
fn eval_polynomial(coefficients: &[u8], x: u8) -> u8 {
    let mut result = 0u8;
    let mut x_pow = 1u8;
    for &coeff in coefficients {
        result ^= gf256_mul(coeff, x_pow);
        x_pow = gf256_mul(x_pow, x);
    }
    result
}

/// A single share in a (k, n) threshold secret sharing scheme
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShamirShare {
    /// 1-based distinct x-coordinate (1..=255)
    pub index: u8,
    /// Threshold `k` required for reconstruction
    pub threshold: u8,
    /// Total shares `n` generated
    pub total_shares: u8,
    /// Evaluated y-coordinates for each secret byte
    pub data: Vec<u8>,
}

impl ShamirShare {
    /// Compute a 4-byte SHA-256 verification checksum
    pub fn checksum(&self) -> [u8; 4] {
        let mut hasher = Sha256::new();
        hasher.update([self.threshold, self.total_shares, self.index]);
        hasher.update(&self.data);
        let hash = hasher.finalize();
        let mut cs = [0u8; 4];
        cs.copy_from_slice(&hash[0..4]);
        cs
    }

    /// Encode share to a human-readable string representation
    pub fn to_formatted_string(&self) -> String {
        let hex_data: String = self.data.iter().map(|b| format!("{:02x}", b)).collect();
        let hex_cs: String = self
            .checksum()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        format!(
            "vault-share-k{}n{}-{:02x}-{}-{}",
            self.threshold, self.total_shares, self.index, hex_data, hex_cs
        )
    }

    /// Parse share from human-readable string representation with checksum verification
    pub fn from_formatted_string(s: &str) -> Result<Self, VaultError> {
        let parts: Vec<&str> = s.trim().split('-').collect();
        if parts.len() != 6 || parts[0] != "vault" || parts[1] != "share" {
            return Err(VaultError::Shamir(
                "Invalid share format. Expected 'vault-share-k{k}n{n}-{index}-{data}-{checksum}'"
                    .to_string(),
            ));
        }

        let kn = parts[2];
        if !kn.starts_with('k') || !kn.contains('n') {
            return Err(VaultError::Shamir(
                "Invalid k/n parameter format".to_string(),
            ));
        }

        let n_pos = kn.find('n').unwrap_or(0);
        let threshold: u8 = kn[1..n_pos]
            .parse()
            .map_err(|_| VaultError::Shamir("Invalid threshold number".to_string()))?;
        let total_shares: u8 = kn[n_pos + 1..]
            .parse()
            .map_err(|_| VaultError::Shamir("Invalid total shares number".to_string()))?;

        let index = u8::from_str_radix(parts[3], 16)
            .map_err(|_| VaultError::Shamir("Invalid share index hex".to_string()))?;

        let data_hex = parts[4];
        if data_hex.len() % 2 != 0 {
            return Err(VaultError::Shamir(
                "Odd hex length in share data".to_string(),
            ));
        }

        let mut data = Vec::with_capacity(data_hex.len() / 2);
        for i in (0..data_hex.len()).step_by(2) {
            let b = u8::from_str_radix(&data_hex[i..i + 2], 16)
                .map_err(|_| VaultError::Shamir("Invalid byte in share data hex".to_string()))?;
            data.push(b);
        }

        let share = Self {
            index,
            threshold,
            total_shares,
            data,
        };

        let expected_cs = parts[5];
        let actual_cs: String = share
            .checksum()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        if expected_cs.to_lowercase() != actual_cs {
            return Err(VaultError::Shamir(
                "Share checksum validation failed: share is corrupted or tampered".to_string(),
            ));
        }

        Ok(share)
    }
}

/// Split an arbitrary secret slice into `total_shares` shares with threshold `threshold` (Business Tier)
pub fn split_secret(
    secret: &[u8],
    threshold: u8,
    total_shares: u8,
    gate: &TierGate,
) -> Result<Vec<ShamirShare>, VaultError> {
    gate.require_tier(LicenseTier::Business)?;

    if secret.is_empty() {
        return Err(VaultError::Shamir("Secret cannot be empty".to_string()));
    }
    if threshold < 2 {
        return Err(VaultError::Shamir(
            "Threshold k must be at least 2".to_string(),
        ));
    }
    if threshold > total_shares {
        return Err(VaultError::Shamir(
            "Threshold k cannot exceed total shares n".to_string(),
        ));
    }

    let secret_len = secret.len();
    let mut share_data: Vec<Vec<u8>> = vec![Vec::with_capacity(secret_len); total_shares as usize];

    for &secret_byte in secret {
        let mut poly = Vec::with_capacity(threshold as usize);
        poly.push(secret_byte); // a_0 = secret

        // Generate random coefficients a_1..=a_{k-1}
        let mut random_bytes = vec![0u8; (threshold - 1) as usize];
        OsRng.fill_bytes(&mut random_bytes);
        poly.extend_from_slice(&random_bytes);

        // Evaluate at x = 1..=total_shares
        for x in 1..=total_shares {
            let y = eval_polynomial(&poly, x);
            share_data[(x - 1) as usize].push(y);
        }
    }

    let mut shares = Vec::with_capacity(total_shares as usize);
    for (idx, data) in share_data.into_iter().enumerate() {
        shares.push(ShamirShare {
            index: (idx + 1) as u8,
            threshold,
            total_shares,
            data,
        });
    }

    Ok(shares)
}

/// Split a 256-bit VaultMasterKey into threshold shares (Business Tier)
pub fn split_master_key(
    master_key: &VaultMasterKey,
    threshold: u8,
    total_shares: u8,
    gate: &TierGate,
) -> Result<Vec<ShamirShare>, VaultError> {
    split_secret(&master_key.0, threshold, total_shares, gate)
}

/// Reconstruct the secret using any `k` valid shares
pub fn reconstruct_secret(shares: &[ShamirShare]) -> Result<Vec<u8>, VaultError> {
    if shares.is_empty() {
        return Err(VaultError::Shamir("No shares provided".to_string()));
    }

    let threshold = shares[0].threshold as usize;
    if shares.len() < threshold {
        return Err(VaultError::Shamir(format!(
            "Insufficient shares for reconstruction: provided {}, required {}",
            shares.len(),
            threshold
        )));
    }

    let secret_len = shares[0].data.len();

    // Verify all shares agree on parameters and data length
    let mut seen_indices = std::collections::HashSet::new();
    for share in shares.iter().take(threshold) {
        if share.threshold as usize != threshold {
            return Err(VaultError::Shamir(
                "Mismatched threshold parameters among shares".to_string(),
            ));
        }
        if share.data.len() != secret_len {
            return Err(VaultError::Shamir(
                "Mismatched share data lengths".to_string(),
            ));
        }
        if share.index == 0 {
            return Err(VaultError::Shamir(
                "Share index must be non-zero".to_string(),
            ));
        }
        if !seen_indices.insert(share.index) {
            return Err(VaultError::Shamir(format!(
                "Duplicate share index {} provided",
                share.index
            )));
        }
    }

    let active_shares = &shares[..threshold];
    let mut reconstructed = Vec::with_capacity(secret_len);

    for byte_idx in 0..secret_len {
        let mut secret_byte = 0u8;

        for (i, share_i) in active_shares.iter().enumerate() {
            let xi = share_i.index;
            let yi = share_i.data[byte_idx];

            // Compute Lagrange basis polynomial evaluated at x = 0:
            // ell_i(0) = \prod_{j != i} (0 - x_j) / (x_i - x_j) = \prod (x_j) / (x_i ^ x_j)
            let mut basis = 1u8;
            for (j, share_j) in active_shares.iter().enumerate() {
                if i != j {
                    let xj = share_j.index;
                    let denom = xi ^ xj;
                    let fraction = gf256_div(xj, denom)?;
                    basis = gf256_mul(basis, fraction);
                }
            }

            secret_byte ^= gf256_mul(yi, basis);
        }

        reconstructed.push(secret_byte);
    }

    Ok(reconstructed)
}

/// Reconstruct 256-bit VaultMasterKey using valid threshold shares
pub fn reconstruct_master_key(shares: &[ShamirShare]) -> Result<VaultMasterKey, VaultError> {
    let bytes = reconstruct_secret(shares)?;
    if bytes.len() != 32 {
        return Err(VaultError::Shamir(format!(
            "Reconstructed key length must be 32 bytes, got {}",
            bytes.len()
        )));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(VaultMasterKey(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gf256_multiplication_and_inversion() {
        for a in 1..=255u8 {
            let inv = gf256_inv(a);
            assert_ne!(inv, 0, "Inverse of non-zero element cannot be zero");
            assert_eq!(
                gf256_mul(a, inv),
                1,
                "a * a^(-1) must equal 1 for all a in GF(2^8)"
            );
        }
    }

    #[test]
    fn test_shamir_split_and_reconstruct_2_of_3() {
        let gate = TierGate::new(None); // Default is Free
        let secret = b"ProductionMasterSecretKey1234567";

        // Free tier must fail
        assert!(split_secret(secret, 2, 3, &gate).is_err());

        // Construct mock Business gate for test
        let authority = shared_licensing::LicenseAuthority::generate();
        let cert = authority.sign_license(shared_licensing::LicenseParameters {
            license_id: "BIZ-TEST".to_string(),
            licensee_name: "DevOps Team".to_string(),
            licensee_email: "devops@company.internal".to_string(),
            tier: LicenseTier::Business,
            app_id: shared_licensing::AppId::DevopsSecretVault,
            issued_timestamp_secs: 1000,
            expiry_timestamp_secs: None,
            max_seats: 10,
            features: vec!["all".to_string()],
        });
        let verifier =
            shared_licensing::LicenseVerifier::new(&authority.public_key_bytes()).unwrap();
        let verified = verifier
            .verify(&cert, shared_licensing::AppId::DevopsSecretVault, 2000)
            .unwrap();
        let biz_gate = TierGate::new(Some(verified));

        let shares = split_secret(secret, 2, 3, &biz_gate).expect("Split must succeed");
        assert_eq!(shares.len(), 3);

        // Any 2 shares must successfully reconstruct the secret
        let recomb_1_2 = reconstruct_secret(&[shares[0].clone(), shares[1].clone()]).unwrap();
        assert_eq!(recomb_1_2.as_slice(), secret);

        let recomb_2_3 = reconstruct_secret(&[shares[1].clone(), shares[2].clone()]).unwrap();
        assert_eq!(recomb_2_3.as_slice(), secret);

        let recomb_1_3 = reconstruct_secret(&[shares[0].clone(), shares[2].clone()]).unwrap();
        assert_eq!(recomb_1_3.as_slice(), secret);

        // All 3 shares also reconstruct
        let recomb_all = reconstruct_secret(&shares).unwrap();
        assert_eq!(recomb_all.as_slice(), secret);

        // Insufficient shares (1 of 2 required) must fail
        assert!(reconstruct_secret(&[shares[0].clone()]).is_err());
    }

    #[test]
    fn test_shamir_split_and_reconstruct_3_of_5() {
        let authority = shared_licensing::LicenseAuthority::generate();
        let cert = authority.sign_license(shared_licensing::LicenseParameters {
            license_id: "BIZ-TEST".to_string(),
            licensee_name: "DevOps Team".to_string(),
            licensee_email: "devops@company.internal".to_string(),
            tier: LicenseTier::Business,
            app_id: shared_licensing::AppId::DevopsSecretVault,
            issued_timestamp_secs: 1000,
            expiry_timestamp_secs: None,
            max_seats: 10,
            features: vec!["all".to_string()],
        });
        let verifier =
            shared_licensing::LicenseVerifier::new(&authority.public_key_bytes()).unwrap();
        let verified = verifier
            .verify(&cert, shared_licensing::AppId::DevopsSecretVault, 2000)
            .unwrap();
        let biz_gate = TierGate::new(Some(verified));

        let master_key = VaultMasterKey([0x42; 32]);
        let shares = split_master_key(&master_key, 3, 5, &biz_gate).unwrap();
        assert_eq!(shares.len(), 5);

        // Combine shares 1, 3, 4
        let sub_shares = vec![shares[0].clone(), shares[2].clone(), shares[3].clone()];
        let restored_key = reconstruct_master_key(&sub_shares).unwrap();
        assert_eq!(restored_key.0, master_key.0);
    }

    #[test]
    fn test_shamir_share_string_formatting_and_tamper_detection() {
        let share = ShamirShare {
            index: 2,
            threshold: 3,
            total_shares: 5,
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };

        let formatted = share.to_formatted_string();
        assert!(formatted.starts_with("vault-share-k3n5-02-deadbeef-"));

        let parsed = ShamirShare::from_formatted_string(&formatted).unwrap();
        assert_eq!(parsed, share);

        // Tamper test: modify a data byte
        let tampered = formatted.replace("deadbeef", "cafebabe");
        let err = ShamirShare::from_formatted_string(&tampered).unwrap_err();
        assert!(matches!(err, VaultError::Shamir(_)));
    }
}
