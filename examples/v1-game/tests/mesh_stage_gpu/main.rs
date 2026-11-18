//! Metal proof of the opted-in cooked forward path in the proving ground.

#[cfg(all(feature = "draw-probe", target_os = "macos"))]
mod metal;
#[cfg(all(feature = "draw-probe", target_os = "macos"))]
mod support;
#[cfg(all(feature = "draw-probe", target_os = "macos"))]
mod valley;
