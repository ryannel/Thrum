//! Portable rule arithmetic and canonical authority encoding.

use std::ops::{Add, Div, Mul, Neg, Sub};

#[cfg(test)]
mod tests;

/// Exactly rounded binary32 arithmetic for deterministic rules.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[repr(transparent)]
#[must_use]
pub struct Number(f32);

impl Number {
    /// Zero.
    pub const ZERO: Self = Self(0.0);
    /// One.
    pub const ONE: Self = Self(1.0);

    /// Convert an integer with binary32 rounding.
    pub fn from_i32(value: i32) -> Self {
        Self(value as f32)
    }
    /// Saturating integer conversion, with NaN mapped to zero.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "profile specifies Rust saturating conversion"
    )]
    pub fn to_i32(self) -> i32 {
        self.0 as i32
    }
    /// Convert a device axis through the input wire's canonical quantization.
    #[expect(clippy::cast_possible_truncation, reason = "bounded wire quantization")]
    pub fn from_axis(axis: f32) -> Self {
        let quantized = (f64::from(axis.clamp(-1.0, 1.0)) * 32767.0).round() as i16;
        Self(f32::from(quantized) / 32767.0)
    }
    /// Square root, rounded once.
    pub fn sqrt(self) -> Self {
        Self(self.0.sqrt())
    }
    /// Absolute value.
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }
    /// Floor.
    pub fn floor(self) -> Self {
        Self(self.0.floor())
    }
    /// Ceiling.
    pub fn ceil(self) -> Self {
        Self(self.0.ceil())
    }
    /// Truncation toward zero.
    pub fn trunc(self) -> Self {
        Self(self.0.trunc())
    }
    /// Round ties away from zero.
    pub fn round(self) -> Self {
        Self(self.0.round())
    }
    /// Comparison-based minimum, choosing the right operand on equality.
    pub fn min(self, other: Self) -> Self {
        if self < other { self } else { other }
    }
    /// Comparison-based maximum, choosing the right operand on equality.
    pub fn max(self, other: Self) -> Self {
        if self > other { self } else { other }
    }
    /// Comparison-based clamp.
    pub fn clamp(self, low: Self, high: Self) -> Self {
        self.max(low).min(high)
    }
}

macro_rules! arithmetic {
    ($trait:ident, $method:ident, $op:tt) => {
        impl $trait for Number {
            type Output = Self;
            fn $method(self, rhs: Self) -> Self { Self(self.0 $op rhs.0) }
        }
    };
}
arithmetic!(Add, add, +);
arithmetic!(Sub, sub, -);
arithmetic!(Mul, mul, *);
arithmetic!(Div, div, /);
impl Neg for Number {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

/// Two profile numbers, in game-selected planar coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector {
    /// First coordinate.
    pub x: Number,
    /// Second coordinate.
    pub y: Number,
}

/// Integer-only FNV-1a encoding accumulator, with explicitly ordered bytes.
#[derive(Clone, Copy, Debug)]
pub struct Digest(u64, bool);
impl Default for Digest {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325, true)
    }
}
impl Digest {
    /// Append bytes in the caller's canonical order.
    pub fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
        }
    }
    /// Current digest.
    pub fn finish(self) -> u64 {
        self.0
    }
}

