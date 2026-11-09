#![forbid(unsafe_code)]
//! # Antigravity Offline License Signer CLI (`tools/license-signer`)
//!
//! Generates, cryptographically signs, and validates offline Ed25519 `.offlic` license certificates
//! with zero network egress, adhering to strict offline enterprise security standards.

use shared_licensing::{
    AppId, LicenseAuthority, LicenseCertificate, LicenseParameters, LicenseTier, LicenseVerifier,
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_usage();
        return;
    }

    if args.contains(&"--self-test".to_string()) {
        run_self_test();
        return;
    }

    if args.contains(&"--keygen".to_string()) {
        generate_keypair();
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--verify") {
        if pos + 1 >= args.len() {
            eprintln!("Error: --verify requires a path to a .offlic file");
            std::process::exit(1);
        }
        let cert_path = &args[pos + 1];
        verify_license_file(cert_path);
        return;
    }

    // Parse options for minting a license
    let mut app_id = AppId::AllAppsSuite;
    let mut tier = LicenseTier::Enterprise;
    let mut days: u32 = 365;
    let mut licensee_name = "Licensed Enterprise User".to_string();
    let mut licensee_email = "licensee@antigravity.local".to_string();
    let mut seats: Option<u32> = None;
    let mut features: Vec<String> = Vec::new();
    let mut out_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--app" => {
                i += 1;
                if i < args.len() {
                    match AppId::from_str_lossy(&args[i]) {
                        Some(app) => app_id = app,
                        None => {
                            eprintln!("Error: Unrecognized app ID '{}'", args[i]);
                            eprintln!("Valid apps: devops-secret-vault, universal-log-viewer, ai-file-explorer, universal-live-translator, pro-av-editor, personal-knowledge-graph, api-protocol-studio, parquet-analytics-workbench, dfir-triage-kit, screen-recorder-studio, signal-telemetry-oscilloscope, audio-mastering-studio, qr-code-studio, all");
                            std::process::exit(1);
                        }
                    }
                }
            }
            "--tier" => {
                i += 1;
                if i < args.len() {
                    match LicenseTier::from_str_lossy(&args[i]) {
                        Some(t) => tier = t,
                        None => {
                            eprintln!("Error: Unrecognized license tier '{}'", args[i]);
                            eprintln!("Valid tiers: Free, Pro, ProLifetime, Business, Enterprise");
                            std::process::exit(1);
                        }
                    }
                }
            }
            "--days" => {
                i += 1;
                if i < args.len() {
                    days = args[i].parse().unwrap_or(365);
                }
            }
            "--name" => {
                i += 1;
                if i < args.len() {
                    licensee_name = args[i].clone();
                }
            }
            "--email" => {
                i += 1;
                if i < args.len() {
                    licensee_email = args[i].clone();
                }
            }
            "--seats" => {
                i += 1;
                if i < args.len() {
                    seats = args[i].parse().ok();
                }
            }
            "--features" => {
                i += 1;
                if i < args.len() {
                    features = args[i]
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            "--out" => {
                i += 1;
                if i < args.len() {
                    out_path = Some(PathBuf::from(&args[i]));
                }
            }
            other => {
                eprintln!("Unknown argument: {}", other);
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let default_seats = match tier {
        LicenseTier::Free => 1,
        LicenseTier::Premium => 1,
        LicenseTier::ProLifetime => 3,
        LicenseTier::Business => 25,
        LicenseTier::Enterprise => 250,
    };
    let final_seats = seats.unwrap_or(default_seats);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let expiry_timestamp_secs = if tier == LicenseTier::ProLifetime || days == 0 {
        None
    } else {
        Some(now + (days as u64) * 86400)
    };

    let license_id = format!(
        "AGLIC-{}-{}",
        tier.as_str().to_uppercase(),
        uuid::Uuid::new_v4()
    );

    let authority = LicenseAuthority::official();
    let cert = authority.sign_license(LicenseParameters {
        license_id,
        licensee_name,
        licensee_email,
        tier,
        app_id,
        issued_timestamp_secs: now,
        expiry_timestamp_secs,
        max_seats: final_seats,
        features,
    });

    let json =
        serde_json::to_string_pretty(&cert).expect("Failed to serialize license certificate");

    if let Some(path) = out_path {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&path, &json).unwrap_or_else(|e| {
            eprintln!("Failed to write license file to {}: {}", path.display(), e);
            std::process::exit(1);
        });
        println!("Successfully generated and signed offline license certificate:");
        println!("  File: {}", path.display());
        println!("  App: {:?}", cert.app_id);
        println!("  Tier: {:?}", cert.tier);
        println!(
            "  Licensee: {} <{}>",
            cert.licensee_name, cert.licensee_email
        );
        println!("  Seats: {}", cert.max_seats);
        if let Some(exp) = cert.expiry_timestamp_secs {
            let remaining_days = (exp.saturating_sub(now)) / 86400;
            println!(
                "  Validity: {} days (expires timestamp {})",
                remaining_days, exp
            );
        } else {
            println!("  Validity: Perpetual (Lifetime)");
        }
    } else {
        println!("{}", json);
    }
}

fn print_usage() {
    println!(
        r#"Antigravity Offline License Signer CLI
======================================
Generates and cryptographically signs Ed25519 offline license certificates (.offlic)

USAGE:
  license-signer [OPTIONS]
  license-signer --verify <PATH>
  license-signer --keygen
  license-signer --self-test

OPTIONS:
  --app <ID>          Target app ID (default: all)
                      Values: devops-secret-vault, universal-log-viewer, ai-file-explorer,
                              universal-live-translator, pro-av-editor, personal-knowledge-graph,
                              api-protocol-studio, parquet-analytics-workbench, dfir-triage-kit,
                              screen-recorder-studio, signal-telemetry-oscilloscope,
                              audio-mastering-studio, qr-code-studio, all
  --tier <TIER>       License tier (default: Enterprise)
                      Values: Free, Pro, ProLifetime, Business, Enterprise
  --days <DAYS>       Validity in days (default: 365; 0 for perpetual)
  --name <NAME>       Licensee name or organization (default: "Licensed Enterprise User")
  --email <EMAIL>     Licensee email (default: "licensee@antigravity.local")
  --seats <N>         Maximum seats (default: based on tier)
  --features <A,B>    Comma-separated list of granted feature flags
  --out <PATH>        Output file path for the .offlic JSON (default: stdout)
  --verify <PATH>     Verify an existing .offlic file
  --keygen            Generate a new standalone Ed25519 seed & public keypair
  --self-test         Run full verification test suite

EXAMPLES:
  license-signer --app devops-secret-vault --tier Enterprise --days 365 --out vault.offlic
  license-signer --app all --tier ProLifetime --name "Jane Doe" --out suite.offlic
  license-signer --verify vault.offlic
"#
    );
}

fn verify_license_file(path_str: &str) {
    let path = Path::new(path_str);
    if !path.exists() {
        eprintln!("Error: License file does not exist: {}", path.display());
        std::process::exit(1);
    }

    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading license file: {}", e);
        std::process::exit(1);
    });

    let cert: LicenseCertificate = serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Error: Invalid JSON certificate: {}", e);
        std::process::exit(1);
    });

    let verifier = LicenseVerifier::official();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    match verifier.verify(&cert, cert.app_id, now) {
        Ok(verified) => {
            println!("VALID OFFLINE LICENSE CERTIFICATE");
            println!("---------------------------------");
            println!("License ID: {}", verified.license_id);
            println!("Licensee:   {}", verified.licensee_name);
            println!("Tier:       {:?}", verified.tier);
            println!("App ID:     {:?}", verified.app_id);
            println!("Perpetual:  {}", verified.is_perpetual);
            println!("Features:   {:?}", verified.features);
            println!("Status:     CRYPTOGRAPHICALLY VERIFIED (Ed25519 OK)");
        }
        Err(e) => {
            eprintln!("INVALID LICENSE CERTIFICATE: {}", e);
            std::process::exit(1);
        }
    }
}

