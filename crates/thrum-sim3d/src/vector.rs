use std::ops::{Add, Div, Mul, Neg, Sub};

use thrum_core::profile::{Number, StateEncode};

/// A position, displacement or direction in three profile coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vector3 {
    /// First coordinate.
    pub x: Number,
    /// Second coordinate.
    pub y: Number,
    /// Third coordinate.
    pub z: Number,
}

impl Vector3 {
    /// The origin or a zero displacement.
    pub const ZERO: Self = Self::new(Number::ZERO, Number::ZERO, Number::ZERO);
    /// World axes in x, y, z order.
    pub const AXES: [Self; 3] = [
        Self::new(Number::ONE, Number::ZERO, Number::ZERO),
        Self::new(Number::ZERO, Number::ONE, Number::ZERO),
        Self::new(Number::ZERO, Number::ZERO, Number::ONE),
    ];

    /// Make a vector from three profile coordinates.
    pub const fn new(x: Number, y: Number, z: Number) -> Self {
        Self { x, y, z }
    }

    /// Scalar product.
    pub fn dot(self, rhs: Self) -> Number {
        (self.x * rhs.x + self.y * rhs.y) + self.z * rhs.z
    }

    /// Vector product in the right-handed coordinate frame.
    #[must_use]
    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    /// Euclidean length.
    pub fn length(self) -> Number {
        let squared = self.dot(self);
        if squared > Number::ZERO || self == Self::ZERO {
            return squared.sqrt();
        }
        let scale = self.max_component();
        let scaled = self / scale;
        scale * scaled.dot(scaled).sqrt()
    }

    pub(crate) fn max_component(self) -> Number {
        self.x.abs().max(self.y.abs()).max(self.z.abs())
    }

    pub(crate) fn unit(self) -> Option<Self> {
        let scale = self.max_component();
        if scale == Number::ZERO {
            return None;
        }
        let scaled = self / scale;
        Some(scaled / scaled.dot(scaled).sqrt())
    }

    pub(crate) fn is_finite(self) -> bool {
        [self.x, self.y, self.z].iter().all(StateEncode::is_finite)
    }
}

impl Add for Vector3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vector3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<Number> for Vector3 {
    type Output = Self;

    fn mul(self, rhs: Number) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Div<Number> for Vector3 {
    type Output = Self;

    fn div(self, rhs: Number) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl Neg for Vector3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self::ZERO - self
    }
}
