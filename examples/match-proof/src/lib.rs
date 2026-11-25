//! Headless reference crowd proof through the registered session world.

mod image;
#[cfg(feature = "sockets")]
pub mod sockets;

#[cfg(all(feature = "windowed", any(target_os = "macos", target_arch = "wasm32")))]
pub mod windowed;

#[cfg(any(not(target_arch = "wasm32"), feature = "sockets"))]
mod world;
pub use image::{capture, restore};

#[cfg(not(target_arch = "wasm32"))]
mod memory;
#[cfg(not(target_arch = "wasm32"))]
pub use memory::{MemoryProofError, MemoryRun, run_fault, run_memory, run_prediction};

use thrum_core::sim::RuleSetIdentity;

fn rule_set(mixed: bool) -> RuleSetIdentity {
    let bytes = if mixed {
        include_bytes!(concat!(env!("OUT_DIR"), "/rule-set-mixed"))
    } else {
        include_bytes!(concat!(env!("OUT_DIR"), "/rule-set-crowd"))
    };
    RuleSetIdentity(*bytes)
}

/// Configured Crowd or Mixed rule-set identity, in hexadecimal.
pub fn identity(mixed: bool) -> String {
    hex(&rule_set(mixed).0)
}

/// Install a Mac image in a fresh browser runtime and continue the trace.
///
/// # Errors
/// The runtime refuses an incompatible or invalid image.
///
/// # Panics
/// A reference trace tick fails.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = restore)]
pub fn restore_in_browser(
    count: u32,
    mixed: bool,
    bytes: &[u8],
    last: u32,
) -> Result<String, wasm_bindgen::JsError> {
    restore(count, mixed, bytes, last)
        .map_err(|error| wasm_bindgen::JsError::new(&error.to_string()))
}

#[cfg(any(not(target_arch = "wasm32"), feature = "sockets"))]
pub(crate) fn seeded_axes(seed: u64, slot: usize, tick: u64) -> [f32; 3] {
    // Hold most axes briefly so both axis encodings are represented in the cost profile.
    let value = seed ^ (tick / 11).wrapping_mul(0x9e37_79b9) ^ ((slot as u64 + 1) * 0x85eb_ca77);
    let value = value.wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    let axis = |shift| {
        let integer = u16::try_from((value >> shift) & 1023_u64).expect("ten-bit input");
        f32::from(integer) / 521.7 - 0.98
    };
    [axis(32), axis(49), 0.0]
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    output
}