/// A fixed-width state encoding. Implementations must encode every field and
/// propagate the validity of nested values. Raw floats and usize have no implementation.
pub trait StateEncode {
    /// Append canonical bytes, with lengths encoded as u64.
    fn encode(&self, digest: &mut Digest);
    /// Whether all nested profile values are finite. The default validates the
    /// complete encoding. An override must propagate every nested value's validity.
    fn is_finite(&self) -> bool {
        let mut digest = Digest::default();
        self.encode(&mut digest);
        digest.1
    }
}
macro_rules! integers {
    ($($ty:ty),*) => { $(impl StateEncode for $ty {
        fn encode(&self, digest: &mut Digest) { digest.bytes(&self.to_le_bytes()); }
        fn is_finite(&self) -> bool { true }
    })* };
}
integers!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl StateEncode for bool {
    fn is_finite(&self) -> bool {
        true
    }
    fn encode(&self, digest: &mut Digest) {
        u8::from(*self).encode(digest);
    }
}
impl<T: StateEncode, const N: usize> StateEncode for [T; N] {
    fn encode(&self, digest: &mut Digest) {
        u64::try_from(N)
            .expect("array length fits u64")
            .encode(digest);
        for value in self {
            value.encode(digest);
        }
    }
    fn is_finite(&self) -> bool {
        self.iter().all(StateEncode::is_finite)
    }
}
impl StateEncode for Number {
    fn encode(&self, digest: &mut Digest) {
        digest.1 &= self.0.is_finite();
        let bits = if self.0.is_nan() {
            0x7fc0_0000
        } else {
            self.0.to_bits()
        };
        bits.encode(digest);
    }
    fn is_finite(&self) -> bool {
        self.0.is_finite()
    }
}
impl StateEncode for Vector {
    fn encode(&self, digest: &mut Digest) {
        self.x.encode(digest);
        self.y.encode(digest);
    }
    fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// Philox2x32-10 counter-based random number. The tick, stable identity and stream
/// are folded into its fixed-width counter and key. No cursor is retained.
#[expect(
    clippy::cast_possible_truncation,
    reason = "Philox uses high and low words"
)]
pub fn random(seed: u64, tick: u64, entity: u128, stream: u32) -> Number {
    let mut x = tick as u32 ^ entity as u32 ^ (entity >> 64) as u32;
    let mut y = (tick >> 32) as u32 ^ (entity >> 32) as u32 ^ (entity >> 96) as u32 ^ stream;
    let mut key = seed as u32 ^ (seed >> 32) as u32;
    for _ in 0..10 {
        let product = u64::from(x) * 0xd256_d193;
        x = (product >> 32) as u32 ^ y ^ key;
        y = product as u32;
        key = key.wrapping_add(0x9e37_79b9);
    }
    Number((x >> 8) as f32 / 16_777_216.0)
}

/// Deliberately platform-dependent operation for the conformance negative control.
/// Available only to diagnostic runners. Rule packages must never enable this feature.
#[cfg(feature = "conformance-control")]
pub fn platform_sine_control(value: Number) -> Number {
    Number(value.0.sin())
}

/// Fixed arithmetic conformance fixture. Values are returned as canonical words
/// solely for these fixed operands. NaNs are normalized because their payload
/// and sign are outside the profile's observables.
pub fn conformance_vectors() -> Vec<u32> {
    let words = [
        0,
        0x8000_0000,
        1,
        0x8000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x3f00_0000,
        0xbf00_0000,
        0x3f80_0000,
        0x3f80_0001,
        0x3fff_ffff,
        0x4000_0000,
        0x4b80_0000,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0001,
    ];
    let mut output = Vec::new();
    let word = |value: Number| {
        if value.0.is_nan() {
            0x7fc0_0000
        } else {
            value.0.to_bits()
        }
    };
    for bits in words {
        let a = Number(f32::from_bits(bits));
        for result in [
            a.sqrt(),
            -a,
            a.abs(),
            a.floor(),
            a.ceil(),
            a.trunc(),
            a.round(),
            a.clamp(-Number::ONE, Number::ONE),
        ] {
            output.push(word(result));
        }
        output.extend(a.to_i32().to_le_bytes().map(u32::from));
        for rhs in words {
            let b = Number(f32::from_bits(rhs));
            for result in [a + b, a - b, a * b, a / b, a.min(b), a.max(b)] {
                output.push(word(result));
            }
            output.extend([
                u32::from(a < b),
                u32::from(a <= b),
                u32::from(a == b),
                u32::from(a >= b),
                u32::from(a > b),
            ]);
        }
    }
    for integer in [i32::MIN, -16_777_217, -1, 0, 1, 16_777_217, i32::MAX] {
        output.push(word(Number::from_i32(integer)));
    }
    for (seed, tick, entity, stream) in [
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (u64::MAX, u64::MAX, u128::MAX, u32::MAX),
    ] {
        output.push(word(random(seed, tick, entity, stream)));
    }
    output
}
