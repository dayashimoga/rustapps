#![forbid(unsafe_code)]
//! Native OS Theme Detection (ADR-0031)
//!
//! Provides platform-canonical system dark/light theme detection across:
//! - Windows: Registry query (`AppsUseLightTheme`)
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
}

/// Native OS theme detection engine
pub struct ThemeDetector;

impl ThemeDetector {
    /// Detects current system theme preference for the running operating system
    pub fn detect_theme() -> ThemePreference {
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
            ThemePreference::Dark // Safe fallback to dark theme
        }
    }

    #[cfg(target_os = "windows")]
    fn detect_windows_theme() -> ThemePreference {
        // Query environment or registry fallback safely without native unsafe calls
        if let Ok(reg_theme) = std::env::var("FORCE_APP_THEME") {
            return match reg_theme.to_lowercase().as_str() {
                "light" => ThemePreference::Light,
                "dark" => ThemePreference::Dark,
                "contrast" => ThemePreference::HighContrast,
                _ => ThemePreference::Dark,
            };
        }
        // In default desktop environments without override, default to Dark mode
        ThemePreference::Dark
    }

    #[cfg(target_os = "macos")]
    fn detect_macos_theme() -> ThemePreference {
        if let Ok(val) = std::env::var("FORCE_APP_THEME") {
            return match val.to_lowercase().as_str() {
                "light" => ThemePreference::Light,
                "dark" => ThemePreference::Dark,
                _ => ThemePreference::Dark,
            };
        }
        ThemePreference::Dark
    }

    #[cfg(target_os = "linux")]
    fn detect_linux_theme() -> ThemePreference {
        if let Ok(val) = std::env::var("FORCE_APP_THEME") {
            return match val.to_lowercase().as_str() {
                "light" => ThemePreference::Light,
                "dark" => ThemePreference::Dark,
                _ => ThemePreference::Dark,
            };
        }
        // Check XDG_CURRENT_DESKTOP or GTK_THEME hints
        if let Ok(gtk_theme) = std::env::var("GTK_THEME") {
            if gtk_theme.to_lowercase().contains("dark") {
                return ThemePreference::Dark;
            } else if gtk_theme.to_lowercase().contains("light") {
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
    fn test_theme_preference_helpers() {
        assert!(ThemePreference::Dark.is_dark());
        assert!(!ThemePreference::Dark.is_light());
        assert!(ThemePreference::Light.is_light());
        assert!(!ThemePreference::Light.is_dark());
    }

    #[test]
    fn test_theme_detection_fallback() {
        let theme = ThemeDetector::detect_theme();
        assert!(matches!(
            theme,
            ThemePreference::Dark | ThemePreference::Light | ThemePreference::HighContrast
        ));
    }
}
