//! Headless reference crowd proof through the registered session world.

mod image;
pub use image::{capture, restore};

use thrum_core::sim::RuleSetIdentity;

const RULE_SET: RuleSetIdentity =
    RuleSetIdentity(*include_bytes!(concat!(env!("OUT_DIR"), "/rule-set-id")));

/// Rule-set identity embedded in this build, in hexadecimal.
pub fn identity() -> String {
    hex(&RULE_SET.0)
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
