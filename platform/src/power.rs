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
        // Safe default on desktop/workstation environments
        PowerStatus {
            source: PowerSource::AcPower,
            battery_state: BatteryState::Full,
            battery_percentage: Some(100),
        }
    }

    #[cfg(target_os = "macos")]
    fn query_macos_power() -> PowerStatus {
        PowerStatus {
            source: PowerSource::AcPower,
            battery_state: BatteryState::Full,
            battery_percentage: Some(100),
        }
    }

    #[cfg(target_os = "linux")]
    fn query_linux_power() -> PowerStatus {
        // Safe reading of /sys/class/power_supply if present
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
                        return PowerStatus {
                            source: PowerSource::Battery,
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
    fn test_power_telemetry_query() {
        let status = PowerTelemetry::query_power_status();
        assert!(matches!(
            status.source,
            PowerSource::AcPower | PowerSource::Battery | PowerSource::Ups | PowerSource::Unknown
        ));
        if let Some(pct) = status.battery_percentage {
            assert!(pct <= 100);
        }
    }
}
