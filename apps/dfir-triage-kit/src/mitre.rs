#![forbid(unsafe_code)]
//! MITRE ATT&CK Matrix Technique Mapping Engine
//!
//! Automatically correlates host telemetry, running processes, network connections,
//! file entropy, and event logs against standard MITRE ATT&CK enterprise tactics & techniques.

use crate::models::{
    FileIntegrityRecord, LogEventRecord, MitreAttackMapping, ProcessArtifact, Severity,
    SocketArtifact,
};

pub struct MitreEngine;

impl MitreEngine {
    /// Correlates all collected triage artifacts to identify MITRE ATT&CK techniques
    pub fn map_artifacts(
        processes: &[ProcessArtifact],
        sockets: &[SocketArtifact],
        files: &[FileIntegrityRecord],
        logs: &[LogEventRecord],
    ) -> Vec<MitreAttackMapping> {
        let mut mappings = Vec::new();

        // 1. Process Correlator
        for p in processes {
            let name_lower = p.name.to_ascii_lowercase();
            let cmd_lower = p.command_line.to_ascii_lowercase();

            if name_lower.contains("powershell") || name_lower.contains("pwsh") {
                mappings.push(MitreAttackMapping {
                    technique_id: "T1059.001".to_string(),
                    technique_name: "Command and Scripting Interpreter: PowerShell".to_string(),
                    tactic: "Execution".to_string(),
                    severity: if p.is_suspicious
                        || cmd_lower.contains("-enc")
                        || cmd_lower.contains("bypass")
                    {
                        Severity::High
                    } else {
                        Severity::Medium
                    },
                    evidence: format!("PID {} ({}): {}", p.pid, p.name, p.command_line),
                });
            } else if name_lower == "cmd.exe"
                || name_lower == "bash"
                || name_lower == "sh"
                || name_lower == "zsh"
            {
                mappings.push(MitreAttackMapping {
                    technique_id: "T1059.003".to_string(),
                    technique_name: "Command and Scripting Interpreter: Windows Command Shell"
                        .to_string(),
                    tactic: "Execution".to_string(),
                    severity: Severity::Low,
                    evidence: format!("PID {} ({}): {}", p.pid, p.name, p.command_line),
                });
            } else if name_lower == "rundll32.exe"
                || name_lower == "regsvr32.exe"
                || name_lower == "mshta.exe"
            {
                mappings.push(MitreAttackMapping {
                    technique_id: "T1218".to_string(),
                    technique_name: "System Binary Proxy Execution".to_string(),
                    tactic: "Defense Evasion".to_string(),
                    severity: Severity::High,
                    evidence: format!(
                        "Proxy execution binary PID {} ({}) invoked: {}",
                        p.pid, p.name, p.command_line
                    ),
                });
            } else if name_lower == "wmic.exe"
                || name_lower == "cscript.exe"
                || name_lower == "wscript.exe"
            {
                mappings.push(MitreAttackMapping {
                    technique_id: "T1047".to_string(),
                    technique_name: "Windows Management Instrumentation (WMI)".to_string(),
                    tactic: "Execution".to_string(),
                    severity: Severity::Medium,
                    evidence: format!("WMI execution binary PID {} ({}) invoked", p.pid, p.name),
                });
            }

            if p.is_suspicious {
                mappings.push(MitreAttackMapping {
                    technique_id: "T1055".to_string(),
                    technique_name: "Process Injection / Anomalous Executable".to_string(),
                    tactic: "Privilege Escalation".to_string(),
                    severity: Severity::High,
                    evidence: format!(
                        "Flagged process PID {} ({}): {}",
                        p.pid,
                        p.name,
                        p.threat_indicator
                            .as_deref()
                            .unwrap_or("Suspicious heuristics")
                    ),
                });
            }
        }

        // 2. Network Socket Correlator
        for s in sockets {
            if s.is_external {
                if let Some(remote) = &s.remote_addr {
                    let port = remote.rsplit(':').next().unwrap_or("");
                    if port == "4444"
                        || port == "1337"
                        || port == "5555"
                        || port == "6667"
                        || port == "8888"
                    {
                        mappings.push(MitreAttackMapping {
                            technique_id: "T1571".to_string(),
                            technique_name: "Non-Standard Port Communication".to_string(),
                            tactic: "Command and Control".to_string(),
                            severity: Severity::Critical,
                            evidence: format!(
                                "Socket connected to known adversarial port {} ({}) by PID {}",
                                port, remote, s.pid
                            ),
                        });
                    } else {
                        mappings.push(MitreAttackMapping {
                            technique_id: "T1071.001".to_string(),
                            technique_name: "Application Layer Protocol: Web Protocols".to_string(),
                            tactic: "Command and Control".to_string(),
                            severity: Severity::Low,
                            evidence: format!(
                                "External outbound connection to {} by PID {}",
                                remote, s.pid
                            ),
                        });
                    }
                }
            }
        }

        // 3. File Integrity Correlator
        for f in files {
            if f.entropy > 7.2 {
                mappings.push(MitreAttackMapping {
                    technique_id: "T1027.002".to_string(),
                    technique_name: "Obfuscated/Compressed: Software Packing".to_string(),
                    tactic: "Defense Evasion".to_string(),
                    severity: Severity::High,
                    evidence: format!("High entropy binary ({:.2}): {}", f.entropy, f.path),
                });
            }

            let path_lower = f.path.to_ascii_lowercase();
            if path_lower.contains("start menu")
                || path_lower.contains("startup")
                || path_lower.contains("systemd")
                || path_lower.contains("launchagents")
            {
                mappings.push(MitreAttackMapping {
                    technique_id: "T1547.001".to_string(),
                    technique_name: "Boot or Logon Autostart Execution: Startup Folder / Service"
                        .to_string(),
                    tactic: "Persistence".to_string(),
                    severity: Severity::Medium,
                    evidence: format!("Persistence path modification: {}", f.path),
                });
            }
        }

        // 4. Log Event Correlator
        for l in logs {
            match l.event_id {
                4625 => {
                    mappings.push(MitreAttackMapping {
                        technique_id: "T1110.001".to_string(),
                        technique_name: "Brute Force: Password Guessing".to_string(),
                        tactic: "Credential Access".to_string(),
                        severity: Severity::Medium,
                        evidence: format!(
                            "Event ID 4625 (Logon failure) observed for user: {:?}",
                            l.user_id
                        ),
                    });
                }
                7045 => {
                    mappings.push(MitreAttackMapping {
                        technique_id: "T1543.003".to_string(),
                        technique_name: "Create or Modify System Process: Windows Service"
                            .to_string(),
                        tactic: "Persistence".to_string(),
                        severity: Severity::High,
                        evidence: format!("Event ID 7045 (New service installed): {}", l.message),
                    });
                }
                1102 | 104 => {
                    mappings.push(MitreAttackMapping {
                        technique_id: "T1070.001".to_string(),
                        technique_name: "Indicator Removal: Clear Windows Event Logs".to_string(),
                        tactic: "Defense Evasion".to_string(),
                        severity: Severity::Critical,
                        evidence: format!("Security event log cleared (Event ID {})", l.event_id),
                    });
                }
                _ => {}
            }
        }

        // Deduplicate mappings by technique_id and evidence
        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::new();
        for m in mappings {
            let key = format!("{}:{}", m.technique_id, m.evidence);
            if seen.insert(key) {
                deduped.push(m);
            }
        }

        deduped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LogLevel, SocketProtocol, SocketState};

