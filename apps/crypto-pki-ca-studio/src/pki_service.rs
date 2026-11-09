//! Visual Air-Gapped PKI Management Service.
//!
//! Handles CA hierarchies, CSR signing, certificate revocation lists (CRL),
//! and Shamir M-of-N custodian key ceremonies.

use crate::crypto_engine::CryptoEngine;
use crate::models::{
    CertType, CertificateRevocationList, CertificateSigningRequest, ExtendedKeyUsageFlags,
    KeyAlgorithm, KeyUsageFlags, PkiStats, RevokedCertificateEntry, ShamirCeremony, ShamirShard,
    SubjectDn, X509Certificate,
};
use ed25519_dalek::SigningKey;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

struct CaRecord {
    pub cert: X509Certificate,
    pub signing_key: SigningKey,
}

pub struct PkiService {
    cas: Arc<RwLock<HashMap<String, CaRecord>>>,
    issued_certs: Arc<RwLock<HashMap<String, X509Certificate>>>,
    #[allow(dead_code)]
    csrs: Arc<RwLock<HashMap<String, CertificateSigningRequest>>>,
    crls: Arc<RwLock<HashMap<String, CertificateRevocationList>>>,
    shamir_ceremonies: Arc<RwLock<HashMap<String, ShamirCeremony>>>,
    stats: Arc<RwLock<PkiStats>>,
}

impl Default for PkiService {
    fn default() -> Self {
        Self::new()
    }
}

