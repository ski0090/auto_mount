//! Smart mounting module with intelligent decision making
//!
//! This module provides high-level functions that make intelligent decisions
//! about partition table types, filesystem choices, etc.

use crate::{
    change_devices_to_gpt, create_partition, filter_unmounted_hdd_devices, find_connected_satas,
    format_devices, mount_devices, DeviceDiscoveryError, DeviceFilterError, FilesystemError,
    PartitionError,
};

/// Errors that can occur during smart mounting
#[derive(Debug, thiserror::Error)]
pub enum SmartMountError {
    #[error("Device discovery failed: {0}")]
    DeviceDiscovery(#[from] DeviceDiscoveryError),
    #[error("Device filtering failed: {0}")]
    DeviceFilter(#[from] DeviceFilterError),
    #[error("Partition operation failed: {0}")]
    Partition(#[from] PartitionError),
    #[error("Filesystem operation failed: {0}")]
    Filesystem(#[from] FilesystemError),
    #[error("No devices found to process")]
    NoDevicesFound,
}

/// Configuration for smart mounting
#[derive(Debug, Clone)]
pub struct MountConfig {
    /// Force GPT even for small disks
    pub force_gpt: bool,
    /// Minimum disk size (in GB) to automatically use GPT
    pub gpt_threshold_gb: u64,
    /// Skip GPT conversion entirely
    pub skip_gpt: bool,
}

impl Default for MountConfig {
    fn default() -> Self {
        Self {
            force_gpt: false,
            gpt_threshold_gb: 2000, // 2TB threshold
            skip_gpt: false,
        }
    }
}

/// Smart auto-mount with intelligent decisions
pub fn smart_auto_mount() -> Result<(), SmartMountError> {
    smart_auto_mount_with_config(MountConfig::default())
}

/// Smart auto-mount with custom configuration
pub fn smart_auto_mount_with_config(config: MountConfig) -> Result<(), SmartMountError> {
    // Find and filter devices
    let devices = find_connected_satas()?;
    if devices.is_empty() {
        return Err(SmartMountError::NoDevicesFound);
    }

    let devices = filter_unmounted_hdd_devices(devices)?;
    if devices.is_empty() {
        return Err(SmartMountError::NoDevicesFound);
    }

    // Decide whether to use GPT
    if should_use_gpt(&devices, &config)? {
        change_devices_to_gpt(&devices)?;
    }

    // Create partitions, format, and mount
    let devices = create_partition(&devices)?;
    format_devices(&devices)?;
    mount_devices(&devices);

    Ok(())
}

/// Determine if GPT should be used based on device sizes and configuration
fn should_use_gpt(devices: &[String], config: &MountConfig) -> Result<bool, SmartMountError> {
    if config.skip_gpt {
        return Ok(false);
    }

    if config.force_gpt {
        return Ok(true);
    }

    // Check device sizes to determine if GPT is needed
    for device in devices {
        if let Ok(size_gb) = get_device_size_gb(device) {
            if size_gb >= config.gpt_threshold_gb {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Get device size in GB
fn get_device_size_gb(device: &str) -> Result<u64, SmartMountError> {
    use std::process::Command;

    let output = Command::new("sudo")
        .args(["blockdev", "--getsize64", device])
        .output()
        .map_err(|e| SmartMountError::Partition(PartitionError::IoError(e)))?;

    if !output.status.success() {
        return Ok(0); // Default to 0 if we can't determine size
    }

    let size_bytes = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .unwrap_or(0);

    Ok(size_bytes / (1024 * 1024 * 1024)) // Convert to GB
}

/// Simple auto-mount without GPT conversion (for compatibility)
pub fn simple_auto_mount() -> Result<(), SmartMountError> {
    let config = MountConfig {
        skip_gpt: true,
        ..Default::default()
    };
    smart_auto_mount_with_config(config)
}

/// Auto-mount with forced GPT (for large disks or modern systems)
pub fn gpt_auto_mount() -> Result<(), SmartMountError> {
    let config = MountConfig {
        force_gpt: true,
        ..Default::default()
    };
    smart_auto_mount_with_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_config_default() {
        let config = MountConfig::default();
        assert!(!config.force_gpt);
        assert_eq!(config.gpt_threshold_gb, 2000);
        assert!(!config.skip_gpt);
    }

    #[test]
    fn test_should_use_gpt_force() {
        let config = MountConfig {
            force_gpt: true,
            ..Default::default()
        };
        let devices = vec!["/dev/sda".to_string()];
        assert!(should_use_gpt(&devices, &config).unwrap());
    }

    #[test]
    fn test_should_use_gpt_skip() {
        let config = MountConfig {
            skip_gpt: true,
            ..Default::default()
        };
        let devices = vec!["/dev/sda".to_string()];
        assert!(!should_use_gpt(&devices, &config).unwrap());
    }
}
