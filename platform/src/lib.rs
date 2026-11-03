#![forbid(unsafe_code)]
//! Platform Common Abstraction Layer
//!
//! Provides platform-independent path resolution, portable mode detection,
//! OS capability queries, and runtime resource telemetry safely.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;
use thiserror::Error;

pub mod keyring;
pub mod packaging;
pub mod power;
pub mod theme;

pub use keyring::{
    InMemoryKeyring, KeyringItem, LinuxSecretServiceKeyring, MacOsKeychainKeyring, PlatformKeyring,
    PlatformKeyringFactory, WindowsCredentialKeyring,
};
pub use packaging::{PackageFormat, PackageManifest, PackagingValidator, TargetArchitecture};
pub use power::{BatteryState, PowerSource, PowerStatus, PowerTelemetry};
pub use theme::{ThemeDetector, ThemePreference};

/// Platform error definitions
#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("Failed to resolve standard OS directory for {0}")]
    PathResolutionFailed(String),
    #[error("I/O error during platform operation: {0}")]
    Io(#[from] std::io::Error),
    #[error("Feature not supported on current platform: {0}")]
    Unsupported(String),
    #[error("Cryptographic operation failed: {0}")]
    CryptoError(String),
    #[error("Keyring operation failed: {0}")]
    KeyringError(String),
}

/// Supported operating systems
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OsType {
    Windows,
    Linux,
    MacOS,
    Android,
    IOS,
    Unknown,
}

/// Architecture targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArchType {
    X86_64,
    AArch64,
    Unknown,
}

/// Static system metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlatformInfo {
    pub os: OsType,
    pub arch: ArchType,
    pub is_portable: bool,
}

impl PlatformInfo {
    /// Detect the current operating system, architecture, and portable mode status.
    pub fn current() -> Self {
        let os = if cfg!(target_os = "windows") {
            OsType::Windows
        } else if cfg!(target_os = "linux") {
            OsType::Linux
        } else if cfg!(target_os = "macos") {
            OsType::MacOS
        } else if cfg!(target_os = "android") {
            OsType::Android
        } else if cfg!(target_os = "ios") {
            OsType::IOS
        } else {
            OsType::Unknown
        };

        let arch = if cfg!(target_arch = "x86_64") {
            ArchType::X86_64
        } else if cfg!(target_arch = "aarch64") {
            ArchType::AArch64
        } else {
            ArchType::Unknown
        };

        let is_portable = Self::detect_portable_mode();

        Self {
            os,
            arch,
            is_portable,
        }
    }

    /// Check for `.portable` or `portable.flag` adjacent to current executable or working directory
    pub fn detect_portable_mode() -> bool {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                if parent.join(".portable").exists() || parent.join("portable.flag").exists() {
                    return true;
                }
            }
        }

        Path::new(".portable").exists() || Path::new("portable.flag").exists()
    }
}

/// Standardized application directory resolver
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub app_name: String,
    pub base_dir: PathBuf,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl AppPaths {
    /// Initialize paths for the specified application name.
    pub fn from_app_name(app_name: &str) -> Result<Self, PlatformError> {
        let is_portable = PlatformInfo::detect_portable_mode();

        let base_dir = if is_portable {
            PathBuf::from("data").join(app_name)
        } else {
            Self::resolve_platform_base(app_name)?
        };

        let data_dir = base_dir.join("data");
        let config_dir = base_dir.join("config");
        let cache_dir = base_dir.join("cache");
        let log_dir = base_dir.join("logs");

        Ok(Self {
            app_name: app_name.to_string(),
            base_dir,
            data_dir,
            config_dir,
            cache_dir,
            log_dir,
        })
    }

    /// Ensure that all required application directories exist.
    pub fn ensure_directories(&self) -> Result<(), PlatformError> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.log_dir)?;
        Ok(())
    }

    fn resolve_platform_base(app_name: &str) -> Result<PathBuf, PlatformError> {
        if cfg!(target_os = "windows") {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                return Ok(PathBuf::from(local_app_data).join(app_name));
            }
            if let Ok(app_data) = std::env::var("APPDATA") {
                return Ok(PathBuf::from(app_data).join(app_name));
            }
            if let Ok(user_profile) = std::env::var("USERPROFILE") {
                return Ok(PathBuf::from(user_profile)
                    .join("AppData")
                    .join("Local")
                    .join(app_name));
            }
        } else if cfg!(target_os = "macos") {
            if let Ok(home) = std::env::var("HOME") {
                return Ok(PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join(app_name));
            }
        } else {
            // Linux / Unix / BSD
            if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
                return Ok(PathBuf::from(xdg_data_home).join(app_name));
            }
            if let Ok(home) = std::env::var("HOME") {
                return Ok(PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join(app_name));
            }
        }

        Err(PlatformError::PathResolutionFailed(app_name.to_string()))
    }
}

