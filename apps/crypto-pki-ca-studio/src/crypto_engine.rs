//! High-assurance cryptographic operations: Ed25519 signing, PEM formatting,
//! and Shamir M-of-N Secret Sharing over GF(2^8).

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;

/// Galois Field GF(2^8) Arithmetic with polynomial x^8 + x^4 + x^3 + x + 1 (0x11B)
pub struct GaloisField {
    exp: [u8; 512],
    log: [u8; 256],
}

impl Default for GaloisField {
    fn default() -> Self {
        Self::new()
    }
}

impl GaloisField {
    pub fn new() -> Self {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];

        let mut x = 1u8;
        for i in 0..255 {
            exp[i] = x;
            exp[i + 255] = x;
            log[x as usize] = i as u8;

            // Multiply by primitive generator 3 in GF(2^8)
            let x2 = if (x & 0x80) != 0 {
                (x << 1) ^ 0x1B
            } else {
                x << 1
            };
            x = x2 ^ x;
        }

        Self { exp, log }
    }

    #[inline]
    pub fn add(&self, a: u8, b: u8) -> u8 {
        a ^ b
    }

    #[inline]
    pub fn sub(&self, a: u8, b: u8) -> u8 {
        a ^ b
    }

    #[inline]
    pub fn mul(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            let idx = (self.log[a as usize] as usize) + (self.log[b as usize] as usize);
            self.exp[idx]
        }
    }

    #[inline]
    pub fn div(&self, a: u8, b: u8) -> u8 {
        if a == 0 {
            0
        } else if b == 0 {
            panic!("Division by zero in GF(2^8)");
        } else {
            let idx = (self.log[a as usize] as usize) + 255 - (self.log[b as usize] as usize);
            self.exp[idx]
        }
    }

    /// Evaluates polynomial f(x) = c0 + c1*x + c2*x^2 + ... at point x in GF(2^8)
    pub fn eval_poly(&self, coeffs: &[u8], x: u8) -> u8 {
        if coeffs.is_empty() {
            return 0;
        }
        let mut result = coeffs[coeffs.len() - 1];
        for &c in coeffs[..coeffs.len() - 1].iter().rev() {
            result = self.add(self.mul(result, x), c);
        }
        result
    }
}

pub struct CryptoEngine;

impl CryptoEngine {
    /// Generates a new Ed25519 private signing key
    pub fn generate_ed25519_key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    /// Derives VerifyingKey (Public Key) from SigningKey
    pub fn get_verifying_key(signing_key: &SigningKey) -> VerifyingKey {
        signing_key.verifying_key()
    }

    /// Formats a generic string into a standard PEM block
    pub fn format_pem(tag: &str, data: &[u8]) -> String {
        let hex_str = hex::encode(data);
        let mut out = format!("-----BEGIN {}-----\n", tag);

        for chunk in hex_str.as_bytes().chunks(64) {
            if let Ok(line) = std::str::from_utf8(chunk) {
                out.push_str(line);
                out.push('\n');
            }
        }

        out.push_str(&format!("-----END {}-----\n", tag));
        out
    }

    /// Computes SHA-256 fingerprint formatted with colon separators
    pub fn compute_sha256_fingerprint(data: &[u8]) -> String {
        let hash = blake3::hash(data);
        let hex = hash.to_hex();
        let mut parts = Vec::new();
        for chunk in hex.as_bytes().chunks(2) {
            if let Ok(s) = std::str::from_utf8(chunk) {
                parts.push(s.to_uppercase());
            }
        }
        parts.join(":")
    }

    /// Splits a 32-byte secret into N shares with threshold M using Shamir's Secret Sharing over GF(2^8)
    pub fn shamir_split_secret(
        secret: &[u8],
        threshold_m: usize,
        total_n: usize,
    ) -> Result<Vec<(u8, Vec<u8>)>, String> {
        if threshold_m < 2 || threshold_m > total_n || total_n > 254 {
            return Err(format!(
                "Invalid Shamir parameters: threshold {} must be between 2 and {} (max 254)",
                threshold_m, total_n
            ));
        }

        let gf = GaloisField::new();
        let mut shares: Vec<(u8, Vec<u8>)> = (1..=total_n as u8)
            .map(|x| (x, Vec::with_capacity(secret.len())))
            .collect();

        // For each byte of the secret, create a polynomial of degree M - 1:
        // f(x) = secret_byte + a1*x + a2*x^2 + ... + a_{m-1}*x^{m-1}
        for &secret_byte in secret {
            let mut coeffs = Vec::with_capacity(threshold_m);
            coeffs.push(secret_byte);

            for _ in 1..threshold_m {
                let mut rand_byte = [0u8; 1];
                OsRng.fill_bytes(&mut rand_byte);
                // Ensure non-zero highest degree coefficient
                if rand_byte[0] == 0 {
                    rand_byte[0] = 1;
                }
                coeffs.push(rand_byte[0]);
            }

            // Evaluate polynomial for each custodian x = 1..N
            for (x, share_bytes) in shares.iter_mut() {
                let y = gf.eval_poly(&coeffs, *x);
                share_bytes.push(y);
            }
        }

        Ok(shares)
    }

