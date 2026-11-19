//! Read the forward Sun term at the terminator of a coarse, smooth prism.

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
mod ray_shadow_band_gpu {
    mod terminator;
}
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_shadow_support;
#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod ray_support;
