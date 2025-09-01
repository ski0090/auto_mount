//! Partition management module for auto_mount
//!
//! This module handles partition creation with proper error handling and modern tools

use std::io::Write;
use std::process::{Command, Stdio};

/// Errors that can occur during partition operations
#[derive(Debug, thiserror::Error)]
pub enum PartitionError {
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    IoError(std::io::Error),
    #[error("Process spawn failed")]
    ProcessSpawnFailed,
    #[error("Invalid device path: {0}")]
    InvalidDevicePath(String),
    #[error("Partition creation failed for device: {0}")]
    PartitionCreationFailed(String),
}

impl From<std::io::Error> for PartitionError {
    fn from(error: std::io::Error) -> Self {
        PartitionError::IoError(error)
    }
}

/// Partition creation result
#[derive(Debug, Clone)]
pub struct PartitionResult {
    pub original_device: String,
    pub partition_path: String,
    pub success: bool,
}

/// Create single partition on each device using modern parted command
pub fn create_partition(devices: &[String]) -> Result<Vec<String>, PartitionError> {
    let mut partition_paths = Vec::new();

    for device in devices {
        let partition_path = create_single_partition_parted(device)?;
        partition_paths.push(partition_path);
    }

    Ok(partition_paths)
}

/// Create partition using parted (recommended approach)
fn create_single_partition_parted(device: &str) -> Result<String, PartitionError> {
    validate_device_path(device)?;

    // Create partition using parted (more reliable than fdisk)
    let output = Command::new("sudo")
        .args([
            "parted", "-s", // script mode (non-interactive)
            device, "mkpart",  // make partition
            "primary", // partition type
            "ext4",    // filesystem type hint
            "0%",      // start at beginning
            "100%",    // use entire disk
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PartitionError::CommandFailed(stderr.to_string()));
    }

    // Return the first partition path
    Ok(format!("{}1", device))
}

/// Create partition using fdisk (fallback method)
#[allow(dead_code)]
fn create_single_partition_fdisk(device: &str) -> Result<String, PartitionError> {
    validate_device_path(device)?;

    // Prepare fdisk commands
    let fdisk_commands = "n\np\n1\n\n\nw\n";

    let mut fdisk_process = Command::new("sudo")
        .args(["fdisk", device])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| PartitionError::ProcessSpawnFailed)?;

    // Send commands to fdisk
    if let Some(stdin) = fdisk_process.stdin.as_mut() {
        stdin.write_all(fdisk_commands.as_bytes())?;
    }

    let output = fdisk_process.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PartitionError::CommandFailed(stderr.to_string()));
    }

    Ok(format!("{}1", device))
}

/// Validate device path format
fn validate_device_path(device: &str) -> Result<(), PartitionError> {
    if !device.starts_with("/dev/") {
        return Err(PartitionError::InvalidDevicePath(device.to_string()));
    }

    // Additional validation for SATA devices
    if device.starts_with("/dev/sd") && device.len() == 8 {
        // Valid SATA device format like /dev/sda
        Ok(())
    } else {
        Err(PartitionError::InvalidDevicePath(device.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Result of GPT conversion operation
    #[derive(Debug, Clone)]
    pub struct GptConversionResult {
        pub device: String,
        pub success: bool,
        pub error_message: Option<String>,
    }

    #[test]
    fn test_validate_device_path_valid() {
        assert!(validate_device_path("/dev/sda").is_ok());
        assert!(validate_device_path("/dev/sdb").is_ok());
        assert!(validate_device_path("/dev/sdz").is_ok());
    }

    #[test]
    fn test_validate_device_path_invalid() {
        assert!(validate_device_path("sda").is_err());
        assert!(validate_device_path("/dev/").is_err());
        assert!(validate_device_path("/dev/nvme0n1").is_err());
        assert!(validate_device_path("/dev/sda1").is_err());
    }

    #[test]
    fn test_partition_result_creation() {
        let result = PartitionResult {
            original_device: "/dev/sda".to_string(),
            partition_path: "/dev/sda1".to_string(),
            success: true,
        };

        assert_eq!(result.original_device, "/dev/sda");
        assert_eq!(result.partition_path, "/dev/sda1");
        assert!(result.success);
    }

    #[test]
    fn test_partition_path_generation() {
        let device = "/dev/sda";
        let expected = "/dev/sda1";
        let actual = format!("{}1", device);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_gpt_conversion_result() {
        let result = GptConversionResult {
            device: "/dev/sda".to_string(),
            success: true,
            error_message: None,
        };

        assert_eq!(result.device, "/dev/sda");
        assert!(result.success);
        assert!(result.error_message.is_none());
    }
}

/// Convert devices to GPT partition table (supports devices larger than 4TB)
pub fn change_devices_to_gpt(devices: &[String]) -> Result<(), PartitionError> {
    for device in devices {
        change_single_device_to_gpt(device)?;
    }
    Ok(())
}

/// Convert a single device to GPT partition table
fn change_single_device_to_gpt(device: &str) -> Result<(), PartitionError> {
    validate_device_path(device)?;

    let output = Command::new("sudo")
        .args(["parted", "-s", device, "mklabel", "gpt"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PartitionError::CommandFailed(stderr.to_string()));
    }

    Ok(())
}