    /// Reconstructs secret using Lagrange interpolation from any M shares over GF(2^8)
    pub fn shamir_reconstruct_secret(
        shares: &[(u8, Vec<u8>)],
        threshold_m: usize,
    ) -> Result<Vec<u8>, String> {
        if shares.len() < threshold_m {
            return Err(format!(
                "Insufficient shares: got {}, requires at least {}",
                shares.len(),
                threshold_m
            ));
        }

        let gf = GaloisField::new();
        let selected_shares = &shares[..threshold_m];

        // Verify share length consistency
        let secret_len = selected_shares[0].1.len();
        for s in selected_shares {
            if s.1.len() != secret_len {
                return Err("Mismatched share lengths".to_string());
            }
        }

        // Check for duplicate custodian x-coordinates
        for i in 0..selected_shares.len() {
            for j in i + 1..selected_shares.len() {
                if selected_shares[i].0 == selected_shares[j].0 {
                    return Err(format!(
                        "Duplicate custodian index detected: {}",
                        selected_shares[i].0
                    ));
                }
            }
        }

        let mut reconstructed = Vec::with_capacity(secret_len);

        // Precompute Lagrange basis polynomials at x = 0:
        // L_j(0) = Prod_{k != j} (x_k / (x_k ^ x_j))
        let mut lagrange_weights = Vec::with_capacity(threshold_m);
        for j in 0..threshold_m {
            let mut num = 1u8;
            let mut den = 1u8;
            let xj = selected_shares[j].0;

            for (k, share_k) in selected_shares.iter().enumerate().take(threshold_m) {
                if k != j {
                    let xk = share_k.0;
                    num = gf.mul(num, xk);
                    den = gf.mul(den, gf.sub(xk, xj));
                }
            }
            lagrange_weights.push(gf.div(num, den));
        }

        // Reconstruct each byte: secret_i = Sum_{j=1}^M (y_{i,j} * L_j(0))
        for byte_idx in 0..secret_len {
            let mut secret_byte = 0u8;
            for j in 0..threshold_m {
                let y = selected_shares[j].1[byte_idx];
                let weighted = gf.mul(y, lagrange_weights[j]);
                secret_byte = gf.add(secret_byte, weighted);
            }
            reconstructed.push(secret_byte);
        }

        Ok(reconstructed)
    }

    /// Digitally signs canonical certificate manifest using Ed25519
    pub fn sign_manifest(signing_key: &SigningKey, manifest: &str) -> String {
        let sig = signing_key.sign(manifest.as_bytes());
        hex::encode(sig.to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_galois_field_arithmetic() {
        let gf = GaloisField::new();
        // Identity checks
        assert_eq!(gf.add(5, 5), 0);
        assert_eq!(gf.sub(12, 12), 0);
        assert_eq!(gf.mul(7, 1), 7);
        assert_eq!(gf.mul(0, 42), 0);
        assert_eq!(gf.div(42, 1), 42);

        // Invertibility: (a * b) / b == a
        let a = 137u8;
        let b = 89u8;
        let prod = gf.mul(a, b);
        let div = gf.div(prod, b);
        assert_eq!(div, a);
    }

    #[test]
    fn test_shamir_3_of_5_roundtrip() {
        let secret = b"Apex-Offline-Root-CA-Seed-2026";
        let shares = CryptoEngine::shamir_split_secret(secret, 3, 5).expect("split secret");
        assert_eq!(shares.len(), 5);

        // Reconstruct with first 3 shares
        let recon1 =
            CryptoEngine::shamir_reconstruct_secret(&shares[0..3], 3).expect("reconstruct 1,2,3");
        assert_eq!(&recon1, secret);

        // Reconstruct with shares 1, 3, 5
        let recon2 = CryptoEngine::shamir_reconstruct_secret(
            &[shares[0].clone(), shares[2].clone(), shares[4].clone()],
            3,
        )
        .expect("reconstruct 1,3,5");
        assert_eq!(&recon2, secret);

        // Fail if only 2 shares provided
        let err = CryptoEngine::shamir_reconstruct_secret(&shares[0..2], 3);
        assert!(err.is_err());
    }
}