impl PkiService {
    pub fn new() -> Self {
        let service = Self {
            cas: Arc::new(RwLock::new(HashMap::new())),
            issued_certs: Arc::new(RwLock::new(HashMap::new())),
            csrs: Arc::new(RwLock::new(HashMap::new())),
            crls: Arc::new(RwLock::new(HashMap::new())),
            shamir_ceremonies: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PkiStats {
                total_cas: 0,
                total_issued_certificates: 0,
                active_certificates: 0,
                revoked_certificates: 0,
                active_crls: 0,
                shamir_ceremonies_count: 0,
            })),
        };

        service.seed_enterprise_pki();
        service
    }

    /// Seeds realistic air-gapped Enterprise PKI with Root CA, Issuing CA, End-Entity, Revocation, and Shamir Ceremony
    fn seed_enterprise_pki(&self) {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let ten_years_ms = 10 * 365 * 24 * 3600 * 1000i64;
        let twenty_years_ms = 20 * 365 * 24 * 3600 * 1000i64;
        let one_year_ms = 365 * 24 * 3600 * 1000i64;

        // 1. Root CA: Apex Air-Gapped Root CA G1 (Self-Signed)
        let root_key = CryptoEngine::generate_ed25519_key();
        let root_pub = CryptoEngine::get_verifying_key(&root_key);
        let root_pub_pem = CryptoEngine::format_pem("PUBLIC KEY", root_pub.as_bytes());

        let root_subject = SubjectDn {
            common_name: "Apex Air-Gapped Root CA G1".to_string(),
            organization: "Apex Enterprise Trust Network".to_string(),
            organizational_unit: "Offline Certificate Authority Operations".to_string(),
            country: "US".to_string(),
            state_or_province: "Virginia".to_string(),
            locality: "McLean".to_string(),
        };

        let root_manifest = format!(
            "X509|ROOT|SERIAL:01|SUBJ:{}|ISSUER:{}|VALID:{}-{}|ALGO:Ed25519",
            root_subject.to_rfc2253(),
            root_subject.to_rfc2253(),
            now_ms,
            now_ms + twenty_years_ms
        );
        let root_sig = CryptoEngine::sign_manifest(&root_key, &root_manifest);
        let root_cert_pem = CryptoEngine::format_pem("CERTIFICATE", root_sig.as_bytes());
        let root_fp = CryptoEngine::compute_sha256_fingerprint(root_cert_pem.as_bytes());

        let root_cert = X509Certificate {
            certificate_id: "ca-root-apex-g1".to_string(),
            serial_number_hex: "00A1F028C7".to_string(),
            cert_type: CertType::RootCa,
            subject: root_subject.clone(),
            issuer_dn: root_subject.to_rfc2253(),
            issuer_ca_id: None,
            key_algorithm: KeyAlgorithm::Ed25519,
            validity_not_before_epoch_ms: now_ms,
            validity_not_after_epoch_ms: now_ms + twenty_years_ms,
            subject_alt_names: vec!["Apex-Root-G1".to_string()],
            key_usage: KeyUsageFlags {
                digital_signature: true,
                key_encipherment: false,
                key_cert_sign: true,
                crl_sign: true,
            },
            ext_key_usage: ExtendedKeyUsageFlags::default(),
            is_ca: true,
            path_len_constraint: Some(1),
            fingerprint_sha256: root_fp,
            certificate_pem: root_cert_pem,
            public_key_pem: root_pub_pem,
            is_revoked: false,
            revocation_reason: None,
            revocation_epoch_ms: None,
        };

        // 2. Intermediate CA: Apex Enterprise Issuing CA 1A (Signed by Root CA)
        let issuing_key = CryptoEngine::generate_ed25519_key();
        let issuing_pub = CryptoEngine::get_verifying_key(&issuing_key);
        let issuing_pub_pem = CryptoEngine::format_pem("PUBLIC KEY", issuing_pub.as_bytes());

        let issuing_subject = SubjectDn {
            common_name: "Apex Enterprise Issuing CA 1A".to_string(),
            organization: "Apex Enterprise Trust Network".to_string(),
            organizational_unit: "Subordinate CA Operations".to_string(),
            country: "US".to_string(),
            state_or_province: "Virginia".to_string(),
            locality: "McLean".to_string(),
        };

        let issuing_manifest = format!(
            "X509|SUBCA|SERIAL:02|SUBJ:{}|ISSUER:{}|VALID:{}-{}|ALGO:Ed25519",
            issuing_subject.to_rfc2253(),
            root_subject.to_rfc2253(),
            now_ms,
            now_ms + ten_years_ms
        );
        let issuing_sig = CryptoEngine::sign_manifest(&root_key, &issuing_manifest);
        let issuing_cert_pem = CryptoEngine::format_pem("CERTIFICATE", issuing_sig.as_bytes());
        let issuing_fp = CryptoEngine::compute_sha256_fingerprint(issuing_cert_pem.as_bytes());

        let issuing_cert = X509Certificate {
            certificate_id: "ca-sub-apex-1a".to_string(),
            serial_number_hex: "00B48A109E".to_string(),
            cert_type: CertType::IntermediateCa,
            subject: issuing_subject.clone(),
            issuer_dn: root_subject.to_rfc2253(),
            issuer_ca_id: Some("ca-root-apex-g1".to_string()),
            key_algorithm: KeyAlgorithm::Ed25519,
            validity_not_before_epoch_ms: now_ms,
            validity_not_after_epoch_ms: now_ms + ten_years_ms,
            subject_alt_names: vec!["Apex-Issuing-1A".to_string()],
            key_usage: KeyUsageFlags {
                digital_signature: true,
                key_encipherment: false,
                key_cert_sign: true,
                crl_sign: true,
            },
            ext_key_usage: ExtendedKeyUsageFlags::default(),
            is_ca: true,
            path_len_constraint: Some(0),
            fingerprint_sha256: issuing_fp,
            certificate_pem: issuing_cert_pem,
            public_key_pem: issuing_pub_pem,
            is_revoked: false,
            revocation_reason: None,
            revocation_epoch_ms: None,
        };

        // 3. End-Entity Certificate: api.corp.internal (Signed by Issuing CA)
        let ee_key = CryptoEngine::generate_ed25519_key();
        let ee_pub = CryptoEngine::get_verifying_key(&ee_key);
        let ee_pub_pem = CryptoEngine::format_pem("PUBLIC KEY", ee_pub.as_bytes());

        let ee_subject = SubjectDn {
            common_name: "api.corp.internal".to_string(),
            organization: "Apex Enterprise Trust Network".to_string(),
            organizational_unit: "Core Cloud API Infrastructure".to_string(),
            country: "US".to_string(),
            state_or_province: "Virginia".to_string(),
            locality: "McLean".to_string(),
        };

        let ee_manifest = format!(
            "X509|EE|SERIAL:03|SUBJ:{}|ISSUER:{}|VALID:{}-{}|ALGO:Ed25519",
            ee_subject.to_rfc2253(),
            issuing_subject.to_rfc2253(),
            now_ms,
            now_ms + one_year_ms
        );
        let ee_sig = CryptoEngine::sign_manifest(&issuing_key, &ee_manifest);
        let ee_cert_pem = CryptoEngine::format_pem("CERTIFICATE", ee_sig.as_bytes());
        let ee_fp = CryptoEngine::compute_sha256_fingerprint(ee_cert_pem.as_bytes());

        let ee_cert = X509Certificate {
            certificate_id: "cert-ee-api-corp".to_string(),
            serial_number_hex: "00C912A84F".to_string(),
            cert_type: CertType::TlsServer,
            subject: ee_subject,
            issuer_dn: issuing_subject.to_rfc2253(),
            issuer_ca_id: Some("ca-sub-apex-1a".to_string()),
            key_algorithm: KeyAlgorithm::Ed25519,
            validity_not_before_epoch_ms: now_ms,
            validity_not_after_epoch_ms: now_ms + one_year_ms,
            subject_alt_names: vec![
                "DNS:api.corp.internal".to_string(),
                "DNS:auth.corp.internal".to_string(),
                "IP:10.0.0.1".to_string(),
            ],
            key_usage: KeyUsageFlags {
                digital_signature: true,
                key_encipherment: true,
                key_cert_sign: false,
                crl_sign: false,
            },
            ext_key_usage: ExtendedKeyUsageFlags {
                server_auth: true,
                client_auth: true,
                code_signing: false,
                email_protection: false,
            },
            is_ca: false,
            path_len_constraint: None,
            fingerprint_sha256: ee_fp,
            certificate_pem: ee_cert_pem,
            public_key_pem: ee_pub_pem,
            is_revoked: false,
            revocation_reason: None,
            revocation_epoch_ms: None,
        };

        // 4. Revoked Test Certificate: staging-worker.corp.internal
        let rev_cert = X509Certificate {
            certificate_id: "cert-ee-staging-compromised".to_string(),
            serial_number_hex: "00D8412F9A".to_string(),
            cert_type: CertType::TlsServer,
            subject: SubjectDn {
                common_name: "staging-worker.corp.internal".to_string(),
                organization: "Apex Enterprise Trust Network".to_string(),
                organizational_unit: "Staging Fleet".to_string(),
                country: "US".to_string(),
                state_or_province: "Virginia".to_string(),
                locality: "McLean".to_string(),
            },
            issuer_dn: issuing_subject.to_rfc2253(),
            issuer_ca_id: Some("ca-sub-apex-1a".to_string()),
            key_algorithm: KeyAlgorithm::Ed25519,
            validity_not_before_epoch_ms: now_ms - 86400000 * 30,
            validity_not_after_epoch_ms: now_ms + 86400000 * 335,
            subject_alt_names: vec!["DNS:staging-worker.corp.internal".to_string()],
            key_usage: KeyUsageFlags {
                digital_signature: true,
                key_encipherment: true,
                key_cert_sign: false,
                crl_sign: false,
            },
            ext_key_usage: ExtendedKeyUsageFlags {
                server_auth: true,
                client_auth: false,
                code_signing: false,
                email_protection: false,
            },
            is_ca: false,
            path_len_constraint: None,
            fingerprint_sha256: "88:AA:BB:CC:DD:EE:11:22:33:44:55:66:77:88:99:00:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99".to_string(),
            certificate_pem: "-----BEGIN CERTIFICATE-----\nREVOKED_CERT_DEMO\n-----END CERTIFICATE-----\n".to_string(),
            public_key_pem: "-----BEGIN PUBLIC KEY-----\nREVOKED_PUB_DEMO\n-----END PUBLIC KEY-----\n".to_string(),
            is_revoked: true,
            revocation_reason: Some("KeyCompromise".to_string()),
            revocation_epoch_ms: Some(now_ms - 86400000 * 5),
        };

        // 5. CRL for Issuing CA
        let crl_manifest = format!(
            "CRL|ISSUER:{}|REVOKED:00D8412F9A:KeyCompromise|UPDATE:{}",
            issuing_subject.to_rfc2253(),
            now_ms
        );
        let crl_sig = CryptoEngine::sign_manifest(&issuing_key, &crl_manifest);
        let crl = CertificateRevocationList {
            crl_id: "crl-issuing-1a".to_string(),
            issuer_ca_id: "ca-sub-apex-1a".to_string(),
            issuer_dn: issuing_subject.to_rfc2253(),
            this_update_epoch_ms: now_ms,
            next_update_epoch_ms: now_ms + 86400000 * 7,
            revoked_entries: vec![RevokedCertificateEntry {
                serial_number_hex: "00D8412F9A".to_string(),
                revocation_epoch_ms: now_ms - 86400000 * 5,
                reason: "KeyCompromise".to_string(),
            }],
            signature_hex: crl_sig.clone(),
            crl_pem: CryptoEngine::format_pem("X509 CRL", crl_sig.as_bytes()),
        };

        // 6. Shamir 3-of-5 Ceremony for Root CA Protection
        let root_secret_bytes = root_key.to_bytes();
        let shards =
            CryptoEngine::shamir_split_secret(&root_secret_bytes, 3, 5).unwrap_or_default();
        let custodian_labels = [
            "Custodian 1: Security Director (Hardware Token A)",
            "Custodian 2: Chief Information Security Officer (Physical Safe B)",
            "Custodian 3: Lead Cryptographic Engineer (Escrow C)",
            "Custodian 4: Infrastructure Operations Lead (HSM D)",
            "Custodian 5: Compliance Officer (Bank Safe Deposit Box E)",
        ];

        let shamir_shards: Vec<ShamirShard> = shards
            .into_iter()
            .enumerate()
            .map(|(i, (idx, share_bytes))| {
                let hex_s = hex::encode(&share_bytes);
                let commit = blake3::hash(&share_bytes).to_hex().to_string();
                ShamirShard {
                    shard_index: idx,
                    shard_hex: hex_s,
                    custodian_label: custodian_labels[i].to_string(),
                    commitment_hash: commit,
                }
            })
            .collect();

        let ceremony = ShamirCeremony {
            ceremony_id: "ceremony-root-apex-g1-shamir".to_string(),
            ca_id: "ca-root-apex-g1".to_string(),
            ca_common_name: "Apex Air-Gapped Root CA G1".to_string(),
            threshold_m: 3,
            total_n: 5,
            shards: shamir_shards,
            created_at_epoch_ms: now_ms,
            status: "Active".to_string(),
        };

        // Store into service
        if let Ok(mut lock) = self.cas.write() {
            lock.insert(
                "ca-root-apex-g1".to_string(),
                CaRecord {
                    cert: root_cert.clone(),
                    signing_key: root_key,
                },
            );
            lock.insert(
                "ca-sub-apex-1a".to_string(),
                CaRecord {
                    cert: issuing_cert.clone(),
                    signing_key: issuing_key,
                },
            );
        }

        if let Ok(mut lock) = self.issued_certs.write() {
            lock.insert("ca-root-apex-g1".to_string(), root_cert);
            lock.insert("ca-sub-apex-1a".to_string(), issuing_cert);
            lock.insert("cert-ee-api-corp".to_string(), ee_cert);
            lock.insert("cert-ee-staging-compromised".to_string(), rev_cert);
        }

        if let Ok(mut lock) = self.crls.write() {
            lock.insert("crl-issuing-1a".to_string(), crl);
        }

        if let Ok(mut lock) = self.shamir_ceremonies.write() {
            lock.insert("ceremony-root-apex-g1-shamir".to_string(), ceremony);
        }

        self.update_stats();
    }

    fn update_stats(&self) {
        let cas_count = self.cas.read().unwrap().len();
        let certs = self.issued_certs.read().unwrap();
        let total_certs = certs.len();
        let revoked_count = certs.values().filter(|c| c.is_revoked).count();
        let active_certs = total_certs.saturating_sub(revoked_count);
        let crls_count = self.crls.read().unwrap().len();
        let shamir_count = self.shamir_ceremonies.read().unwrap().len();

        if let Ok(mut stats) = self.stats.write() {
            stats.total_cas = cas_count;
            stats.total_issued_certificates = total_certs;
            stats.active_certificates = active_certs;
            stats.revoked_certificates = revoked_count;
            stats.active_crls = crls_count;
            stats.shamir_ceremonies_count = shamir_count;
        }
    }

    pub fn get_stats(&self) -> PkiStats {
        self.stats.read().unwrap().clone()
    }

    pub fn get_hierarchy(&self) -> Vec<X509Certificate> {
        let certs = self.issued_certs.read().unwrap();
        let mut list: Vec<X509Certificate> = certs.values().cloned().collect();
        list.sort_by(|a, b| a.certificate_id.cmp(&b.certificate_id));
        list
    }

    pub fn get_certificate_detail(&self, cert_id: &str) -> Option<X509Certificate> {
        let certs = self.issued_certs.read().unwrap();
        if cert_id.is_empty() {
            certs.values().next().cloned()
        } else {
            certs.get(cert_id).cloned()
        }
    }

    pub fn get_crls(&self) -> Vec<CertificateRevocationList> {
        let crls = self.crls.read().unwrap();
        crls.values().cloned().collect()
    }

    pub fn get_shamir_ceremonies(&self) -> Vec<ShamirCeremony> {
        let ceremonies = self.shamir_ceremonies.read().unwrap();
        ceremonies.values().cloned().collect()
    }

    pub fn revoke_certificate(
        &self,
        cert_id: &str,
        reason: &str,
    ) -> Result<X509Certificate, String> {
        let mut certs = self.issued_certs.write().unwrap();
        let cert = certs
            .get_mut(cert_id)
            .ok_or_else(|| format!("Certificate not found: {}", cert_id))?;

        if cert.is_revoked {
            return Err("Certificate is already revoked".to_string());
        }

        let now_ms = chrono::Utc::now().timestamp_millis();
        cert.is_revoked = true;
        cert.revocation_reason = Some(reason.to_string());
        cert.revocation_epoch_ms = Some(now_ms);

        let updated = cert.clone();
        drop(certs);

        // Update CRL for the issuer CA
        if let Some(ref ca_id) = updated.issuer_ca_id {
            if let Ok(mut crls) = self.crls.write() {
                if let Some(crl) = crls.values_mut().find(|c| c.issuer_ca_id == *ca_id) {
                    crl.revoked_entries.push(RevokedCertificateEntry {
                        serial_number_hex: updated.serial_number_hex.clone(),
                        revocation_epoch_ms: now_ms,
                        reason: reason.to_string(),
                    });
                    crl.this_update_epoch_ms = now_ms;
                }
            }
        }

        self.update_stats();
        Ok(updated)
    }

    /// Signs a CSR and issues a newly signed X.509 Certificate
    pub fn sign_csr(
        &self,
        ca_id: &str,
        subject_cn: &str,
        cert_type: CertType,
        sans: &[String],
        validity_days: u32,
    ) -> Result<X509Certificate, String> {
        let cas = self.cas.read().unwrap();
        let ca_record = cas
            .get(ca_id)
            .ok_or_else(|| format!("Issuer CA not found: {}", ca_id))?;

        let now_ms = chrono::Utc::now().timestamp_millis();
        let validity_ms = (validity_days as i64) * 86400000;

        let ee_key = CryptoEngine::generate_ed25519_key();
        let ee_pub = CryptoEngine::get_verifying_key(&ee_key);
        let ee_pub_pem = CryptoEngine::format_pem("PUBLIC KEY", ee_pub.as_bytes());

        let subject = SubjectDn {
            common_name: subject_cn.to_string(),
            organization: ca_record.cert.subject.organization.clone(),
            organizational_unit: "Signed By Crypto PKI Studio".to_string(),
            country: ca_record.cert.subject.country.clone(),
            state_or_province: ca_record.cert.subject.state_or_province.clone(),
            locality: ca_record.cert.subject.locality.clone(),
        };

        let serial = format!("00{:08X}", rand::random::<u32>());
        let cert_id = format!("cert-{}", serial.to_lowercase());

        let manifest = format!(
            "X509|{}|SERIAL:{}|SUBJ:{}|ISSUER:{}|VALID:{}-{}|ALGO:Ed25519",
            cert_id,
            serial,
            subject.to_rfc2253(),
            ca_record.cert.subject.to_rfc2253(),
            now_ms,
            now_ms + validity_ms
        );

        let sig = CryptoEngine::sign_manifest(&ca_record.signing_key, &manifest);
        let cert_pem = CryptoEngine::format_pem("CERTIFICATE", sig.as_bytes());
        let fp = CryptoEngine::compute_sha256_fingerprint(cert_pem.as_bytes());

        let cert = X509Certificate {
            certificate_id: cert_id.clone(),
            serial_number_hex: serial,
            cert_type,
            subject,
            issuer_dn: ca_record.cert.subject.to_rfc2253(),
            issuer_ca_id: Some(ca_id.to_string()),
            key_algorithm: KeyAlgorithm::Ed25519,
            validity_not_before_epoch_ms: now_ms,
            validity_not_after_epoch_ms: now_ms + validity_ms,
            subject_alt_names: sans.to_vec(),
            key_usage: KeyUsageFlags {
                digital_signature: true,
                key_encipherment: true,
                key_cert_sign: false,
                crl_sign: false,
            },
            ext_key_usage: ExtendedKeyUsageFlags {
                server_auth: cert_type == CertType::TlsServer,
                client_auth: cert_type == CertType::TlsClient,
                code_signing: cert_type == CertType::CodeSigning,
                email_protection: false,
            },
            is_ca: false,
            path_len_constraint: None,
            fingerprint_sha256: fp,
            certificate_pem: cert_pem,
            public_key_pem: ee_pub_pem,
            is_revoked: false,
            revocation_reason: None,
            revocation_epoch_ms: None,
        };

        drop(cas);
        if let Ok(mut certs) = self.issued_certs.write() {
            certs.insert(cert_id, cert.clone());
        }

        self.update_stats();
        Ok(cert)
    }

    /// Reconstructs private key from M custodian shards
    pub fn reconstruct_shamir_key(
        &self,
        shards: &[(u8, String)],
        threshold_m: usize,
    ) -> Result<String, String> {
        let mut byte_shards = Vec::new();
        for (idx, hex_str) in shards {
            let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex shard: {}", e))?;
            byte_shards.push((*idx, bytes));
        }

        let reconstructed = CryptoEngine::shamir_reconstruct_secret(&byte_shards, threshold_m)?;
        Ok(hex::encode(reconstructed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pki_hierarchy_and_csr_signing() {
        let service = PkiService::new();
        let stats = service.get_stats();
        assert_eq!(stats.total_cas, 2);
        assert_eq!(stats.active_crls, 1);
        assert_eq!(stats.shamir_ceremonies_count, 1);
        assert_eq!(stats.revoked_certificates, 1);

        // Sign new CSR
        let signed = service
            .sign_csr(
                "ca-sub-apex-1a",
                "payment-gateway.corp.internal",
                CertType::TlsServer,
                &["DNS:payment-gateway.corp.internal".to_string()],
                90,
            )
            .expect("sign csr");

        assert_eq!(signed.subject.common_name, "payment-gateway.corp.internal");
        assert_eq!(signed.issuer_ca_id, Some("ca-sub-apex-1a".to_string()));

        // Revoke certificate
        let revoked = service
            .revoke_certificate(&signed.certificate_id, "AffiliationChanged")
            .expect("revoke cert");
        assert!(revoked.is_revoked);
    }
}
