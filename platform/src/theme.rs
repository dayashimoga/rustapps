#![forbid(unsafe_code)]
//! Native OS Theme Detection (ADR-0031)
//!
//! Provides platform-canonical system dark/light theme detection across:
//! - Windows: Registry query (`AppsUseLightTheme` and accessibility HighContrast)
//! - macOS: Interface style (`AppleInterfaceStyle`)
//! - Linux: Freedesktop portal and XDG desktop environment settings

use serde::{Deserialize, Serialize};

/// System appearance theme preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePreference {
    /// Dark appearance mode
    Dark,
    /// Light appearance mode
    Light,
    /// High contrast accessibility mode
    HighContrast,
    /// System default or indeterminate
    SystemDefault,
}

impl ThemePreference {
    /// Returns true if the theme is dark
    pub fn is_dark(&self) -> bool {
        matches!(self, ThemePreference::Dark)
    }

    /// Returns true if the theme is light
    pub fn is_light(&self) -> bool {
        matches!(self, ThemePreference::Light)
    }

    /// Returns true if high contrast mode is enabled
    pub fn is_high_contrast(&self) -> bool {
        matches!(self, ThemePreference::HighContrast)
    }
}

/// Native OS theme detection engine
pub struct ThemeDetector;

impl ThemeDetector {
    /// Detects current system theme preference for the running operating system
    pub fn detect_theme() -> ThemePreference {
        if let Ok(reg_theme) = std::env::var("FORCE_APP_THEME") {
            return match reg_theme.to_lowercase().as_str() {
                "light" => ThemePreference::Light,
                "dark" => ThemePreference::Dark,
                "contrast" | "highcontrast" => ThemePreference::HighContrast,
                _ => ThemePreference::SystemDefault,
            };
        }

        #[cfg(target_os = "windows")]
        {
            Self::detect_windows_theme()
        }
        #[cfg(target_os = "macos")]
        {
            Self::detect_macos_theme()
        }
        #[cfg(target_os = "linux")]
        {
            Self::detect_linux_theme()
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            ThemePreference::Dark
        }
    }

    #[cfg(target_os = "windows")]
    fn detect_windows_theme() -> ThemePreference {
        // 1. Check high contrast accessibility setting
        if let Ok(out) = std::process::Command::new("reg")
            .args([
                "query",
                "HKCU\\Control Panel\\Accessibility\\HighContrast",
                "/v",
                "Flags",
            ])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            // Flags value & 0x1 indicates high contrast active
            if let Some(pos) = text.find("0x") {
                if let Ok(val) = u32::from_str_radix(&text[pos + 2..pos + 3], 16) {
                    if (val & 0x1) == 0x1 {
                        return ThemePreference::HighContrast;
                    }
                }
            }
        }

        // 2. Query Windows personalization AppsUseLightTheme registry setting
        let output = std::process::Command::new("reg")
            .args([
                "query",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                "/v",
                "AppsUseLightTheme",
            ])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            if text.contains("0x1") {
                return ThemePreference::Light;
            } else if text.contains("0x0") {
                return ThemePreference::Dark;
            }
        }

        // Default if registry is unreadable (e.g. Server Core)
        ThemePreference::Dark
    }

    #[cfg(target_os = "macos")]
    fn detect_macos_theme() -> ThemePreference {
        if let Ok(out) = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if text.trim().eq_ignore_ascii_case("Dark") {
                return ThemePreference::Dark;
            } else {
                return ThemePreference::Light;
            }
        }
        ThemePreference::Light
    }

    #[cfg(target_os = "linux")]
    fn detect_linux_theme() -> ThemePreference {
        // 1. Try GNOME desktop portal setting
        if let Ok(out) = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if text.contains("dark") {
                return ThemePreference::Dark;
            } else if text.contains("light") || text.contains("default") {
                return ThemePreference::Light;
            }
        }

        // 2. Fall back to GTK_THEME environment hint
        if let Ok(gtk_theme) = std::env::var("GTK_THEME") {
            let lower = gtk_theme.to_lowercase();
            if lower.contains("dark") {
                return ThemePreference::Dark;
            } else if lower.contains("light") {
                return ThemePreference::Light;
            }
        }

        ThemePreference::Dark
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_detection_live() {
        let theme = ThemeDetector::detect_theme();
        assert!(matches!(
            theme,
            ThemePreference::Dark
                | ThemePreference::Light
                | ThemePreference::HighContrast
                | ThemePreference::SystemDefault
        ));
    }

    #[test]
    fn test_theme_force_override() {
        std::env::set_var("FORCE_APP_THEME", "light");
        assert_eq!(ThemeDetector::detect_theme(), ThemePreference::Light);

        std::env::set_var("FORCE_APP_THEME", "dark");
        assert_eq!(ThemeDetector::detect_theme(), ThemePreference::Dark);

        std::env::set_var("FORCE_APP_THEME", "contrast");
        assert_eq!(ThemeDetector::detect_theme(), ThemePreference::HighContrast);

        std::env::remove_var("FORCE_APP_THEME");
    }
}
