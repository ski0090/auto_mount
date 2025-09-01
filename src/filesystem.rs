//! Filesystem formatting module for auto_mount
//!
//! This module handles filesystem creation with support for multiple filesystem types

use std::process::Command;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

/// Errors that can occur during filesystem operations
#[derive(Debug, thiserror::Error)]
pub enum FilesystemError {
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    IoError(std::io::Error),
    #[error("Unsupported filesystem type: {0}")]
    UnsupportedFilesystem(String),
    #[error("Invalid device path: {0}")]
    InvalidDevice(String),
    #[error("Filesystem formatting failed for device: {0}")]
    FormatFailed(String),
}

impl From<std::io::Error> for FilesystemError {
    fn from(error: std::io::Error) -> Self {
        FilesystemError::IoError(error)
    }
}

/// Supported filesystem types
#[derive(Debug, Clone, PartialEq, EnumString, Display, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum FilesystemType {
    Ext4,
    Ext3,
    Ext2,
    Xfs,
    Btrfs,
    Ntfs,
    #[strum(serialize = "fat32")]
    Fat32,
}

impl FilesystemType {
    /// Return the command and arguments for formatting
    fn format_command(&self) -> (&'static str, Vec<&'static str>) {
        match self {
            FilesystemType::Ext4 => ("mkfs.ext4", vec!["-F"]),
            FilesystemType::Ext3 => ("mkfs.ext3", vec!["-F"]),
            FilesystemType::Ext2 => ("mkfs.ext2", vec!["-F"]),
            FilesystemType::Xfs => ("mkfs.xfs", vec!["-f"]),
            FilesystemType::Btrfs => ("mkfs.btrfs", vec!["-f"]),
            FilesystemType::Ntfs => ("mkfs.ntfs", vec!["-f", "-Q"]),
            FilesystemType::Fat32 => ("mkfs.fat", vec!["-F", "32"]),
        }
    }

    /// Get all supported filesystem types
    pub fn supported_types() -> Vec<FilesystemType> {
        FilesystemType::iter().collect()
    }

    /// Get all supported filesystem type names
    pub fn supported_type_names() -> Vec<String> {
        FilesystemType::iter().map(|fs| fs.to_string()).collect()
    }

    /// Check if a filesystem type is supported
    pub fn is_supported(fs_type: &str) -> bool {
        fs_type.parse::<FilesystemType>().is_ok()
    }
}

/// Format result for a single device
#[derive(Debug, Clone)]
pub struct FormatResult {
    pub device: String,
    pub filesystem: FilesystemType,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Format devices with ext4 filesystem (backward compatibility)
pub fn format_devices(devices: &[String]) -> Result<(), FilesystemError> {
    format_devices_with_type(devices, FilesystemType::Ext4)
}

/// Format devices with specified filesystem type
pub fn format_devices_with_type(
    devices: &[String],
    filesystem: FilesystemType,
) -> Result<(), FilesystemError> {
    for device in devices {
        format_single_device(device, &filesystem)?;
    }
    Ok(())
}

/// Format a single device with specified filesystem
fn format_single_device(device: &str, filesystem: &FilesystemType) -> Result<(), FilesystemError> {
    validate_device_path(device)?;

    let (command_name, mut args) = filesystem.format_command();
    args.push(device);

    let output = Command::new("sudo")
        .arg(command_name)
        .args(&args)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(FilesystemError::FormatFailed(format!(
            "Device: {}, Error: {}",
            device, stderr
        )));
    }

    Ok(())
}

/// Validate device path
fn validate_device_path(device: &str) -> Result<(), FilesystemError> {
    if !device.starts_with("/dev/") {
        return Err(FilesystemError::InvalidDevice(device.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_type_from_str() {
        use std::str::FromStr;

        assert_eq!(
            "ext4".parse::<FilesystemType>().unwrap(),
            FilesystemType::Ext4
        );
        assert_eq!(
            "EXT4".parse::<FilesystemType>().unwrap(),
            FilesystemType::Ext4
        );
        assert_eq!(
            "xfs".parse::<FilesystemType>().unwrap(),
            FilesystemType::Xfs
        );
        assert_eq!(
            "fat32".parse::<FilesystemType>().unwrap(),
            FilesystemType::Fat32
        );
        assert!(FilesystemType::from_str("invalid").is_err());
    }

    #[test]
    fn test_filesystem_commands() {
        let (cmd, args) = FilesystemType::Ext4.format_command();
        assert_eq!(cmd, "mkfs.ext4");
        assert_eq!(args, vec!["-F"]);

        let (cmd, args) = FilesystemType::Xfs.format_command();
        assert_eq!(cmd, "mkfs.xfs");
        assert_eq!(args, vec!["-f"]);
    }

    #[test]
    fn test_filesystem_display() {
        assert_eq!(FilesystemType::Ext4.to_string(), "ext4");
        assert_eq!(FilesystemType::Xfs.to_string(), "xfs");
        assert_eq!(FilesystemType::Fat32.to_string(), "fat32");
        assert_eq!(FilesystemType::Btrfs.to_string(), "btrfs");
    }

    #[test]
    fn test_supported_types() {
        let supported = FilesystemType::supported_types();
        assert!(supported.contains(&FilesystemType::Ext4));
        assert!(supported.contains(&FilesystemType::Xfs));
        assert!(supported.contains(&FilesystemType::Fat32));
        assert_eq!(supported.len(), 7); // All 7 filesystem types
    }

    #[test]
    fn test_supported_type_names() {
        let names = FilesystemType::supported_type_names();
        assert!(names.contains(&"ext4".to_string()));
        assert!(names.contains(&"xfs".to_string()));
        assert!(names.contains(&"fat32".to_string()));
        assert_eq!(names.len(), 7);
    }

    #[test]
    fn test_is_supported() {
        assert!(FilesystemType::is_supported("ext4"));
        assert!(FilesystemType::is_supported("xfs"));
        assert!(FilesystemType::is_supported("fat32"));
        assert!(!FilesystemType::is_supported("invalid"));
        assert!(!FilesystemType::is_supported("zfs")); // Not supported yet
    }

    #[test]
    fn test_validate_device_path() {
        assert!(validate_device_path("/dev/sda1").is_ok());
        assert!(validate_device_path("sda1").is_err());
        assert!(validate_device_path("/home/user").is_err());
    }

    #[test]
    fn test_format_result_creation() {
        let result = FormatResult {
            device: "/dev/sda1".to_string(),
            filesystem: FilesystemType::Ext4,
            success: true,
            error_message: None,
        };

        assert_eq!(result.device, "/dev/sda1");
        assert_eq!(result.filesystem, FilesystemType::Ext4);
        assert!(result.success);
        assert!(result.error_message.is_none());
    }
}
