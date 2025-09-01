use crate::{DeviceDiscoveryError, DeviceFilterError, FilesystemError, PartitionError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Device discovery error: {0}")]
    DeviceDiscoveryError(#[from] DeviceDiscoveryError),
    #[error("Device filter error: {0}")]
    DeviceFilterError(#[from] DeviceFilterError),
    #[error("Partition error: {0}")]
    PartitionError(#[from] PartitionError),
    #[error("Filesystem error: {0}")]
    FilesystemError(#[from] FilesystemError),
}
