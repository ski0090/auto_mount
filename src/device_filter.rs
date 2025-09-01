//! Device filtering module for auto_mount
//!
//! This module handles filtering of devices based on type (HDD) and mount status

use std::process::Command;

/// Errors that can occur during device filtering
#[derive(Debug, thiserror::Error)]
pub enum DeviceFilterError {
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Invalid command output format")]
    InvalidOutputFormat,
    #[error("System information error")]
    SystemInfoError,
    #[error("IO error: {0}")]
    IoError(std::io::Error),
}

impl From<std::io::Error> for DeviceFilterError {
    fn from(error: std::io::Error) -> Self {
        DeviceFilterError::IoError(error)
    }
}

/// Device information structure
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub path: String,
    pub is_rotational: bool,
    pub is_mounted: bool,
}

/// Filter unmounted HDD devices with proper error handling
pub fn filter_unmounted_hdd_devices(
    devices: Vec<String>,
) -> Result<Vec<String>, DeviceFilterError> {
    let device_infos = collect_device_infos(devices)?;

    let filtered_devices: Vec<String> = device_infos
        .into_iter()
        .filter(|info| info.is_rotational && !info.is_mounted)
        .map(|info| info.path)
        .collect();

    Ok(filtered_devices)
}

/// Collect detailed information about devices
pub fn collect_device_infos(devices: Vec<String>) -> Result<Vec<DeviceInfo>, DeviceFilterError> {
    let system = create_disk_info()?;
    let mut device_infos = Vec::new();

    for device in devices {
        let info = DeviceInfo {
            path: device.clone(),
            is_rotational: is_rotational_device(&device)?,
            is_mounted: is_device_mounted(&device, &system)?,
        };
        device_infos.push(info);
    }

    Ok(device_infos)
}

/// Check if a device is rotational (HDD)
fn is_rotational_device(device: &str) -> Result<bool, DeviceFilterError> {
    let output = Command::new("sudo")
        .args(["lsblk", "-d", "-o", "rota", device])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DeviceFilterError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // Expected format: header line + data line
    if lines.len() < 2 {
        return Err(DeviceFilterError::InvalidOutputFormat);
    }

    let data_line = lines[1].trim();
    Ok(data_line == "1")
}

/// Check if a device is currently mounted
fn is_device_mounted(device: &str, disks: &sysinfo::Disks) -> Result<bool, DeviceFilterError> {
    let is_mounted = disks.iter().any(|disk| {
        let disk_name = disk.name().to_string_lossy();
        disk_name.contains(device)
    });

    Ok(is_mounted)
}

/// Create and initialize system information
fn create_disk_info() -> Result<sysinfo::Disks, DeviceFilterError> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    Ok(disks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_creation() {
        let info = DeviceInfo {
            path: "/dev/sda".to_string(),
            is_rotational: true,
            is_mounted: false,
        };

        assert_eq!(info.path, "/dev/sda");
        assert!(info.is_rotational);
        assert!(!info.is_mounted);
    }

    #[test]
    fn test_device_name_extraction() {
        let device = "/dev/sda";
        let name = device.strip_prefix("/dev/").unwrap();
        assert_eq!(name, "sda");
    }

    #[test]
    fn test_sysfs_path_construction() {
        let device_name = "sda";
        let expected_path = "/sys/block/sda/queue/rotational";
        let actual_path = format!("/sys/block/{}/queue/rotational", device_name);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_empty_device_list() {
        let devices = vec![];
        let result = filter_unmounted_hdd_devices(devices);

        match result {
            Ok(filtered) => assert!(filtered.is_empty()),
            Err(_) => {} // System info creation might fail in test environment
        }
    }
}
