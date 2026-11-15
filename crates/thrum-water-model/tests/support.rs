//! Shared public-API test support; each target uses a subset of these fixtures.
#![expect(
    dead_code,
    reason = "shared fixtures are used by different integration test targets"
)]
pub(crate) mod fixtures;
pub(crate) mod joined;
pub(crate) mod sheets;
pub(crate) mod shore;
pub(crate) mod surf_map;
pub(crate) mod untransformed;
