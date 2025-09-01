use auto_mount::{
    change_devices_to_gpt, create_partition, filter_unmounted_hdd_devices, find_connected_satas,
    format_devices, mount_devices, Error,
};

fn main() -> Result<(), Error> {
    let devices = find_connected_satas()?;
    let devices = filter_unmounted_hdd_devices(devices)?;
    change_devices_to_gpt(&devices);
    let devices = create_partition(&devices)?;
    format_devices(&devices)?;
    mount_devices(&devices);

    Ok(())
}
