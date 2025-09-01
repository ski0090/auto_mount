use crate::{DeviceDiscoveryError, DeviceFilterError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Device discovery error: {0}")]
    DeviceDiscoveryError(#[from] DeviceDiscoveryError),
    #[error("Device filter error: {0}")]
    DeviceFilterError(#[from] DeviceFilterError),
}
