//! Filesystem formatting module for auto_mount
//!
//! This module handles filesystem creation with support for multiple filesystem types

use std::process::Command;

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
#[derive(Debug, Clone, PartialEq)]
pub enum FilesystemType {
    Ext4,
    Ext3,
    Ext2,
    Xfs,
    Btrfs,
    Ntfs,
    Fat32,
}

impl FilesystemType {
    /// Get the command and arguments for formatting
    fn get_format_command(&self) -> (&'static str, Vec<&'static str>) {
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

    /// Get filesystem type from string
    pub fn from_str(fs_type: &str) -> Result<Self, FilesystemError> {
        match fs_type.to_lowercase().as_str() {
            "ext4" => Ok(FilesystemType::Ext4),
            "ext3" => Ok(FilesystemType::Ext3),
            "ext2" => Ok(FilesystemType::Ext2),
            "xfs" => Ok(FilesystemType::Xfs),
            "btrfs" => Ok(FilesystemType::Btrfs),
            "ntfs" => Ok(FilesystemType::Ntfs),
            "fat32" => Ok(FilesystemType::Fat32),
            _ => Err(FilesystemError::UnsupportedFilesystem(fs_type.to_string())),
        }
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

/// Format devices with detailed results
#[allow(dead_code)]
pub fn format_devices_with_results(
    devices: &[String],
    filesystem: FilesystemType,
) -> Vec<FormatResult> {
    devices
        .iter()
        .map(|device| match format_single_device(device, &filesystem) {
            Ok(()) => FormatResult {
                device: device.clone(),
                filesystem: filesystem.clone(),
                success: true,
                error_message: None,
            },
            Err(e) => FormatResult {
                device: device.clone(),
                filesystem: filesystem.clone(),
                success: false,
                error_message: Some(e.to_string()),
            },
        })
        .collect()
}

/// Format a single device with specified filesystem
fn format_single_device(device: &str, filesystem: &FilesystemType) -> Result<(), FilesystemError> {
    validate_device_path(device)?;

    let (command_name, mut args) = filesystem.get_format_command();
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

/// Check if device is already formatted
#[allow(dead_code)]
pub fn is_device_formatted(device: &str) -> Result<bool, FilesystemError> {
    let output = Command::new("sudo").args(["blkid", device]).output()?;

    // blkid returns 0 if filesystem is detected, non-zero otherwise
    Ok(output.status.success())
}

/// Get filesystem type of a device
#[allow(dead_code)]
pub fn get_device_filesystem(device: &str) -> Result<Option<String>, FilesystemError> {
    let output = Command::new("sudo")
        .args(["blkid", "-s", "TYPE", "-o", "value", device])
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let filesystem = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if filesystem.is_empty() {
        Ok(None)
    } else {
        Ok(Some(filesystem))
    }
}

/// Format devices with safety checks
#[allow(dead_code)]
pub fn format_devices_safe(
    devices: &[String],
    filesystem: FilesystemType,
) -> Result<Vec<FormatResult>, FilesystemError> {
    let mut results = Vec::new();

    for device in devices {
        // Check if already formatted
        let is_formatted = is_device_formatted(device)?;

        if is_formatted {
            // Skip already formatted devices
            results.push(FormatResult {
                device: device.clone(),
                filesystem: filesystem.clone(),
                success: true,
                error_message: Some("Device already formatted, skipped".to_string()),
            });
            continue;
        }

        // Format the device
        match format_single_device(device, &filesystem) {
            Ok(()) => results.push(FormatResult {
                device: device.clone(),
                filesystem: filesystem.clone(),
                success: true,
                error_message: None,
            }),
            Err(e) => results.push(FormatResult {
                device: device.clone(),
                filesystem: filesystem.clone(),
                success: false,
                error_message: Some(e.to_string()),
            }),
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_type_from_str() {
        assert_eq!(
            FilesystemType::from_str("ext4").unwrap(),
            FilesystemType::Ext4
        );
        assert_eq!(
            FilesystemType::from_str("EXT4").unwrap(),
            FilesystemType::Ext4
        );
        assert_eq!(
            FilesystemType::from_str("xfs").unwrap(),
            FilesystemType::Xfs
        );
        assert!(FilesystemType::from_str("invalid").is_err());
    }

    #[test]
    fn test_filesystem_commands() {
        let (cmd, args) = FilesystemType::Ext4.get_format_command();
        assert_eq!(cmd, "mkfs.ext4");
        assert_eq!(args, vec!["-F"]);

        let (cmd, args) = FilesystemType::Xfs.get_format_command();
        assert_eq!(cmd, "mkfs.xfs");
        assert_eq!(args, vec!["-f"]);
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
