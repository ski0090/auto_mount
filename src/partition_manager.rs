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

/// Create partitions with detailed results
#[allow(dead_code)]
pub fn create_partitions_with_results(devices: &[String]) -> Vec<PartitionResult> {
    devices
        .iter()
        .map(|device| match create_single_partition_parted(device) {
            Ok(partition_path) => PartitionResult {
                original_device: device.clone(),
                partition_path,
                success: true,
            },
            Err(_) => PartitionResult {
                original_device: device.clone(),
                partition_path: String::new(),
                success: false,
            },
        })
        .collect()
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

/// Get partition information for a device
#[allow(dead_code)]
pub fn get_partition_info(device: &str) -> Result<Vec<String>, PartitionError> {
    let output = Command::new("sudo")
        .args(["lsblk", "-ln", "-o", "NAME", device])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PartitionError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let partitions: Vec<String> = stdout
        .lines()
        .skip(1) // Skip the device itself, only get partitions
        .map(|line| format!("/dev/{}", line.trim()))
        .collect();

    Ok(partitions)
}

/// Check if device already has partitions
#[allow(dead_code)]
pub fn has_partitions(device: &str) -> Result<bool, PartitionError> {
    let partitions = get_partition_info(device)?;
    Ok(!partitions.is_empty())
}

/// Enhanced partition creation with pre-checks
#[allow(dead_code)]
pub fn create_partition_safe(devices: &[String]) -> Result<Vec<String>, PartitionError> {
    let mut results = Vec::new();

    for device in devices {
        // Check if device already has partitions
        if has_partitions(device)? {
            // If partitions exist, return the first one
            let existing_partitions = get_partition_info(device)?;
            if let Some(first_partition) = existing_partitions.first() {
                results.push(first_partition.clone());
                continue;
            }
        }

        // Create new partition
        let partition_path = create_single_partition_parted(device)?;
        results.push(partition_path);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

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

/// Result of GPT conversion operation
#[derive(Debug, Clone)]
pub struct GptConversionResult {
    pub device: String,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Convert devices to GPT with detailed results
#[allow(dead_code)]
pub fn change_devices_to_gpt_with_results(devices: &[String]) -> Vec<GptConversionResult> {
    devices
        .iter()
        .map(|device| match change_single_device_to_gpt(device) {
            Ok(()) => GptConversionResult {
                device: device.clone(),
                success: true,
                error_message: None,
            },
            Err(e) => GptConversionResult {
                device: device.clone(),
                success: false,
                error_message: Some(e.to_string()),
            },
        })
        .collect()
}

/// Check if device is already using GPT
#[allow(dead_code)]
pub fn is_device_gpt(device: &str) -> Result<bool, PartitionError> {
    let output = Command::new("sudo")
        .args(["parted", "-s", device, "print"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PartitionError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains("Partition Table: gpt"))
}

/// Convert devices to GPT with safety checks (skip if already GPT)
#[allow(dead_code)]
pub fn change_devices_to_gpt_safe(
    devices: &[String],
) -> Result<Vec<GptConversionResult>, PartitionError> {
    let mut results = Vec::new();

    for device in devices {
        // Check if already GPT
        match is_device_gpt(device) {
            Ok(true) => {
                // Already GPT, skip
                results.push(GptConversionResult {
                    device: device.clone(),
                    success: true,
                    error_message: Some("Device already uses GPT, skipped".to_string()),
                });
            }
            Ok(false) => {
                // Convert to GPT
                match change_single_device_to_gpt(device) {
                    Ok(()) => results.push(GptConversionResult {
                        device: device.clone(),
                        success: true,
                        error_message: None,
                    }),
                    Err(e) => results.push(GptConversionResult {
                        device: device.clone(),
                        success: false,
                        error_message: Some(e.to_string()),
                    }),
                }
            }
            Err(e) => {
                // Error checking GPT status
                results.push(GptConversionResult {
                    device: device.clone(),
                    success: false,
                    error_message: Some(format!("Failed to check GPT status: {}", e)),
                });
            }
        }
    }

    Ok(results)
}
