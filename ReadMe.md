# auto_mount

A safe and intelligent Rust library that automatically mounts newly inserted SATA devices with proper error handling and backup mechanisms.

## Features

- 🔍 **Smart Device Discovery**: Automatically finds connected SATA devices
- 🛡️ **Safe Operations**: Comprehensive error handling with backup and recovery
- 🧠 **Intelligent Mounting**: Auto-decides GPT conversion based on disk size
- 📁 **Multiple Filesystems**: Support for ext4, xfs, btrfs, ntfs, fat32, and more
- ⚡ **Flexible API**: Both high-level smart mounting and fine-grained control
- 🔒 **System Safety**: Atomic operations with `/etc/fstab` backup and validation

## Quick Start

### High-level Smart Mounting (Recommended)

```rust
use auto_mount::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Automatically handles everything with intelligent decisions
    smart_auto_mount()?;
    
    // Alternative options:
    // simple_auto_mount()?;  // No GPT conversion
    // gpt_auto_mount()?;     // Force GPT for all devices
    
    Ok(())
}
```

### Fine-grained Control

```rust
use auto_mount::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = find_connected_satas()?;
    let devices = filter_unmounted_hdd_devices(devices)?;
    
    // Optional: Convert to GPT (only for disks > 2TB)
    change_devices_to_gpt(&devices)?;
    
    let devices = create_partition(&devices)?;
    format_devices(&devices)?;
    mount_devices(&devices)?;
    
    Ok(())
}
```

### Custom Configuration

```rust
use auto_mount::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Custom smart mounting
    let config = MountConfig {
        force_gpt: false,
        gpt_threshold_gb: 1000, // Use GPT for disks >= 1TB
        skip_gpt: false,
    };
    smart_auto_mount_with_config(config)?;
    
    // Custom filesystem
    format_devices_with_type(&devices, FilesystemType::Xfs)?;
    
    Ok(())
}
```

## Supported Filesystems

- **ext4** (default) - Standard Linux filesystem
- **ext3/ext2** - Legacy Linux filesystems  
- **xfs** - High-performance filesystem
- **btrfs** - Advanced filesystem with snapshots
- **ntfs** - Windows compatibility
- **fat32** - Universal compatibility

```rust
// List all supported filesystems
let supported = FilesystemType::supported_type_names();
println!("Supported: {:?}", supported);

// Check if filesystem is supported
if FilesystemType::is_supported("xfs") {
    let fs_type: FilesystemType = "xfs".parse()?;
}
```

## Safety Features

- 🔄 **Automatic Backup**: Creates timestamped backups of `/etc/fstab`
- ✅ **Validation**: Checks fstab syntax before applying changes
- 🔙 **Auto Recovery**: Restores backup if operations fail
- 🛡️ **Atomic Operations**: All-or-nothing approach to prevent corruption
- 📊 **Detailed Results**: Comprehensive error reporting for each device

## Important Caution

⚠️ **This tool formats storage devices!** 
- All unmounted HDDs in `/dev/sd*` will be formatted
- Always backup important data before running
- Test in a safe environment first
- The tool includes safety checks and backups, but use with caution

## Requirements

- Linux system with `sudo` access
- Required system tools: `lsblk`, `parted`, `mkfs.*`, `blkid`, `mount`
- Rust 1.63+ for building from source

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
auto_mount = "0.2.0"
```

## Error Handling

All functions return proper `Result` types with detailed error information:

```rust
match smart_auto_mount() {
    Ok(()) => println!("✅ Successfully mounted devices"),
    Err(SmartMountError::NoDevicesFound) => println!("ℹ️ No devices to mount"),
    Err(e) => eprintln!("❌ Error: {}", e),
}
```

## Contributing

We welcome contributions! Please:

1. Check existing issues or create a new one
2. Fork the repository and create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

This project is licensed under the terms specified in the LICENSE file.

---

**⚠️ Use at your own risk. Always backup important data before running disk operations.**