#![deny(unsafe_code)]
//! Offline Digital Forensics & Incident Response (DFIR) Triage Kit Desktop GUI & CLI Entrypoint

use dfir_triage_kit::commands::dispatch_ipc_command;
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "dfir-triage-kit",
    app_title: "DFIR Triage Kit — 100% Offline Host Forensics & Incident Analysis",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1480,
    window_height: 920,
};

struct DfirDesktopHandler;

impl DesktopIpcHandler for DfirDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        vec![
            SelfTestCheck {
                command: "get_dfir_stats",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "list_triage_runs",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "execute_triage",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "scan_directory",
                payload: json!({ "path": ".", "max_depth": 1 }),
            },
            SelfTestCheck {
                command: "get_mitre_mapping",
                payload: json!({ "triage_id": "latest" }),
            },
            SelfTestCheck {
                command: "get_license_status",
                payload: json!({}),
            },
        ]
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "Offline Digital Forensics & Incident Response (DFIR) Triage Kit v{}",
            env!("CARGO_PKG_VERSION")
        );
        println!("Usage: dfir-triage-kit [OPTIONS]");
        println!("Options:");
        println!("  --collect, -c       Headless USB triage collection mode (runs without GUI)");
        println!("  --output, -o <PATH> Output file path for signed .dfir evidence package");
        println!("  --scan-dir, -d <PATH> Target directory to scan for file integrity/entropy");
        println!("  --verify <PATH>     Verify cryptographic Ed25519 signature of a .dfir package");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --version, -v       Show version");
        return;
    }

    // Headless Verification Mode
    if let Some(pos) = args.iter().position(|a| a == "--verify") {
        if pos + 1 < args.len() {
            let path = &args[pos + 1];
            println!(
                "Verifying cryptographic chain-of-custody for package: {}",
                path
            );
            match dfir_triage_kit::sealer::EvidenceSealer::verify_package(path) {
                Ok(report) => {
                    println!(
                        "Status: {}",
                        if report.is_valid {
                            "VERIFIED (AUTHENTIC)"
                        } else {
                            "TAMPERED / INVALID"
                        }
                    );
                    println!("Signature Verified: {}", report.signature_verified);
                    println!("Files Verified: {}", report.files_verified);
                    println!("Signer Public Key: {}", report.signer_public_key);
                    println!("Notes: {}", report.notes);
                    if !report.is_valid {
                        std::process::exit(1);
                    }
                    return;
                }
                Err(e) => {
                    eprintln!("Verification error: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("Error: --verify requires a package path argument");
            std::process::exit(1);
        }
    }

    // Headless Single-Binary CLI Collector Mode
    if args.iter().any(|a| a == "--collect" || a == "-c") {
        let mut output_path = None;
        let mut scan_dir = None;

        let mut i = 1;
        while i < args.len() {
            if (args[i] == "--output" || args[i] == "-o") && i + 1 < args.len() {
                output_path = Some(args[i + 1].clone());
                i += 1;
            } else if (args[i] == "--scan-dir" || args[i] == "-d") && i + 1 < args.len() {
                scan_dir = Some(args[i + 1].clone());
                i += 1;
            }
            i += 1;
        }

        println!("=== DFIR Triage Kit — Headless USB Evidence Collector ===");
        let service = match dfir_triage_kit::service::DfirService::open_in_memory() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to initialize in-memory triage engine: {}", e);
                std::process::exit(1);
            }
        };

        let target_dir_path = scan_dir.as_ref().map(std::path::Path::new);
        println!("[*] Executing host volatile triage sweep...");
        let package = match service.execute_triage(target_dir_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Triage acquisition error: {}", e);
                std::process::exit(1);
            }
        };

        let default_filename = format!(
            "triage-{}-{}.dfir",
            package.summary.host_name, package.summary.collected_at
        );
        let out_file = output_path.unwrap_or(default_filename);

        println!(
            "[*] Cryptographically sealing artifacts into signed bundle: {}...",
            out_file
        );
        let manifest = match service.export_signed_bundle(&package.summary.triage_id, &out_file) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to seal evidence package: {}", e);
                std::process::exit(1);
            }
        };

        println!("\n--- Chain-of-Custody Summary ---");
        println!("Triage ID:            {}", manifest.package_id);
        println!(
            "Host / OS:            {} ({})",
            manifest.host_name, manifest.os_type
        );
        println!(
            "Overall Severity:     {:?} (Threat Score: {})",
            package.summary.severity, package.summary.threat_score
        );
        println!("Artifacts Sealed:     {} files", manifest.file_hashes.len());
        println!("SHA-256 Package Hash: {}", manifest.sha256_package_hash);
        println!("BLAKE3 Package Hash:  {}", manifest.blake3_package_hash);
        println!(
            "Ed25519 Signature:    {}...",
            &manifest.ed25519_signature[..32]
        );
        println!("Signer Public Key:    {}", manifest.public_key);
        println!(
            "MITRE Techniques:     {} mapped",
            manifest.mitre_summary.len()
        );
        for m in &manifest.mitre_summary {
            println!(
                "  - [{}] {} ({:?}): {}",
                m.technique_id, m.technique_name, m.severity, m.evidence
            );
        }
        println!("\nEvidence collection complete. Standalone package sealed successfully.");
        return;
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, DfirDesktopHandler) {
        eprintln!("DFIR Triage Kit desktop runtime error: {}", e);
        std::process::exit(1);
    }
}