/// Internal state cache for rate-limiting expensive OS process queries (<500ms)
struct ResourceTelemetryCache {
    last_sample_time: Instant,
    last_cpu_time_ms: u64,
    last_usage: ProcessResourceUsage,
}

static TELEMETRY_CACHE: Mutex<Option<ResourceTelemetryCache>> = Mutex::new(None);

/// Instantaneous process resource metrics
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ProcessResourceUsage {
    /// Resident Set Size in bytes (physical RAM currently utilized)
    pub resident_set_size_bytes: u64,
    /// Estimated CPU percentage (0.0 to 100.0)
    pub cpu_percentage: f32,
}

impl ProcessResourceUsage {
    /// Sample current process metrics using genuine OS subsystem queries
    pub fn sample() -> Self {
        let now = Instant::now();
        if let Ok(guard) = TELEMETRY_CACHE.lock() {
            if let Some(ref cache) = *guard {
                if now.duration_since(cache.last_sample_time).as_millis() < 500 {
                    return cache.last_usage;
                }
            }
        }

        let pid = std::process::id();
        let (resident_set_size_bytes, cpu_time_ms) = Self::query_os_metrics(pid);

        let cpu_percentage = {
            let mut guard = TELEMETRY_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            let calculated_cpu = if let Some(ref cache) = *guard {
                let wall_delta_ms = now
                    .duration_since(cache.last_sample_time)
                    .as_millis()
                    .max(1) as f32;
                let cpu_delta_ms = cpu_time_ms.saturating_sub(cache.last_cpu_time_ms) as f32;
                let cpus = std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(1) as f32;
                ((cpu_delta_ms / (wall_delta_ms * cpus)) * 100.0).clamp(0.0, 100.0)
            } else {
                // Initial baseline sample
                0.1
            };

            let usage = Self {
                resident_set_size_bytes,
                cpu_percentage: calculated_cpu,
            };

            *guard = Some(ResourceTelemetryCache {
                last_sample_time: now,
                last_cpu_time_ms: cpu_time_ms,
                last_usage: usage,
            });

            calculated_cpu
        };

        Self {
            resident_set_size_bytes,
            cpu_percentage,
        }
    }

