use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

use thrum_core::profile::Number;

use crate::{
    Data,
    data::{DataReader, DataWriter, InvalidData},
};

use thrum_core::profile::{StateDecode, StateEncode};

/// A portable number used in a rule.
/// ```
/// use thrum::Num;
/// assert_eq!(Num::from(2) + Num::ONE, Num::from(3));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Num(pub(crate) Number);

impl Num {
    /// The portable number zero.
    /// ```
    /// assert_eq!(thrum::Num::ZERO, thrum::Num::from(0));
    /// ```
    pub const ZERO: Self = Self(Number::ZERO);

    /// The portable number one.
    /// ```
    /// assert_eq!(thrum::Num::ONE, thrum::Num::from(1));
    /// ```
    pub const ONE: Self = Self(Number::ONE);

    /// Convert to an integer, truncating the fraction.
    /// ```
    /// assert_eq!(thrum::Num::ONE.to_i32(), 1);
    /// ```
    pub fn to_i32(self) -> i32 {
        self.0.to_i32()
    }

    pub(crate) fn ratio(value: u64, scale: u64) -> Self {
        Self::from_u64(value) / Self::from_u64(scale)
    }

    fn from_u64(value: u64) -> Self {
        // Horner conversion stays inside the profile's portable operations.
        let limb = |value| Self::from(i32::from(value));
        let [a, b, c, d, e, f, g, h] = value.to_le_bytes();
        let [lo, mid_lo, mid_hi, hi] = [[a, b], [c, d], [e, f], [g, h]].map(u16::from_le_bytes);
        let shift = Self::from(65_536);
        ((limb(hi) * shift + limb(mid_hi)) * shift + limb(mid_lo)) * shift + limb(lo)
    }

    pub(crate) fn float(self) -> f32 {
        f32::from_bits(self.0.to_bits())
    }
}

impl From<i32> for Num {
    fn from(value: i32) -> Self {
        Self(Number::from_i32(value))
    }
}

macro_rules! arithmetic {
    ($trait:ident, $method:ident, $operator:tt) => {
        impl $trait for Num {
            type Output = Self;

            fn $method(self, rhs: Self) -> Self {
                Self(self.0 $operator rhs.0)
            }
        }
    };
}
arithmetic!(Add, add, +);
arithmetic!(Sub, sub, -);
arithmetic!(Mul, mul, *);
arithmetic!(Div, div, /);

impl Neg for Num {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl AddAssign for Num {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Num {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Data for Num {
    const SCHEMA: &'static str = "thrum::Num(binary32)";

    fn same_encoding(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }

    fn write(&self, writer: &mut DataWriter<'_, '_>) {
        self.0.encode(writer.0);
    }

    fn read(reader: &mut DataReader<'_, '_>) -> Result<Self, InvalidData> {
        Number::decode(reader.0).map(Self).map_err(|_| InvalidData)
    }
}

/// Two portable coordinates.
/// ```
/// use thrum::{Num, Vec2, vec2};
/// assert_eq!(vec2(1, 2), Vec2 { x: Num::ONE, y: Num::from(2) });
/// ```
#[derive(Data, Copy, Default)]
pub struct Vec2 {
    /// The first coordinate.
    /// ```
    /// assert_eq!(thrum::vec2(1, 2).x, thrum::Num::ONE);
    /// ```
    pub x: Num,
    /// The second coordinate.
    /// ```
    /// assert_eq!(thrum::vec2(1, 2).y, thrum::Num::from(2));
    /// ```
    pub y: Num,
}

impl Vec2 {
    /// A vector with both coordinates at zero.
    /// ```
    /// assert_eq!(thrum::Vec2::ZERO, thrum::vec2(0, 0));
    /// ```
    pub const ZERO: Self = Self {
        x: Num::ZERO,
        y: Num::ZERO,
    };
}

/// Make a vector from two portable coordinates.
/// ```
/// assert_eq!(thrum::vec2(0, 0), thrum::Vec2::ZERO);
/// ```
pub fn vec2(x: impl Into<Num>, y: impl Into<Num>) -> Vec2 {
    Vec2 {
        x: x.into(),
        y: y.into(),
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        vec2(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        vec2(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<Num> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: Num) -> Self {
        vec2(self.x * rhs, self.y * rhs)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
