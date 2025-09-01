//! Device discovery module for auto_mount
//!
//! This module handles the discovery of connected SATA devices with proper error handling

use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

/// Errors that can occur during device discovery
#[derive(Debug, thiserror::Error)]
pub enum DeviceDiscoveryError {
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Invalid UTF-8 in command output")]
    InvalidUtf8,
    #[error("IO error: {0}")]
    IoError(io::Error),
    #[error("/dev directory not found")]
    DevDirectoryNotFound,
    #[error("No SATA devices found")]
    NoDevicesFound,
}

impl From<io::Error> for DeviceDiscoveryError {
    fn from(error: io::Error) -> Self {
        DeviceDiscoveryError::IoError(error)
    }
}

/// Find connected SATA devices with robust error handling
pub fn find_connected_satas() -> Result<Vec<String>, DeviceDiscoveryError> {
    // Check if /dev directory exists
    if !Path::new("/dev").exists() {
        return Err(DeviceDiscoveryError::DevDirectoryNotFound);
    }

    // Try primary method first (using /sys/block)
    match find_devices_via_sysblock() {
        Ok(devices) if !devices.is_empty() => Ok(devices),
        Ok(_) => find_devices_via_find_command(),
        Err(_) => find_devices_via_find_command(),
    }
}

/// Find SATA devices using /sys/block directory (preferred method)
fn find_devices_via_sysblock() -> Result<Vec<String>, DeviceDiscoveryError> {
    let mut devices = Vec::new();

    let entries = fs::read_dir("/sys/block")?;

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Check for SATA devices (sd + single letter: sda, sdb, etc.)
        if name_str.starts_with("sd") && name_str.len() == 3 {
            let device_path = format!("/dev/{}", name_str);
            if Path::new(&device_path).exists() {
                devices.push(device_path);
            }
        }
    }

    devices.sort();

    if devices.is_empty() {
        Err(DeviceDiscoveryError::NoDevicesFound)
    } else {
        Ok(devices)
    }
}

/// Find SATA devices using find command (fallback method)
fn find_devices_via_find_command() -> Result<Vec<String>, DeviceDiscoveryError> {
    // Try without sudo first
    match try_find_without_sudo() {
        Ok(devices) => Ok(devices),
        Err(_) => try_find_with_sudo(),
    }
}

/// Try to find devices without sudo privileges
fn try_find_without_sudo() -> Result<Vec<String>, DeviceDiscoveryError> {
    let output = Command::new("find")
        .args(["/dev", "-name", "sd?"])
        .output()?;

    process_find_output(output)
}

/// Try to find devices with sudo privileges
fn try_find_with_sudo() -> Result<Vec<String>, DeviceDiscoveryError> {
    let output = Command::new("sudo")
        .args(["find", "/dev", "-name", "sd?"])
        .output()?;

    process_find_output(output)
}

/// Process the output from find command
fn process_find_output(output: Output) -> Result<Vec<String>, DeviceDiscoveryError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DeviceDiscoveryError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8(output.stdout).map_err(|_| DeviceDiscoveryError::InvalidUtf8)?;

    let mut devices: Vec<String> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    devices.sort();

    if devices.is_empty() {
        Err(DeviceDiscoveryError::NoDevicesFound)
    } else {
        Ok(devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_name_validation() {
        // Unit test for device name validation logic
        let valid_names = vec!["sda", "sdb", "sdz"];
        let invalid_names = vec!["sd", "sda1", "nvme0n1", "loop0", "sdaa"];

        for name in valid_names {
            assert!(name.starts_with("sd") && name.len() == 3);
        }

        for name in invalid_names {
            assert!(!(name.starts_with("sd") && name.len() == 3));
        }
    }

    #[test]
    fn test_process_find_output_success() {
        use std::process::Command;

        // Create a successful command output for testing
        let output = Command::new("echo")
            .arg("/dev/sda\n/dev/sdb")
            .output()
            .unwrap();

        if output.status.success() {
            let result = process_find_output(output).unwrap();
            assert_eq!(result.len(), 2);
            assert!(result.contains(&"/dev/sda".to_string()));
            assert!(result.contains(&"/dev/sdb".to_string()));
        }
    }

    #[test]
    fn test_process_find_output_empty() {
        use std::process::Command;

        // Create an empty output for testing
        let output = Command::new("echo").arg("").output().unwrap();

        if output.status.success() {
            let result = process_find_output(output);
            assert!(matches!(result, Err(DeviceDiscoveryError::NoDevicesFound)));
        }
    }
}
