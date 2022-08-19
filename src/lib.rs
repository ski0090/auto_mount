//! # auto_mount
//!
//! auto_mount has a function that automatically mounts a newly inserted device as a gpt partition.
//! # example
//! ```
//! use auto_mount::{
//!     change_devices_to_gpt, create_partition, filter_unmounted_hdd_devices, find_connected_satas,
//!     format_devices, mount_devices,
//! };
//!
//! fn main() {
//!     let devices = find_connected_satas();
//!     let devices = filter_unmounted_hdd_devices(devices);
//!     change_devices_to_gpt(&devices);
//!     let devices = create_partition(&devices);
//!     format_devices(&devices);
//!     mount_devices(&devices);
//! }
//! ```

use std::ffi::OsStr;
use std::io::{ErrorKind, Write};
use std::{
    collections::VecDeque,
    fs::{create_dir, OpenOptions},
    process::{Command, Output},
};

use sysinfo::{DiskExt, RefreshKind, SystemExt};

pub fn mount_devices(devices: &Vec<String>) {
    devices.iter().for_each(|device| {
        let mount_path = device.split("/").collect::<Vec<_>>()[2];
        let mount_path = format!("/mnt/{}", mount_path);
        if let Err(err) = create_dir(&mount_path) {
            if err.kind() != ErrorKind::AlreadyExists {
                panic!("{}", err);
            }
        }
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open("/etc/fstab")
            .unwrap();
        let uuid = find_uuid(device);
        if let Err(e) = writeln!(file, "{}  {}  ext4    defaults    0   0", uuid, mount_path) {
            dbg!("Couldn't write to file: {}", e);
        }
    });

    command("mount -a");
}

pub fn format_devices(devices: &Vec<String>) {
    devices.iter().for_each(|device| {
        command(format!("mkfs.ext4 -F {}", device));
    });
}

/// one device one partition
pub fn create_partition(devices: &Vec<String>) -> Vec<String> {
    devices
        .iter()
        .map(|device| {
            command(format!("printf \"n\n\n\n\nw\n\" | fdisk {}", device));
            String::from(device) + "1"
        })
        .collect::<Vec<_>>()
}

/// changed to gpt to support devices larger than 4TB
pub fn change_devices_to_gpt(devices: &Vec<String>) {
    devices.iter().for_each(|device| {
        command(format!("parted -s {} mklabel gpt", device));
    });
}

/// find unmounted hdd devices
pub fn filter_unmounted_hdd_devices(devices: Vec<String>) -> Vec<String> {
    let mut system =
        sysinfo::System::new_with_specifics(RefreshKind::new().with_disks().with_disks_list());
    system.refresh_all();

    let hdds = devices
        .into_iter()
        .filter(|device| {
            let output = command(format!("lsblk -d -o rota {}", device));
            let result = output_to_string_list(output)[1].to_owned();
            let mut iter = result.split_whitespace();
            let is_mounted = system.disks().iter().any(|disk| {
                let disk_name = disk.name().to_string_lossy().to_string();
                disk_name.contains(device)
            });
            iter.next() == Some("1") && !is_mounted
        })
        .collect::<Vec<String>>();

    hdds
}

/// find connected satas
pub fn find_connected_satas() -> Vec<String> {
    let output = command("find /dev -name \"sd?\"");
    let mut devices = Vec::from(output_to_string_list(output));
    devices.sort();
    devices
}

fn output_to_string_list(output: Output) -> VecDeque<String> {
    let mut outputs = String::from_utf8(output.stdout)
        .unwrap()
        .split('\n')
        .map(|str| str.to_owned())
        .collect::<VecDeque<String>>();
    outputs.pop_back(); // NOTE: remove empty string
    outputs
}

fn find_uuid(device: &str) -> String {
    let output = command(format!("blkid {} -s UUID -o export", device));
    output_to_string_list(output)[1].clone()
}

fn command<S>(command: S) -> Output
where
    S: AsRef<OsStr>,
{
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .expect("failed to execute process")
}