fn generate_keypair() {
    let authority = LicenseAuthority::generate();
    let pub_key = authority.public_key_bytes();
    println!("GENERATED NEW ED25519 AUTHORITY KEYPAIR");
    println!("========================================");
    println!(
        "Public Key (hex): {}",
        pub_key
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    );
    println!("Public Key (bytes): {:?}", pub_key);
}

fn run_self_test() {
    println!("Running license-signer self-test...");
    let authority = LicenseAuthority::official();
    let verifier = LicenseVerifier::official();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // 1. Sign Enterprise suite license
    let cert = authority.sign_license(LicenseParameters {
        license_id: "SELF-TEST-001".to_string(),
        licensee_name: "Self-Test Organization".to_string(),
        licensee_email: "test@local".to_string(),
        tier: LicenseTier::Enterprise,
        app_id: AppId::AllAppsSuite,
        issued_timestamp_secs: now,
        expiry_timestamp_secs: Some(now + 86400 * 30),
        max_seats: 50,
        features: vec!["all_features".to_string()],
    });

    // 2. Verify against individual app (DevopsSecretVault)
    let verified = verifier
        .verify(&cert, AppId::DevopsSecretVault, now)
        .expect("Enterprise suite license must verify for DevopsSecretVault");
    assert_eq!(verified.tier, LicenseTier::Enterprise);

    // 3. Serialization roundtrip
    let json = serde_json::to_string(&cert).expect("Must serialize to JSON");
    let deserialized: LicenseCertificate = serde_json::from_str(&json).expect("Must deserialize");
    assert_eq!(cert, deserialized);

    // 4. Verify tampered certificate fails
    let mut tampered = cert.clone();
    tampered.licensee_name = "Tampered Name".to_string();
    assert!(
        verifier
            .verify(&tampered, AppId::DevopsSecretVault, now)
            .is_err(),
        "Tampered cert must fail signature check"
    );

    // 5. Verify expired certificate fails
    let mut expired = cert.clone();
    expired.expiry_timestamp_secs = Some(now - 100);
    // Sign the expired cert to verify signature is valid but expiration check triggers
    let expired_signed = authority.sign_license(LicenseParameters {
        license_id: "SELF-TEST-EXPIRED".to_string(),
        licensee_name: "Expired User".to_string(),
        licensee_email: "expired@local".to_string(),
        tier: LicenseTier::Premium,
        app_id: AppId::AiFileExplorer,
        issued_timestamp_secs: now - 200,
        expiry_timestamp_secs: Some(now - 100),
        max_seats: 1,
        features: vec![],
    });
    assert!(
        verifier
            .verify(&expired_signed, AppId::AiFileExplorer, now)
            .is_err(),
        "Expired cert must be rejected"
    );

    println!("license-signer self-test PASSED (5 checks passed).");
}