    #[test]
    fn test_mitre_mapping_powershell_and_evtx() {
        let procs = vec![ProcessArtifact {
            pid: 1234,
            ppid: 456,
            name: "powershell.exe".to_string(),
            exe_path: "C:\\Windows\\System32\\powershell.exe".to_string(),
            command_line: "powershell.exe -enc AAAA".to_string(),
            sha256: "dummy".to_string(),
            user: "admin".to_string(),
            cpu_percent: 0.1,
            memory_mb: 40,
            is_suspicious: true,
            threat_indicator: Some("Encoded command line parameter".to_string()),
        }];

        let sockets = vec![SocketArtifact {
            protocol: SocketProtocol::Tcp,
            local_addr: "192.168.1.5:54321".to_string(),
            remote_addr: Some("198.51.100.1:4444".to_string()),
            state: SocketState::Established,
            pid: 1234,
            process_name: "powershell.exe".to_string(),
            is_external: true,
        }];

        let files = vec![FileIntegrityRecord {
            path: "C:\\Windows\\Temp\\payload.exe".to_string(),
            file_size_bytes: 40960,
            blake3_hash: "blake".to_string(),
            sha256_hash: "sha".to_string(),
            entropy: 7.85,
            is_suspicious: true,
            anomaly_notes: Some("High entropy executable".to_string()),
        }];

        let logs = vec![LogEventRecord {
            timestamp: 1700000000,
            source: "Security".to_string(),
            event_id: 1102,
            level: LogLevel::Critical,
            message: "Audit log cleared".to_string(),
            user_id: Some("admin".to_string()),
        }];

        let techniques = MitreEngine::map_artifacts(&procs, &sockets, &files, &logs);
        assert!(!techniques.is_empty());

        let t_ids: Vec<&str> = techniques.iter().map(|t| t.technique_id.as_str()).collect();
        assert!(t_ids.contains(&"T1059.001")); // PowerShell
        assert!(t_ids.contains(&"T1571")); // Port 4444
        assert!(t_ids.contains(&"T1027.002")); // High entropy packing
        assert!(t_ids.contains(&"T1070.001")); // Log cleared
    }
}
