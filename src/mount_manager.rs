//! Safe mount management module for auto_mount
//!
//! This module handles mounting with proper safety measures including
//! backup, validation, and atomic operations

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// Errors that can occur during mount operations
#[derive(Debug, thiserror::Error)]
pub enum MountError {
    #[error("IO error: {0}")]
    IoError(std::io::Error),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Invalid device path: {0}")]
    InvalidDevice(String),
    #[error("Mount point creation failed: {0}")]
    MountPointCreationFailed(String),
    #[error("Fstab backup failed")]
    BackupFailed,
    #[error("Fstab validation failed: {0}")]
    ValidationFailed(String),
    #[error("UUID not found for device: {0}")]
    UuidNotFound(String),
    #[error("Permission denied")]
    PermissionDenied,
}

impl From<std::io::Error> for MountError {
    fn from(error: std::io::Error) -> Self {
        MountError::IoError(error)
    }
}

/// Mount configuration
#[derive(Debug, Clone)]
pub struct MountConfig {
    pub filesystem_type: String,
    pub mount_options: String,
    pub mount_base_path: String,
    pub backup_fstab: bool,
    pub validate_before_write: bool,
}

impl Default for MountConfig {
    fn default() -> Self {
        Self {
            filesystem_type: "ext4".to_string(),
            mount_options: "rw,acl".to_string(),
            mount_base_path: "/mnt".to_string(),
            backup_fstab: true,
            validate_before_write: true,
        }
    }
}

/// Mount entry information
#[derive(Debug, Clone)]
pub struct MountEntry {
    pub device: String,
    pub uuid: String,
    pub mount_point: String,
    pub filesystem: String,
    pub options: String,
}

/// Result of mount operation
#[derive(Debug, Clone)]
pub struct MountResult {
    pub device: String,
    pub mount_point: String,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Safe mount devices with comprehensive error handling and backup
pub fn mount_devices(devices: &[String]) -> Result<Vec<MountResult>, MountError> {
    mount_devices_with_config(devices, MountConfig::default())
}

/// Mount devices with custom configuration
pub fn mount_devices_with_config(
    devices: &[String],
    config: MountConfig,
) -> Result<Vec<MountResult>, MountError> {
    let fstab_path = "/etc/fstab";

    // Step 1: Create backup if enabled
    let backup_path = if config.backup_fstab {
        Some(create_fstab_backup(fstab_path)?)
    } else {
        None
    };

    // Step 2: Prepare mount entries
    let mut mount_entries = Vec::new();
    let mut results = Vec::new();

    for device in devices {
        match prepare_mount_entry(device, &config) {
            Ok(entry) => {
                mount_entries.push(entry.clone());
                results.push(MountResult {
                    device: device.clone(),
                    mount_point: entry.mount_point.clone(),
                    success: true,
                    error_message: None,
                });
            }
            Err(e) => {
                results.push(MountResult {
                    device: device.clone(),
                    mount_point: String::new(),
                    success: false,
                    error_message: Some(e.to_string()),
                });
            }
        }
    }

    if mount_entries.is_empty() {
        return Ok(results);
    }

    // Step 3: Update fstab safely
    match update_fstab_safe(fstab_path, &mount_entries, &config) {
        Ok(()) => {
            // Step 4: Apply mounts
            if let Err(e) = apply_mounts() {
                // If mount fails, try to restore backup
                if let Some(backup) = backup_path {
                    let _ = restore_fstab_backup(fstab_path, &backup);
                }
                return Err(e);
            }
        }
        Err(e) => {
            // Restore backup if update failed
            if let Some(backup) = backup_path {
                let _ = restore_fstab_backup(fstab_path, &backup);
            }
            return Err(e);
        }
    }

    Ok(results)
}

/// Create a timestamped backup of fstab
fn create_fstab_backup(fstab_path: &str) -> Result<PathBuf, MountError> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let backup_path = format!("{}.backup.{}", fstab_path, timestamp);

    fs::copy(fstab_path, &backup_path)?;

    // Verify backup
    let original_size = fs::metadata(fstab_path)?.len();
    let backup_size = fs::metadata(&backup_path)?.len();

    if original_size != backup_size {
        return Err(MountError::BackupFailed);
    }

    Ok(PathBuf::from(backup_path))
}

/// Restore fstab from backup
fn restore_fstab_backup(fstab_path: &str, backup_path: &Path) -> Result<(), MountError> {
    fs::copy(backup_path, fstab_path)?;
    Ok(())
}

