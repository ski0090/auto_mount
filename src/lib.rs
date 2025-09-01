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
//!     mount_devices(&devices);
//! ```
pub use device_discovery::{find_connected_satas, DeviceDiscoveryError};
pub use device_filter::{filter_unmounted_hdd_devices, DeviceFilterError, DeviceInfo};
pub use error::Error;
pub use filesystem::{format_devices, FilesystemError, FilesystemType, FormatResult};
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
mod partition_manager;
mod smart_mount;

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, ErrorKind, Seek, Write};
use std::{
    collections::VecDeque,
    fs::create_dir,
    process::{Command, Output},
};

pub fn mount_devices(devices: &[String]) {
    let fstab_path = "/etc/fstab";
    command(["chmod", "666", fstab_path]);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(fstab_path)
        .unwrap();
    let buf = BufReader::new(&file);
    let mut save = buf
        .lines()
        .filter_map(move |line| line.ok())
        .collect::<Vec<_>>();
    let mounts = devices
        .iter()
        .map(|dev| {
            let mount_path = dev.split('/').collect::<Vec<_>>()[2];
            let mount_path = format!("/mnt/{}", mount_path);
            if let Err(err) = create_dir(&mount_path) {
                if err.kind() != ErrorKind::AlreadyExists {
                    panic!("{}", err);
                }
            }
            (find_uuid(dev), mount_path)
        })
        .collect::<Vec<_>>();

    save.retain(|line| !mounts.iter().any(|(_, mp)| line.contains(mp)));
    let mut fstab_appends = mounts
        .iter()
        .map(|(uuid, mp)| format!("{}  {}  ext4    rw,acl    0   0", uuid, mp))
        .collect::<Vec<_>>();
    save.append(&mut fstab_appends);
    let save = save
        .into_iter()
        .map(|line| line.as_bytes().to_vec())
        .collect::<Vec<_>>()
        .join("\n".as_bytes());
    file.seek(std::io::SeekFrom::Start(0)).unwrap();
    file.write_all(&save).unwrap();

    command(["chmod", "664", fstab_path]);

    command(["mount", "-a"]);
}

fn output_to_string_list(output: Output) -> VecDeque<String> {
    if !output.stderr.is_empty() {
        panic!("{}", String::from_utf8(output.stderr).unwrap());
    }
    let mut outputs = String::from_utf8(output.stdout)
        .unwrap()
        .split('\n')
        .map(|str| str.to_owned())
        .collect::<VecDeque<String>>();
    outputs.pop_back(); // NOTE: remove empty string
    outputs
}

fn find_uuid(device: &str) -> String {
    let output = command(["blkid", device, "-s", "UUID", "-o", "export"]);
    output_to_string_list(output)[1].clone()
}

fn command<I, S>(command: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("sudo")
        .args(command)
        .output()
        .expect("failed to execute process")
}

#[test]
fn sudo_test() {
    assert!(Command::new("sudo")
        .args(["find", "/dev", "-name", "-sd?"])
        .status()
        .unwrap()
        .success())
}
