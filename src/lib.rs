//! # auto_mount
//!
//! auto_mount has a function that automatically mounts a newly inserted device as a gpt partition.
//! # example
//! ```
//!     let devices = find_connected_satas();
//!     let devices = filter_unmounted_hdd_devices(devices);
//!     change_devices_to_gpt(&devices);
//!     let devices = create_partition(&devices);
//!     format_devices(&devices);
//!     mount_devices(&devices);
//! ```

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, ErrorKind, Read, Seek, Write};
use std::process::Stdio;
use std::{
    collections::VecDeque,
    fs::create_dir,
    process::{Command, Output},
};

use sysinfo::{DiskExt, RefreshKind, SystemExt};

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
            if let Err(err) = create_dir(&mount_path) {
                if err.kind() != ErrorKind::AlreadyExists {
                    panic!("{}", err);
                }
            }
            (find_uuid(dev), format!("/mnt/{}", mount_path))
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

pub fn format_devices(devices: &[String]) {
    devices.iter().for_each(|device| {
        command(["mkfs.ext4", "-F", &device]);
    });
}

/// one device one partition
pub fn create_partition(devices: &[String]) -> Vec<String> {
    devices
        .iter()
        .map(|device| {
            let mut answers = Command::new("printf")
                .arg("n\n\n\n\nw\n\"")
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let mut fdisk = Command::new("sudo")
                .args(["fdisk", &device])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();

            if let Some(ref mut stdout) = answers.stdout {
                if let Some(ref mut stdin) = fdisk.stdin {
                    let mut buf: Vec<u8> = Vec::new();
                    stdout.read_to_end(&mut buf).unwrap();
                    stdin.write_all(&buf).unwrap();
                }
            }

            let res = fdisk.wait_with_output().unwrap().stdout;
            println!("{:?}", String::from_utf8_lossy(&res));
            String::from(device) + "1"
        })
        .collect::<Vec<_>>()
}

/// changed to gpt to support devices larger than 4TB
pub fn change_devices_to_gpt(devices: &[String]) {
    devices.iter().for_each(|device| {
        command(["parted", "-s", &device, "mklabel", "gpt"]);
    });
}

/// find unmounted hdd devices
pub fn filter_unmounted_hdd_devices(devices: Vec<String>) -> Vec<String> {
    let mut system =
        sysinfo::System::new_with_specifics(RefreshKind::new().with_disks().with_disks_list());
    system.refresh_all();

    devices
        .into_iter()
        .filter(|device| {
            let output = command(["lsblk", "-d", "-o", "rota", &device]);
            let result = output_to_string_list(output)[1].to_owned();
            let mut iter = result.split_whitespace();
            let is_mounted = system.disks().iter().any(|disk| {
                let disk_name = disk.name().to_string_lossy().to_string();
                disk_name.contains(device)
            });
            iter.next() == Some("1") && !is_mounted
        })
        .collect::<Vec<String>>()
}

/// find connected satas
pub fn find_connected_satas() -> Vec<String> {
    let output = command(["find", "/dev", "-name", "sd?"]);
    let mut devices = Vec::from(output_to_string_list(output));
    devices.sort();
    devices
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
    dbg!(device);

    let output = command(["blkid", &device, "-s", "UUID", "-o", "export"]);
    dbg!(&output);
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
