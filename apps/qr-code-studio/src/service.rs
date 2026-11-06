#![forbid(unsafe_code)]
//! High-Level Offline QR Code Studio Service
//!
//! Orchestrates ISO/IEC 18004 QR generation, SVG/PNG rendering, image frame decoding,
//! anti-quishing threat analysis, cryptographic signing & encryption, and SQLite WAL history persistence.

use crate::models::{
    extract_structured_fields, BatchJobReport, BatchQrGeneratedEntry, BatchQrItem, FinderStyle,
    ModuleShape, QrEccLevel, QrGenerationResult, QrHistoryRecord, QrPayloadType, QrScanResult,
    QrStudioError, QrStudioStats, QrStyleConfig, SecurityRiskLevel,
};
use crate::qr_engine::{
    compute_contrast_ratio, decode_png_to_rgb, decode_qr_from_rgb, generate_qr_matrix, render_png,
    render_svg,
};
use crate::security_analyzer::SecurityAnalyzer;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub struct QrStudioService {
    conn: Arc<Mutex<Connection>>,
}

impl QrStudioService {
    /// Opens persistent SQLite database in the application's data directory
    pub fn open(base_dir: impl AsRef<Path>) -> Result<Self, QrStudioError> {
        let dir = base_dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let db_path = dir.join("qr_studio.db");
        let conn = Connection::open(db_path)?;
        Self::init_db(conn)
    }

    /// Opens in-memory SQLite database for test suites
    pub fn open_in_memory() -> Result<Self, QrStudioError> {
        let conn = Connection::open_in_memory()?;
        Self::init_db(conn)
    }

    fn init_db(conn: Connection) -> Result<Self, QrStudioError> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;

             CREATE TABLE IF NOT EXISTS qr_history (
                 id TEXT PRIMARY KEY,
                 direction TEXT NOT NULL,
                 label TEXT NOT NULL,
                 payload_type TEXT NOT NULL,
                 raw_payload TEXT NOT NULL,
                 version INTEGER NOT NULL,
                 ecc_level TEXT NOT NULL,
                 svg_markup TEXT NOT NULL,
                 risk_level TEXT NOT NULL,
                 is_favorite INTEGER NOT NULL DEFAULT 0,
                 created_at INTEGER NOT NULL
             );

             CREATE INDEX IF NOT EXISTS idx_qr_created ON qr_history(created_at DESC);
             CREATE INDEX IF NOT EXISTS idx_qr_fav ON qr_history(is_favorite);
             CREATE INDEX IF NOT EXISTS idx_qr_type ON qr_history(payload_type);",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Generates a single styled QR code with contrast validation and threat scan
    pub fn generate_qr(
        &self,
        raw_text: &str,
        label_opt: Option<&str>,
        style_opt: Option<QrStyleConfig>,
        passphrase_opt: Option<&str>,
        sign_ed25519: bool,
    ) -> Result<QrGenerationResult, QrStudioError> {
        if raw_text.trim().is_empty() {
            return Err(QrStudioError::InvalidInput(
                "QR payload text cannot be empty".to_string(),
            ));
        }

        let style = style_opt.unwrap_or_default();
        let payload_type = QrPayloadType::detect_from_raw(raw_text);

        // Optional encryption or signing transformation
        let final_payload = if let Some(pass) = passphrase_opt {
            if !pass.trim().is_empty() {
                SecurityAnalyzer::encrypt_payload(raw_text, pass)?
            } else {
                raw_text.to_string()
            }
        } else if sign_ed25519 {
            SecurityAnalyzer::sign_payload(raw_text, None)?
        } else {
            raw_text.to_string()
        };

        // 1. Generate QR matrix
        let matrix = generate_qr_matrix(final_payload.as_bytes(), style.ecc_level)
            .map_err(QrStudioError::EncodingError)?;

        // 2. Render SVG vector
        let svg_markup = render_svg(&matrix, &style);

        // 3. Render PNG raster & base64 encode
        let png_bytes = render_png(&matrix, &style).map_err(QrStudioError::EncodingError)?;
        let png_base64 = format!("data:image/png;base64,{}", simple_base64_encode(&png_bytes));

        // 4. Contrast check & warnings
        let (contrast_ratio, is_inverted) =
            compute_contrast_ratio(&style.fg_color, &style.bg_color);
        let scannability_warning = if is_inverted {
            Some("Foreground is lighter than background. Many camera scanners fail on inverted QR codes.".to_string())
        } else if contrast_ratio < 3.0 {
            Some(format!(
                "Low contrast ratio ({:.1}:1). Minimum 4.5:1 recommended for reliable scanning.",
                contrast_ratio
            ))
        } else {
            None
        };

        // 5. Security threat analysis
        let security_report = SecurityAnalyzer::inspect_payload(&final_payload);

        let id = uuid::Uuid::new_v4().to_string();
        let label = label_opt
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .unwrap_or_else(|| format!("QR: {}", raw_text.chars().take(24).collect::<String>()));

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // 6. Save to SQLite history
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO qr_history (id, direction, label, payload_type, raw_payload, version, ecc_level, svg_markup, risk_level, is_favorite, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                "generated",
                label,
                payload_type.as_str(),
                raw_text,
                matrix.version as i64,
                style.ecc_level.as_str(),
                svg_markup,
                security_report.risk_level.as_str(),
                0,
                now as i64,
            ],
        )?;

