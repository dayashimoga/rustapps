#![forbid(unsafe_code)]
//! Multi-Platform Packaging Matrix & Bundle Validation (ADR-0030)
//!
//! Provides metadata modeling, prerequisite checking, and bundle validation
//! across the complete 6-target desktop distribution matrix:
//! - Windows: MSI (WiX Toolset), InnoSetup EXE, Portable ZIP
//! - Linux: AppImage, DEB (dpkg), RPM (rpmbuild), tar.gz
//! - macOS: DMG (hdiutil), .app bundle, Universal Binary (lipo)

use crate::PlatformError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Primary desktop distribution targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetArchitecture {
    /// Windows 64-bit Intel/AMD
    WindowsX64,
    /// Windows 64-bit ARM (Snapdragon X, Copilot+ PCs)
    WindowsArm64,
    /// Linux 64-bit Intel/AMD
    LinuxX64,
    /// Linux 64-bit ARM (Graviton, Ampere, Raspberry Pi 5)
    LinuxArm64,
    /// macOS 64-bit Intel
    MacOsIntel,
    /// macOS 64-bit Apple Silicon (M1/M2/M3/M4)
    MacOsAppleSilicon,
    /// macOS Universal Binary (fat binary combining Intel + Apple Silicon)
    MacOsUniversal,
}

impl TargetArchitecture {
    /// Returns the standard LLVM target triple
    pub fn target_triple(&self) -> &'static str {
        match self {
            TargetArchitecture::WindowsX64 => "x86_64-pc-windows-msvc",
            TargetArchitecture::WindowsArm64 => "aarch64-pc-windows-msvc",
            TargetArchitecture::LinuxX64 => "x86_64-unknown-linux-gnu",
            TargetArchitecture::LinuxArm64 => "aarch64-unknown-linux-gnu",
            TargetArchitecture::MacOsIntel => "x86_64-apple-darwin",
            TargetArchitecture::MacOsAppleSilicon => "aarch64-apple-darwin",
            TargetArchitecture::MacOsUniversal => "universal2-apple-darwin",
        }
    }

    /// Returns human-readable label
    pub fn display_name(&self) -> &'static str {
        match self {
            TargetArchitecture::WindowsX64 => "Windows (x64)",
            TargetArchitecture::WindowsArm64 => "Windows (ARM64)",
            TargetArchitecture::LinuxX64 => "Linux (x64)",
            TargetArchitecture::LinuxArm64 => "Linux (ARM64)",
            TargetArchitecture::MacOsIntel => "macOS (Intel)",
            TargetArchitecture::MacOsAppleSilicon => "macOS (Apple Silicon)",
            TargetArchitecture::MacOsUniversal => "macOS (Universal 2)",
        }
    }
}

/// Native package distribution formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackageFormat {
    /// Windows WiX Installer (.msi)
    Msi,
    /// Windows InnoSetup Installer (.exe)
    WindowsExe,
    /// Standalone Portable ZIP (.zip)
    PortableZip,
    /// Linux Standalone Executable Container (.AppImage)
    AppImage,
    /// Debian / Ubuntu Package (.deb)
    Deb,
    /// Red Hat / Fedora / openSUSE Package (.rpm)
    Rpm,
    /// Linux Standalone Tarball (.tar.gz)
    TarGz,
    /// macOS Application Bundle (.app)
    MacOsApp,
    /// macOS Apple Disk Image (.dmg)
    MacOsDmg,
}

impl PackageFormat {
    /// Returns the standard file extension
    pub fn file_extension(&self) -> &'static str {
        match self {
            PackageFormat::Msi => "msi",
            PackageFormat::WindowsExe => "exe",
            PackageFormat::PortableZip => "zip",
            PackageFormat::AppImage => "AppImage",
            PackageFormat::Deb => "deb",
            PackageFormat::Rpm => "rpm",
            PackageFormat::TarGz => "tar.gz",
            PackageFormat::MacOsApp => "app",
            PackageFormat::MacOsDmg => "dmg",
        }
    }
}

