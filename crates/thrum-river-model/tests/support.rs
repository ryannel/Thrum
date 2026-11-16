//! Shared public-API test support; each target uses a subset of these fixtures.
#![expect(
    dead_code,
    reason = "shared fixtures are used by different integration test targets"
)]
pub(crate) mod section;
pub(crate) mod valley;
