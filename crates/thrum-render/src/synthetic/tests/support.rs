//! Device setup shared by scene-renderer unit tests.

use crate::capabilities::DeviceRestriction;

/// The portable profile's device with every optional feature removed, for `crate::test_device::open`.
pub(in crate::synthetic) const NO_FEATURES: DeviceRestriction = DeviceRestriction {
    limits: None,
    features: Some(wgpu::Features::empty()),
    remove_features: wgpu::Features::empty(),
};
