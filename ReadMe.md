# auto_mount

auto_mount has a function that automatically mounts a newly inserted device as a gpt partition.

## Caution

# example
```rust
use auto_mount::{
    change_devices_to_gpt, create_partition, filter_unmounted_hdd_devices, find_connected_satas,
    format_devices, mount_devices,
};

fn main() {
    let devices = find_connected_satas();
    let devices = filter_unmounted_hdd_devices(devices);
    change_devices_to_gpt(&devices);
    let devices = create_partition(&devices);
    format_devices(&devices);
    mount_devices(&devices);
}
```