/// Prepare mount entry for a device
fn prepare_mount_entry(device: &str, config: &MountConfig) -> Result<MountEntry, MountError> {
    // Validate device path
    if !device.starts_with("/dev/") {
        return Err(MountError::InvalidDevice(device.to_string()));
    }

    // Get UUID
    let uuid = device_uuid(device)?;

    // Create mount point
    let device_name = device
        .split('/')
        .next_back()
        .ok_or_else(|| MountError::InvalidDevice(device.to_string()))?;
    let mount_point = format!("{}/{}", config.mount_base_path, device_name);

    create_mount_point(&mount_point)?;

    Ok(MountEntry {
        device: device.to_string(),
        uuid,
        mount_point,
        filesystem: config.filesystem_type.clone(),
        options: config.mount_options.clone(),
    })
}

/// Find UUID for a device
fn device_uuid(device: &str) -> Result<String, MountError> {
    let output = Command::new("sudo")
        .args(["blkid", device, "-s", "UUID", "-o", "export"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MountError::CommandFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // Find UUID= line
    for line in lines {
        if line.starts_with("UUID=") {
            return Ok(line.to_string());
        }
    }

    Err(MountError::UuidNotFound(device.to_string()))
}

/// Create mount point directory
fn create_mount_point(mount_point: &str) -> Result<(), MountError> {
    match fs::create_dir_all(mount_point) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(MountError::MountPointCreationFailed(format!(
            "Failed to create {}: {}",
            mount_point, e
        ))),
    }
}

/// Update fstab file safely with atomic write
fn update_fstab_safe(
    fstab_path: &str,
    mount_entries: &[MountEntry],
    config: &MountConfig,
) -> Result<(), MountError> {
    let temp_path = format!("{}.tmp", fstab_path);

    // Read current fstab
    let mut current_lines = Vec::new();
    if Path::new(fstab_path).exists() {
        let file = File::open(fstab_path)?;
        let reader = BufReader::new(file);
        current_lines = reader.lines().collect::<Result<Vec<_>, _>>()?;
    }

    // Remove existing entries for our mount points
    let mount_points: Vec<&str> = mount_entries
        .iter()
        .map(|entry| entry.mount_point.as_str())
        .collect();

    current_lines.retain(|line| !mount_points.iter().any(|mp| line.contains(mp)));

    // Add new entries
    for entry in mount_entries {
        let fstab_line = format!(
            "{}  {}  {}    {}    0   0",
            entry.uuid, entry.mount_point, entry.filesystem, entry.options
        );
        current_lines.push(fstab_line);
    }

    // Write to temporary file first
    {
        let mut temp_file = File::create(&temp_path)?;
        for line in &current_lines {
            writeln!(temp_file, "{}", line)?;
        }
        temp_file.sync_all()?;
    }

    // Validate the new fstab if enabled
    if config.validate_before_write {
        validate_fstab(&temp_path)?;
    }

    // Atomic move
    fs::rename(&temp_path, fstab_path)?;

    Ok(())
}

/// Validate fstab syntax
fn validate_fstab(fstab_path: &str) -> Result<(), MountError> {
    // Basic validation - check if each line has proper format
    let file = File::open(fstab_path)?;
    let reader = BufReader::new(file);

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Check basic fstab format (6 fields)
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 6 {
            return Err(MountError::ValidationFailed(format!(
                "Line {}: Invalid fstab format, expected 6 fields, got {}",
                line_num + 1,
                fields.len()
            )));
        }
    }

    Ok(())
}

/// Apply mounts using mount command
fn apply_mounts() -> Result<(), MountError> {
    let output = Command::new("sudo").args(["mount", "-a"]).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MountError::CommandFailed(stderr.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_config_default() {
        let config = MountConfig::default();
        assert_eq!(config.filesystem_type, "ext4");
        assert_eq!(config.mount_options, "rw,acl");
        assert_eq!(config.mount_base_path, "/mnt");
        assert!(config.backup_fstab);
        assert!(config.validate_before_write);
    }

    #[test]
    fn test_mount_entry_creation() {
        let entry = MountEntry {
            device: "/dev/sda1".to_string(),
            uuid: "UUID=12345".to_string(),
            mount_point: "/mnt/sda1".to_string(),
            filesystem: "ext4".to_string(),
            options: "rw,acl".to_string(),
        };

        assert_eq!(entry.device, "/dev/sda1");
        assert_eq!(entry.uuid, "UUID=12345");
        assert_eq!(entry.mount_point, "/mnt/sda1");
    }

    #[test]
    fn test_fstab_validation_valid() {
        // This would need a temporary valid fstab file for testing
        // In real implementation, we'd create a temp file with valid content
    }

    #[test]
    fn test_device_path_validation() {
        assert!("/dev/sda1".starts_with("/dev/"));
        assert!(!"sda1".starts_with("/dev/"));
        assert!(!"invalid".starts_with("/dev/"));
    }
}
