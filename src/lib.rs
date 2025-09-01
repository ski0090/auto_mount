//! # auto_mount
//!
//! auto_mount provides both high-level smart mounting and fine-grained control for device management.
//!
//! ## High-level Smart Mounting (Recommended)
//! ```ignore
//!     use auto_mount::*;
//!
//!     // Option 1: Smart auto-mount (automatically decides GPT based on disk size)
//!     smart_auto_mount()?;
//!
//!     // Option 2: Simple auto-mount (no GPT conversion)
//!     simple_auto_mount()?;
//!
//!     // Option 3: Force GPT for all devices
//!     gpt_auto_mount()?;
//!
//!     // Option 4: Custom configuration
//!     let config = MountConfig {
//!         force_gpt: false,
//!         gpt_threshold_gb: 1000, // Use GPT for disks >= 1TB
//!         skip_gpt: false,
//!     };
//!     smart_auto_mount_with_config(config)?;
//! ```
//!
//! ## Fine-grained Control
//! ```ignore
//!     use auto_mount::*;
//!
//!     let devices = find_connected_satas()?;
//!     let devices = filter_unmounted_hdd_devices(devices)?;
//!     
//!     // Optional: Convert to GPT (only if needed)
//!     change_devices_to_gpt(&devices)?;
//!     
//!     let devices = create_partition(&devices)?;
//!     format_devices(&devices)?;
//!     mount_devices(&devices)?;
//! ```
pub use device_discovery::{find_connected_satas, DeviceDiscoveryError};
pub use device_filter::{filter_unmounted_hdd_devices, DeviceFilterError, DeviceInfo};
pub use error::Error;
pub use filesystem::{format_devices, FilesystemError, FilesystemType, FormatResult};
pub use mount_manager::{
    mount_devices, MountConfig as MountManagerConfig, MountEntry, MountError, MountResult,
};
pub use partition_manager::{
    change_devices_to_gpt, create_partition, GptConversionResult, PartitionError, PartitionResult,
};
pub use smart_mount::{
    gpt_auto_mount, simple_auto_mount, smart_auto_mount, smart_auto_mount_with_config, MountConfig,
    SmartMountError,
};

mod device_discovery;
mod device_filter;
mod error;
mod filesystem;
mod mount_manager;
mod partition_manager;
mod smart_mount;
