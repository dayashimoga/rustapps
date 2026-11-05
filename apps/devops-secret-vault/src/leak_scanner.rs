#![forbid(unsafe_code)]
//! Offline Secret Leak Scanner
//!
//! Scans local workspace directories for leaked private keys, cloud tokens,
//! database connection strings, and unencrypted environment secrets.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LeakFinding {
    pub file_path: String,
    pub line_number: usize,
    pub leak_type: String,
    pub severity: String,
    pub snippet: String,
}

/// Recursively scans target directory for leaked secrets with max depth & file size guards
pub fn scan_dir_for_leaks(root: &Path) -> Vec<LeakFinding> {
    let mut findings = Vec::new();
    if !root.exists() {
        return findings;
    }

    fn recurse_scan(dir: &Path, findings: &mut Vec<LeakFinding>, depth: usize) {
        if depth > 8 || findings.len() >= 100 {
            return;
        }
        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return,
        };

        for entry in read_dir.flatten() {
            if findings.len() >= 100 {
                return;
            }
            let path = entry.path();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with(".git")
                || name == "node_modules"
                || name == "target"
                || name == "vendor"
                || name == "dist"
            {
                continue;
            }

            if path.is_dir() {
                recurse_scan(&path, findings, depth + 1);
            } else if path.is_file() {
                if let Ok(meta) = entry.metadata() {
                    if meta.len() > 1_500_000 {
                        continue;
                    }
                }
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for (idx, line) in content.lines().enumerate() {
                        let line_num = idx + 1;
                        if line.contains("BEGIN RSA PRIVATE KEY")
                            || line.contains("BEGIN OPENSSH PRIVATE KEY")
                            || line.contains("BEGIN EC PRIVATE KEY")
                            || line.contains("BEGIN PRIVATE KEY")
                        {
                            findings.push(LeakFinding {
                                file_path: path.to_string_lossy().to_string(),
                                line_number: line_num,
                                leak_type: "Private Key".to_string(),
                                severity: "CRITICAL".to_string(),
                                snippet: "-----BEGIN PRIVATE KEY----- [REDACTED]".to_string(),
                            });
                        } else if let Some(pos) = line.find("AKIA") {
                            if line[pos..].len() >= 20 {
                                let potential = &line[pos..pos + 20];
                                if potential.chars().all(|c| c.is_ascii_alphanumeric()) {
                                    findings.push(LeakFinding {
                                        file_path: path.to_string_lossy().to_string(),
                                        line_number: line_num,
                                        leak_type: "AWS Access Key".to_string(),
                                        severity: "HIGH".to_string(),
                                        snippet: format!("{}****", &potential[..8]),
                                    });
                                }
                            }
                        } else if let Some(pos) = line.find("ghp_") {
                            if line[pos..].len() >= 36 {
                                findings.push(LeakFinding {
                                    file_path: path.to_string_lossy().to_string(),
                                    line_number: line_num,
                                    leak_type: "GitHub PAT".to_string(),
                                    severity: "HIGH".to_string(),
                                    snippet: "ghp_************************".to_string(),
                                });
                            }
                        } else if line.contains("postgres://")
                            || line.contains("mysql://")
                            || line.contains("mongodb://")
                        {
                            if line.contains('@') && line.contains(':') {
                                findings.push(LeakFinding {
                                    file_path: path.to_string_lossy().to_string(),
                                    line_number: line_num,
                                    leak_type: "Database Connection String".to_string(),
                                    severity: "CRITICAL".to_string(),
                                    snippet: line.chars().take(30).collect::<String>()
                                        + " [REDACTED]",
                                });
                            }
                        } else if (name == ".env"
                            || path.extension().and_then(|s| s.to_str()).unwrap_or("") == "env")
                            && (line.contains("SECRET")
                                || line.contains("PASSWORD")
                                || line.contains("KEY")
                                || line.contains("TOKEN"))
                            && line.contains('=')
                        {
                            let parts: Vec<&str> = line.splitn(2, '=').collect();
                            if parts.len() == 2 && !parts[1].trim().is_empty() {
                                findings.push(LeakFinding {
                                    file_path: path.to_string_lossy().to_string(),
                                    line_number: line_num,
                                    leak_type: "Unencrypted Environment Secret".to_string(),
                                    severity: "MEDIUM".to_string(),
                                    snippet: format!("{}=********", parts[0].trim()),
                                });
                            }
                        }
                        if findings.len() >= 100 {
                            break;
                        }
                    }
                }
            }
        }
    }

    recurse_scan(root, &mut findings, 0);
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scan_dir_for_leaks() {
        let dir = tempdir().unwrap();
        let key_file = dir.path().join("id_rsa");
        std::fs::write(&key_file, "-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n").unwrap();

        let env_file = dir.path().join(".env");
        std::fs::write(&env_file, "DATABASE_PASSWORD=super_secret_123\n").unwrap();

        let clean_file = dir.path().join("readme.md");
        std::fs::write(&clean_file, "# Documentation\nAll safe here.\n").unwrap();

        let findings = scan_dir_for_leaks(dir.path());
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.leak_type == "Private Key"));
        assert!(findings
            .iter()
            .any(|f| f.leak_type == "Unencrypted Environment Secret"));
    }
}