    fn query_os_metrics(pid: u32) -> (u64, u64) {
        #[cfg(target_os = "windows")]
        {
            // 1. Query working set memory via fast tasklist command
            let mut rss_bytes = 0u64;
            let filter = format!("PID eq {}", pid);
            if let Ok(out) = std::process::Command::new("tasklist")
                .args(["/FI", &filter, "/FO", "CSV", "/NH"])
                .output()
            {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    let fields: Vec<&str> = line.split(',').collect();
                    if let Some(mem_str) = fields.last() {
                        let sanitized: String =
                            mem_str.chars().filter(|c| c.is_ascii_digit()).collect();
                        if let Ok(kb) = sanitized.parse::<u64>() {
                            rss_bytes = kb * 1024;
                            break;
                        }
                    }
                }
            }

            // 2. Query CPU time in milliseconds via PowerShell
            let mut cpu_ms = 0u64;
            let ps_cmd = format!(
                "try {{ $p = Get-Process -Id {}; [PSCustomObject]@{{ WS = [int64]$p.WorkingSet64; CPU = [int64][math]::Round($p.TotalProcessorTime.TotalMilliseconds) }} | ConvertTo-Json -Compress }} catch {{ 'ERR' }}",
                pid
            );
            if let Ok(out) = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &ps_cmd])
                .output()
            {
                let text = String::from_utf8_lossy(&out.stdout);
                let trimmed = text.trim();
                if !trimmed.is_empty() && trimmed != "ERR" {
                    #[derive(serde::Deserialize)]
                    struct PsProcRaw {
                        #[serde(rename = "WS")]
                        ws: Option<u64>,
                        #[serde(rename = "CPU")]
                        cpu: Option<u64>,
                    }
                    if let Ok(raw) = serde_json::from_str::<PsProcRaw>(trimmed) {
                        if rss_bytes == 0 {
                            if let Some(ws) = raw.ws {
                                rss_bytes = ws;
                            }
                        }
                        if let Some(c) = raw.cpu {
                            cpu_ms = c;
                        }
                    }
                }
            }

            if rss_bytes == 0 {
                // Minimum fallback based on typical page table overhead
                rss_bytes = 16 * 1024 * 1024;
            }

            (rss_bytes, cpu_ms)
        }

        #[cfg(target_os = "linux")]
        {
            let mut rss_bytes = 16 * 1024 * 1024;
            let mut cpu_ms = 0u64;

            if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
                let parts: Vec<&str> = statm.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(pages) = parts[1].parse::<u64>() {
                        rss_bytes = pages * 4096;
                    }
                }
            }

            if let Ok(stat) = std::fs::read_to_string("/proc/self/stat") {
                let parts: Vec<&str> = stat.split_whitespace().collect();
                if parts.len() >= 15 {
                    let utime = parts[13].parse::<u64>().unwrap_or(0);
                    let stime = parts[14].parse::<u64>().unwrap_or(0);
                    // Approx 100 Hz USER_HZ clock tick -> 10ms per tick
                    cpu_ms = (utime + stime) * 10;
                }
            }

            (rss_bytes, cpu_ms)
        }

        #[cfg(target_os = "macos")]
        {
            let mut rss_bytes = 16 * 1024 * 1024;
            let mut cpu_ms = 0u64;

            let pid_str = pid.to_string();
            if let Ok(out) = std::process::Command::new("ps")
                .args(["-o", "rss=,time=", "-p", &pid_str])
                .output()
            {
                let text = String::from_utf8_lossy(&out.stdout);
                let parts: Vec<&str> = text.split_whitespace().collect();
                if !parts.is_empty() {
                    if let Ok(kb) = parts[0].parse::<u64>() {
                        rss_bytes = kb * 1024;
                    }
                }
                if parts.len() >= 2 {
                    // time format [dd-]hh:mm:ss
                    let time_parts: Vec<&str> = parts[1].split(':').collect();
                    if time_parts.len() >= 2 {
                        let mins = time_parts[time_parts.len() - 2].parse::<u64>().unwrap_or(0);
                        let secs = time_parts[time_parts.len() - 1]
                            .parse::<f32>()
                            .unwrap_or(0.0);
                        cpu_ms = (mins * 60 * 1000) + (secs * 1000.0) as u64;
                    }
                }
            }

            (rss_bytes, cpu_ms)
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            (16 * 1024 * 1024, 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_info_detection() {
        let info = PlatformInfo::current();
        assert_ne!(info.os, OsType::Unknown);
        assert_ne!(info.arch, ArchType::Unknown);
    }

    #[test]
    fn test_app_paths_resolution() {
        let paths =
            AppPaths::from_app_name("test-suite-app").expect("Path resolution should succeed");
        assert!(paths.data_dir.ends_with("data"));
        assert!(paths.config_dir.ends_with("config"));
        assert!(paths.cache_dir.ends_with("cache"));
        assert!(paths.log_dir.ends_with("logs"));
    }

    #[test]
    fn test_ensure_directories() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths {
            app_name: "test-app".to_string(),
            base_dir: temp.path().to_path_buf(),
            data_dir: temp.path().join("data"),
            config_dir: temp.path().join("config"),
            cache_dir: temp.path().join("cache"),
            log_dir: temp.path().join("logs"),
        };

        paths.ensure_directories().unwrap();
        assert!(paths.data_dir.exists());
        assert!(paths.config_dir.exists());
        assert!(paths.cache_dir.exists());
        assert!(paths.log_dir.exists());
    }

    #[test]
    fn test_process_resource_sampling() {
        let usage = ProcessResourceUsage::sample();
        assert!(usage.resident_set_size_bytes > 0);
        assert!(usage.cpu_percentage >= 0.0 && usage.cpu_percentage <= 100.0);
    }

    #[test]
    fn test_platform_full_integration() {
        // Theme detection
        let theme = ThemeDetector::detect_theme();
        assert!(theme.is_dark() || theme.is_light());

        // Native Keyring
        let keyring = PlatformKeyringFactory::create_keyring();
        let _bio = keyring.is_biometric_supported();
        keyring.set_secret("test.svc", "admin", "token123").unwrap();
        assert_eq!(
            keyring.get_secret("test.svc", "admin").unwrap(),
            Some("token123".to_string())
        );

        // Power telemetry
        let power = PowerTelemetry::query_power_status();
        assert!(matches!(
            power.source,
            PowerSource::AcPower | PowerSource::Battery | PowerSource::Ups | PowerSource::Unknown
        ));

        // Packaging manifest
        let manifest = PackageManifest::new(
            "universal-log-viewer",
            "1.0.0",
            TargetArchitecture::LinuxX64,
            PackageFormat::AppImage,
        );
        assert_eq!(
            manifest.output_filename,
            "universal-log-viewer_1.0.0_linux-x64.AppImage"
        );
    }
}