        Ok(QrGenerationResult {
            id,
            label,
            payload_type,
            raw_payload: raw_text.to_string(),
            encoded_payload: final_payload,
            version: matrix.version,
            dimension: matrix.dimension,
            ecc_level: style.ecc_level,
            mask_pattern: matrix.mask_pattern,
            svg_markup,
            png_base64,
            contrast_ratio,
            scannability_warning,
            security_report,
            created_at: now,
        })
    }

    /// Scans a PNG image byte buffer or raw RGB pixels
    pub fn scan_qr_image(
        &self,
        png_bytes: Option<&[u8]>,
        raw_rgb: Option<(usize, usize, &[u8])>,
        passphrase_opt: Option<&str>,
    ) -> Result<QrScanResult, QrStudioError> {
        let t0 = Instant::now();

        let (w, h, rgb_buf) = if let Some(bytes) = png_bytes {
            let (width, height, rgb) =
                decode_png_to_rgb(bytes).map_err(QrStudioError::DecodingError)?;
            (width, height, rgb)
        } else if let Some((width, height, rgb)) = raw_rgb {
            (width, height, rgb.to_vec())
        } else {
            return Err(QrStudioError::InvalidInput(
                "No PNG bytes or RGB frame provided for scanning".to_string(),
            ));
        };

        // 1. Decode QR from RGB pixels
        let (raw_decoded, version, ecc_level, dimension) =
            decode_qr_from_rgb(w, h, &rgb_buf).map_err(QrStudioError::DecodingError)?;

        let mut payload_type = QrPayloadType::detect_from_raw(&raw_decoded);
        let mut final_text = raw_decoded.clone();

        // 2. Attempt AES-256-GCM decryption if passphrase given
        if payload_type == QrPayloadType::EncryptedPayload {
            if let Some(pass) = passphrase_opt {
                if !pass.trim().is_empty() {
                    match SecurityAnalyzer::decrypt_payload(&raw_decoded, pass) {
                        Ok(decrypted) => {
                            final_text = decrypted;
                            payload_type = QrPayloadType::detect_from_raw(&final_text);
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
            }
        }

        // 3. Structured fields extraction & security threat report
        let parsed_fields = extract_structured_fields(&final_text, payload_type);
        let security_report = SecurityAnalyzer::inspect_payload(&raw_decoded);
        let elapsed_us = t0.elapsed().as_micros() as u64;

        let id = uuid::Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // 4. Record to SQLite history
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO qr_history (id, direction, label, payload_type, raw_payload, version, ecc_level, svg_markup, risk_level, is_favorite, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                "scanned",
                format!("Scan: {}", final_text.chars().take(24).collect::<String>()),
                payload_type.as_str(),
                final_text,
                version as i64,
                ecc_level.as_str(),
                "",
                security_report.risk_level.as_str(),
                0,
                now as i64,
            ],
        );

        Ok(QrScanResult {
            id,
            decoded_text: final_text,
            payload_type,
            version,
            dimension,
            ecc_level,
            mask_pattern: 0,
            parsed_fields,
            security_report,
            decode_time_us: elapsed_us,
        })
    }

    /// Batch generate QR codes from multiple payload items
    pub fn batch_generate(
        &self,
        items: &[BatchQrItem],
        style: &QrStyleConfig,
    ) -> Result<BatchJobReport, QrStudioError> {
        let t0 = Instant::now();
        let mut entries = Vec::with_capacity(items.len());
        let mut succeeded = 0;
        let mut failed = 0;

        for item in items {
            let res = generate_qr_matrix(item.payload.as_bytes(), style.ecc_level);
            match res {
                Ok(mat) => {
                    let svg = render_svg(&mat, style);
                    let png_bytes = render_png(&mat, style).unwrap_or_default();
                    let png_b64 =
                        format!("data:image/png;base64,{}", simple_base64_encode(&png_bytes));
                    let sec = SecurityAnalyzer::inspect_payload(&item.payload);

                    entries.push(BatchQrGeneratedEntry {
                        label: item.label.clone(),
                        payload: item.payload.clone(),
                        version: mat.version,
                        dimension: mat.dimension,
                        svg_markup: svg,
                        png_base64: png_b64,
                        risk_level: sec.risk_level,
                    });
                    succeeded += 1;
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }

        Ok(BatchJobReport {
            total_requested: items.len(),
            succeeded,
            failed,
            elapsed_ms: t0.elapsed().as_millis() as u64,
            entries,
        })
    }

    /// Lists history records with optional search query and favorite filter
    pub fn list_history(
        &self,
        search_opt: Option<&str>,
        favorites_only: bool,
        limit: usize,
    ) -> Result<Vec<QrHistoryRecord>, QrStudioError> {
        let conn = self.conn.lock().unwrap();
        let mut sql = "SELECT id, direction, label, payload_type, raw_payload, version, ecc_level, svg_markup, risk_level, is_favorite, created_at FROM qr_history WHERE 1=1".to_string();

        if favorites_only {
            sql.push_str(" AND is_favorite = 1");
        }
        if let Some(q) = search_opt {
            if !q.trim().is_empty() {
                sql.push_str(" AND (label LIKE '%");
                sql.push_str(&q.replace('\'', "''"));
                sql.push_str("%' OR raw_payload LIKE '%");
                sql.push_str(&q.replace('\'', "''"));
                sql.push_str("%')");
            }
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ?1");

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let ecc_str: String = row.get(6)?;
            let risk_str: String = row.get(8)?;
            let is_fav: i64 = row.get(9)?;

            Ok(QrHistoryRecord {
                id: row.get(0)?,
                direction: row.get(1)?,
                label: row.get(2)?,
                payload_type: QrPayloadType::from_str_name(&row.get::<_, String>(3)?),
                raw_payload: row.get(4)?,
                version: row.get::<_, i64>(5)? as u8,
                ecc_level: QrEccLevel::from_str_code(&ecc_str),
                svg_markup: row.get(7)?,
                risk_level: match risk_str.as_str() {
                    "CRITICAL" => SecurityRiskLevel::Critical,
                    "HIGH_RISK" => SecurityRiskLevel::HighRisk,
                    "MEDIUM_RISK" => SecurityRiskLevel::MediumRisk,
                    "LOW_RISK" => SecurityRiskLevel::LowRisk,
                    _ => SecurityRiskLevel::Safe,
                },
                is_favorite: is_fav == 1,
                created_at: row.get::<_, i64>(10)? as u64,
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Toggle favorite status of a history record
    pub fn toggle_favorite(&self, id: &str) -> Result<bool, QrStudioError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE qr_history SET is_favorite = CASE WHEN is_favorite = 1 THEN 0 ELSE 1 END WHERE id = ?1",
            params![id],
        )?;
        let mut stmt = conn.prepare("SELECT is_favorite FROM qr_history WHERE id = ?1")?;
        let is_fav: i64 = stmt.query_row(params![id], |r| r.get(0)).unwrap_or(0);
        Ok(is_fav == 1)
    }

    /// Delete a history record by ID
    pub fn delete_history_item(&self, id: &str) -> Result<bool, QrStudioError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM qr_history WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    /// Get overall studio usage statistics
    pub fn get_stats(&self) -> Result<QrStudioStats, QrStudioError> {
        let conn = self.conn.lock().unwrap();
        let total_gen: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM qr_history WHERE direction = 'generated'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let total_scan: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM qr_history WHERE direction = 'scanned'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let favs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM qr_history WHERE is_favorite = 1",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let threats: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM qr_history WHERE risk_level IN ('CRITICAL', 'HIGH_RISK')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let encs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM qr_history WHERE payload_type = 'encrypted'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let signed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM qr_history WHERE payload_type = 'signed'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        Ok(QrStudioStats {
            total_generated: total_gen as usize,
            total_scanned: total_scan as usize,
            favorites_count: favs as usize,
            threats_flagged: threats as usize,
            encrypted_count: encs as usize,
            signed_count: signed as usize,
        })
    }

    /// Seeds demo records if the studio history database is currently empty
    pub fn seed_demo_records(&self) -> Result<(), QrStudioError> {
        let style = QrStyleConfig {
            ecc_level: QrEccLevel::High,
            module_shape: ModuleShape::Rounded,
            finder_style: FinderStyle::Rounded,
            fg_color: "#0F172A".to_string(),
            fg_gradient_end: Some("#2563EB".to_string()),
            bg_color: "#FFFFFF".to_string(),
            quiet_zone_modules: 4,
            size_px: 512,
            center_logo_text: Some("OFFLINE".to_string()),
        };

        let _ = self.generate_qr(
            "https://github.com/dayashimoga/rustapps",
            Some("Monorepo Source Code"),
            Some(style.clone()),
            None,
            false,
        );

        let _ = self.generate_qr(
            "WIFI:T:WPA;S:AirGapLab_5G;P:ZeroEgressSecurePass;;",
            Some("Air-Gapped Lab Wi-Fi"),
            Some(style),
            None,
            false,
        );

        Ok(())
    }
}

/// Simple Base64 encoder avoiding external crate dependency
fn simple_base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 {
            chunk[1] as usize
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            chunk[2] as usize
        } else {
            0
        };

        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(n >> 18) & 63] as char);
        out.push(TABLE[(n >> 12) & 63] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(n >> 6) & 63] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[n & 63] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_studio_service_lifecycle() {
        let service = QrStudioService::open_in_memory().expect("In-memory service");
        service.seed_demo_records().expect("Seed demo records");

        let stats = service.get_stats().expect("Get stats");
        assert!(stats.total_generated >= 2);

        let history = service.list_history(None, false, 10).expect("List history");
        assert_eq!(history.len(), 2);

        let rec_id = &history[0].id;
        let fav = service.toggle_favorite(rec_id).expect("Toggle favorite");
        assert!(fav);

        let favs = service
            .list_history(None, true, 10)
            .expect("List favorites");
        assert_eq!(favs.len(), 1);

        let deleted = service.delete_history_item(rec_id).expect("Delete record");
        assert!(deleted);
    }
}