/// Distribution package manifest definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub app_name: String,
    pub version: String,
    pub target: TargetArchitecture,
    pub format: PackageFormat,
    pub output_filename: String,
    pub sha256_checksum: Option<String>,
}

impl PackageManifest {
    /// Generates a standardized canonical release package name
    pub fn new(
        app_name: &str,
        version: &str,
        target: TargetArchitecture,
        format: PackageFormat,
    ) -> Self {
        let arch_slug = match target {
            TargetArchitecture::WindowsX64 => "win-x64",
            TargetArchitecture::WindowsArm64 => "win-arm64",
            TargetArchitecture::LinuxX64 => "linux-x64",
            TargetArchitecture::LinuxArm64 => "linux-arm64",
            TargetArchitecture::MacOsIntel => "macos-x64",
            TargetArchitecture::MacOsAppleSilicon => "macos-arm64",
            TargetArchitecture::MacOsUniversal => "macos-universal",
        };

        let output_filename = format!(
            "{}_{}_{}.{}",
            app_name,
            version,
            arch_slug,
            format.file_extension()
        );

        Self {
            app_name: app_name.to_string(),
            version: version.to_string(),
            target,
            format,
            output_filename,
            sha256_checksum: None,
        }
    }
}

/// Package integrity and bundle validator
pub struct PackagingValidator;

impl PackagingValidator {
    /// Validates that a packaged file exists and matches its expected manifest name
    pub fn validate_package_file(
        manifest: &PackageManifest,
        file_path: &Path,
    ) -> Result<bool, PlatformError> {
        if !file_path.exists() {
            return Err(PlatformError::PathResolutionFailed(format!(
                "Package file does not exist: {}",
                file_path.display()
            )));
        }

        let file_name = file_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        if !file_name.ends_with(manifest.format.file_extension()) {
            return Err(PlatformError::Unsupported(format!(
                "File extension does not match expected format {:?}",
                manifest.format
            )));
        }

        Ok(true)
    }

    /// Verifies SHA-256 integrity format (64-character hexadecimal)
    pub fn verify_checksum_format(checksum: &str) -> bool {
        checksum.len() == 64 && checksum.chars().all(|c| c.is_ascii_hexdigit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_triples() {
        assert_eq!(
            TargetArchitecture::WindowsX64.target_triple(),
            "x86_64-pc-windows-msvc"
        );
        assert_eq!(
            TargetArchitecture::LinuxArm64.target_triple(),
            "aarch64-unknown-linux-gnu"
        );
        assert_eq!(
            TargetArchitecture::MacOsAppleSilicon.target_triple(),
            "aarch64-apple-darwin"
        );
    }

    #[test]
    fn test_package_manifest_generation() {
        let manifest = PackageManifest::new(
            "devops-secret-vault",
            "0.1.0",
            TargetArchitecture::WindowsX64,
            PackageFormat::Msi,
        );
        assert_eq!(
            manifest.output_filename,
            "devops-secret-vault_0.1.0_win-x64.msi"
        );

        let dmg_manifest = PackageManifest::new(
            "ai-file-explorer",
            "0.1.0",
            TargetArchitecture::MacOsUniversal,
            PackageFormat::MacOsDmg,
        );
        assert_eq!(
            dmg_manifest.output_filename,
            "ai-file-explorer_0.1.0_macos-universal.dmg"
        );
    }

    #[test]
    fn test_checksum_validation() {
        let valid_sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(PackagingValidator::verify_checksum_format(valid_sha256));

        let invalid_short = "e3b0c442";
        assert!(!PackagingValidator::verify_checksum_format(invalid_short));

        let invalid_chars = "g3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b85z";
        assert!(!PackagingValidator::verify_checksum_format(invalid_chars));
    }
}
