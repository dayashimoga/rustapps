#![forbid(unsafe_code)]
//! System Power & Battery Telemetry (ADR-0031)
//!
//! Provides platform-independent power status, battery state, and lifecycle monitoring.

use serde::{Deserialize, Serialize};

/// Power source supplying the machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerSource {
    AcPower,
    Battery,
    Ups,
    Unknown,
}

/// Battery charging and degradation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryState {
    Charging,
    Discharging,
    Full,
    NotPresent,
    Unknown,
}

/// Instantaneous system power status
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PowerStatus {
    pub source: PowerSource,
    pub battery_state: BatteryState,
    pub battery_percentage: Option<u8>,
}

/// Power telemetry query engine
pub struct PowerTelemetry;

impl PowerTelemetry {
    /// Queries the current power and battery metrics for the system
    pub fn query_power_status() -> PowerStatus {
        #[cfg(target_os = "windows")]
        {
            Self::query_windows_power()
        }
        #[cfg(target_os = "macos")]
        {
            Self::query_macos_power()
        }
        #[cfg(target_os = "linux")]
        {
            Self::query_linux_power()
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            PowerStatus {
                source: PowerSource::AcPower,
                battery_state: BatteryState::NotPresent,
                battery_percentage: None,
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn query_windows_power() -> PowerStatus {
        // 1. Fast query via SystemInformation PowerStatus
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "try { Add-Type -AssemblyName System.Windows.Forms; $p = [System.Windows.Forms.SystemInformation]::PowerStatus; [PSCustomObject]@{ Line = [int]$p.PowerLineStatus; Status = [int]$p.BatteryChargeStatus; Pct = [float]$p.BatteryLifePercent } | ConvertTo-Json -Compress } catch { 'ERR' }",
            ])
            .output();

        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            let trimmed = s.trim();
            if !trimmed.is_empty() && trimmed != "ERR" {
                #[derive(Deserialize)]
                struct WinPowerRaw {
                    #[serde(rename = "Line")]
                    line: i32,
                    #[serde(rename = "Status")]
                    status: i32,
                    #[serde(rename = "Pct")]
                    pct: f32,
                }

                if let Ok(raw) = serde_json::from_str::<WinPowerRaw>(trimmed) {
                    let source = match raw.line {
                        1 => PowerSource::AcPower,
                        0 => PowerSource::Battery,
                        _ => PowerSource::Unknown,
                    };

                    // BatteryChargeStatus flags:
                    // 1 = High, 2 = Low, 4 = Critical, 8 = Charging, 128 = NoSystemBattery
                    let (battery_state, battery_percentage) =
                        if (raw.status & 128) != 0 || raw.pct < 0.0 {
                            (BatteryState::NotPresent, None)
                        } else {
                            let pct_u8 = (raw.pct * 100.0).clamp(0.0, 100.0).round() as u8;
                            let state = if (raw.status & 8) != 0 {
                                BatteryState::Charging
                            } else if pct_u8 >= 99 {
                                BatteryState::Full
                            } else {
                                BatteryState::Discharging
                            };
                            (state, Some(pct_u8))
                        };

                    return PowerStatus {
                        source,
                        battery_state,
                        battery_percentage,
                    };
                }
            }
        }

        // 2. Fallback query via Win32_Battery CIM instance
        let cim_output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "try { $b = Get-CimInstance -ClassName Win32_Battery | Select-Object -First 1; if ($b) { [PSCustomObject]@{ Status = [int]$b.BatteryStatus; Pct = [int]$b.EstimatedChargeRemaining } | ConvertTo-Json -Compress } else { 'NO_BATTERY' } } catch { 'ERR' }",
            ])
            .output();

        if let Ok(out) = cim_output {
            let s = String::from_utf8_lossy(&out.stdout);
            let trimmed = s.trim();
            if trimmed == "NO_BATTERY" {
                return PowerStatus {
                    source: PowerSource::AcPower,
                    battery_state: BatteryState::NotPresent,
                    battery_percentage: None,
                };
            }

            #[derive(Deserialize)]
            struct CimBatteryRaw {
                #[serde(rename = "Status")]
                status: i32,
                #[serde(rename = "Pct")]
                pct: u8,
            }

            if let Ok(raw) = serde_json::from_str::<CimBatteryRaw>(trimmed) {
                let state = match raw.status {
                    6..=9 => BatteryState::Charging,
                    3 => BatteryState::Full,
                    1..=2 | 4..=5 => BatteryState::Discharging,
                    _ => BatteryState::Unknown,
                };
                let source = match state {
                    BatteryState::Charging => PowerSource::AcPower,
                    BatteryState::Full => PowerSource::AcPower,
                    _ => PowerSource::Battery,
                };
                return PowerStatus {
                    source,
                    battery_state: state,
                    battery_percentage: Some(raw.pct.min(100)),
                };
            }
        }

        // Safe fallback when power query cannot run
        PowerStatus {
            source: PowerSource::AcPower,
            battery_state: BatteryState::NotPresent,
            battery_percentage: None,
        }
    }

    #[cfg(target_os = "macos")]
    fn query_macos_power() -> PowerStatus {
        if let Ok(out) = std::process::Command::new("pmset")
            .args(["-g", "batt"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            let source = if text.contains("'AC Power'") {
                PowerSource::AcPower
            } else if text.contains("'Battery Power'") {
                PowerSource::Battery
            } else {
                PowerSource::Unknown
            };

            let battery_percentage = text.lines().find_map(|line| {
                if let Some(pct_idx) = line.find('%') {
                    let prefix = &line[..pct_idx];
                    let digits: String = prefix
                        .chars()
                        .rev()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let digits: String = digits.chars().rev().collect();
                    digits.parse::<u8>().ok()
                } else {
                    None
                }
            });

            let lower = text.to_lowercase();
            let battery_state = if !lower.contains("internalbattery") {
                BatteryState::NotPresent
            } else if lower.contains("charging") && !lower.contains("discharging") {
                BatteryState::Charging
            } else if lower.contains("discharging") {
                BatteryState::Discharging
            } else if lower.contains("charged") || battery_percentage == Some(100) {
                BatteryState::Full
            } else {
                BatteryState::Unknown
            };

            return PowerStatus {
                source,
                battery_state,
                battery_percentage,
            };
        }

        PowerStatus {
            source: PowerSource::AcPower,
            battery_state: BatteryState::NotPresent,
            battery_percentage: None,
        }
    }

    #[cfg(target_os = "linux")]
    fn query_linux_power() -> PowerStatus {
        if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(kind) = std::fs::read_to_string(path.join("type")) {
                    if kind.trim().eq_ignore_ascii_case("Battery") {
                        let capacity = std::fs::read_to_string(path.join("capacity"))
                            .ok()
                            .and_then(|c| c.trim().parse::<u8>().ok());
                        let status_str =
                            std::fs::read_to_string(path.join("status")).unwrap_or_default();
                        let state = match status_str.trim().to_lowercase().as_str() {
                            "charging" => BatteryState::Charging,
                            "discharging" => BatteryState::Discharging,
                            "full" => BatteryState::Full,
                            _ => BatteryState::Unknown,
                        };
                        let source = match state {
                            BatteryState::Charging | BatteryState::Full => PowerSource::AcPower,
                            _ => PowerSource::Battery,
                        };
                        return PowerStatus {
                            source,
                            battery_state: state,
                            battery_percentage: capacity,
                        };
                    }
                }
            }
        }

        PowerStatus {
            source: PowerSource::AcPower,
            battery_state: BatteryState::NotPresent,
            battery_percentage: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_telemetry_query_live() {
        let status = PowerTelemetry::query_power_status();
        assert!(matches!(
            status.source,
            PowerSource::AcPower | PowerSource::Battery | PowerSource::Ups | PowerSource::Unknown
        ));
        assert!(matches!(
            status.battery_state,
            BatteryState::Charging
                | BatteryState::Discharging
                | BatteryState::Full
                | BatteryState::NotPresent
                | BatteryState::Unknown
        ));
        if let Some(pct) = status.battery_percentage {
            assert!(pct <= 100);
            println!(
                "Live Power Status: Source={:?}, State={:?}, Level={}%",
                status.source, status.battery_state, pct
            );
        } else {
            println!(
                "Live Power Status: Source={:?}, State={:?}, Level=None",
                status.source, status.battery_state
            );
        }
    }
}
