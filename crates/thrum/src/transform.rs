use thrum_core::profile::StateEncode;

use crate::{Component, Data, Num, Vec2, vec2};

/// Three portable coordinates.
#[derive(Data, Copy, Default)]
/// ```
/// use thrum::prelude::*;
/// assert_eq!(vec3(2, 3, 4), Vec3 { x: Num::from(2), y: Num::from(3), z: Num::from(4) });
/// ```
pub struct Vec3 {
    /// The first coordinate.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(vec3(2, 3, 4).x, Num::from(2));
    /// ```
    pub x: Num,
    /// The upward coordinate.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(vec3(2, 3, 4).y, Num::from(3));
    /// ```
    pub y: Num,
    /// The third coordinate.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(vec3(2, 3, 4).z, Num::from(4));
    /// ```
    pub z: Num,
}

impl Vec3 {
    /// A vector with all coordinates at zero.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Vec3::ZERO, vec3(0, 0, 0));
    /// ```
    pub const ZERO: Self = Self {
        x: Num::ZERO,
        y: Num::ZERO,
        z: Num::ZERO,
    };
}

/// Make a vector from three portable coordinates.
/// ```
/// use thrum::prelude::*;
/// assert_eq!(vec3(0, 0, 0), Vec3::ZERO);
/// ```
pub fn vec3(x: impl Into<Num>, y: impl Into<Num>, z: impl Into<Num>) -> Vec3 {
    Vec3 {
        x: x.into(),
        y: y.into(),
        z: z.into(),
    }
}

/// A portable angle, with a full turn represented by one.
#[derive(Data, Copy, Default)]
/// ```
/// use thrum::prelude::*;
/// assert_eq!(Angle::degrees(90), Angle::turns(Num::ONE / Num::from(4)));
/// ```
pub struct Angle(Num);

impl Angle {
    /// The angle of zero turns.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Angle::ZERO, Angle::turns(0));
    /// ```
    pub const ZERO: Self = Self(Num::ZERO);

    /// Make an angle from degrees.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Angle::degrees(360), Angle::ZERO);
    /// ```
    pub fn degrees(degrees: impl Into<Num>) -> Self {
        Self::turns(degrees.into() / Num::from(360))
    }

    /// Make an angle from turns.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Angle::turns(2), Angle::ZERO);
    /// ```
    pub fn turns(turns: impl Into<Num>) -> Self {
        let turns = turns.into();
        let wrapped = turns - Num(turns.0.floor());
        Self(if wrapped == Num::ONE || wrapped == Num::ZERO {
            Num::ZERO
        } else {
            wrapped
        })
    }

    fn plus(self, other: Self) -> Self {
        Self::turns(self.0 + other.0)
    }

    fn inverse(self) -> Self {
        Self::turns(-self.0)
    }

    fn rotate(self, point: Vec2) -> Vec2 {
        let quarters = self.0 * Num::from(4);
        let quadrant = quarters.to_i32();
        let remainder = quarters - Num::from(quadrant);
        let point = if remainder == Num::ZERO {
            point
        } else {
            let (sin, cos) = quarter_sin_cos(remainder);
            vec2(cos * point.x - sin * point.y, sin * point.x + cos * point.y)
        };
        let negate = |n: Num| if n == Num::ZERO { Num::ZERO } else { -n };
        match quadrant {
            1 => vec2(negate(point.y), point.x),
            2 => vec2(negate(point.x), negate(point.y)),
            3 => vec2(point.y, negate(point.x)),
            _ => point,
        }
    }
}

fn quarter_sin_cos(fraction: Num) -> (Num, Num) {
    // Taylor's polynomial on [0, pi/2], evaluated in the portable numbers.
    let radians = fraction * Num::ratio(1_570_796_327, 1_000_000_000);
    let square = radians * radians;
    let sine = radians
        * (Num::ONE
            - square / Num::from(6)
                * (Num::ONE
                    - square / Num::from(20)
                        * (Num::ONE
                            - square / Num::from(42)
                                * (Num::ONE
                                    - square / Num::from(72)
                                        * (Num::ONE - square / Num::from(110))))));
    let cosine = Num::ONE
        - square / Num::from(2)
            * (Num::ONE
                - square / Num::from(12)
                    * (Num::ONE
                        - square / Num::from(30)
                            * (Num::ONE
                                - square / Num::from(56)
                                    * (Num::ONE
                                        - square / Num::from(90)
                                            * (Num::ONE - square / Num::from(132))))));
    (sine, cosine)
}

/// A rotation about the upward axis.
#[derive(Data, Copy, Default)]
/// ```
/// use thrum::prelude::*;
/// let turn = Rot3::about_y(Angle::degrees(90));
/// assert_ne!(turn, Rot3::IDENTITY);
/// ```
pub struct Rot3(Angle);

impl Rot3 {
    /// The rotation that leaves a vector where it is.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Rot3::IDENTITY, Rot3::about_y(Angle::ZERO));
    /// ```
    pub const IDENTITY: Self = Self(Angle::ZERO);

    /// Turn by an angle about the upward axis.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform3d::at(1, 2, 3).turned_y(Angle::degrees(90)).rotation, Rot3::about_y(Angle::degrees(90)));
    /// ```
    pub const fn about_y(angle: Angle) -> Self {
        Self(angle)
    }
}

/// The local position, turn and uniform scale of a 2D entity.
#[derive(Component)]
/// ```
/// use thrum::prelude::*;
/// let at = Transform2d::at(1, 2).scaled(2);
/// assert_eq!(at.position, vec2(1, 2));
/// ```
pub struct Transform2d {
    /// The position relative to the parent, or in world space at the root.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform2d::at(1, 2).position, vec2(1, 2));
    /// ```
    pub position: Vec2,
    /// The local rotation.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform2d::ORIGIN.turn, Angle::ZERO);
    /// ```
    pub turn: Angle,
    /// The local uniform scale.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform2d::ORIGIN.scale, Num::ONE);
    /// ```
    pub scale: Num,
}

/// The local position, rotation and uniform scale of a 3D entity.
#[derive(Component)]
/// ```
/// use thrum::prelude::*;
/// let at = Transform3d::at(1, 2, 3).scaled(2);
/// assert_eq!(at.position, vec3(1, 2, 3));
/// ```
pub struct Transform3d {
    /// The position relative to the parent, or in world space at the root.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform3d::at(1, 2, 3).position, vec3(1, 2, 3));
    /// ```
    pub position: Vec3,
    /// The local rotation.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform3d::ORIGIN.rotation, Rot3::IDENTITY);
    /// ```
    pub rotation: Rot3,
    /// The local uniform scale.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform3d::ORIGIN.scale, Num::ONE);
    /// ```
    pub scale: Num,
}

impl Transform2d {
    /// The unturned origin with unit scale.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform2d::ORIGIN, Transform2d::at(0, 0));
    /// ```
    pub const ORIGIN: Self = Self {
        position: Vec2::ZERO,
        turn: Angle::ZERO,
        scale: Num::ONE,
    };

    /// Place an entity at two coordinates.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform2d::at(5, -1).position, vec2(5, -1));
    /// ```
    pub fn at(x: impl Into<Num>, y: impl Into<Num>) -> Self {
        Self {
            position: vec2(x, y),
            ..Self::ORIGIN
        }
    }

    /// Set the local turn.
    #[must_use]
    /// ```
    /// use thrum::prelude::*;
    /// let turn = Angle::degrees(90);
    /// assert_eq!(Transform2d::ORIGIN.turned(turn).turn, turn);
    /// ```
    pub fn turned(mut self, turn: Angle) -> Self {
        self.turn = turn;
        self
    }

    /// Set the uniform scale.
    #[must_use]
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform2d::ORIGIN.scaled(2).scale, Num::from(2));
    /// ```
    pub fn scaled(mut self, scale: impl Into<Num>) -> Self {
        self.scale = scale.into();
        self
    }

    /// Move immediately to a different point.
    /// ```
    /// use thrum::prelude::*;
    /// let mut at = Transform2d::ORIGIN;
    /// at.teleport(vec2(3, 4));
    /// assert_eq!(at.position, vec2(3, 4));
    /// ```
    pub fn teleport(&mut self, point: Vec2) {
        self.position = point;
    }
}

impl Transform3d {
    /// The unturned origin with unit scale.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform3d::ORIGIN, Transform3d::at(0, 0, 0));
    /// ```
    pub const ORIGIN: Self = Self {
        position: Vec3::ZERO,
        rotation: Rot3::IDENTITY,
        scale: Num::ONE,
    };

    /// Place an entity at three coordinates.
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform3d::at(5, -1, 2).position, vec3(5, -1, 2));
    /// ```
    pub fn at(x: impl Into<Num>, y: impl Into<Num>, z: impl Into<Num>) -> Self {
        Self {
            position: vec3(x, y, z),
            ..Self::ORIGIN
        }
    }

    /// Set the turn about the upward axis.
    #[must_use]
    /// ```
    /// use thrum::prelude::*;
    /// let angle = Angle::degrees(90);
    /// assert_eq!(Transform3d::ORIGIN.turned_y(angle).rotation, Rot3::about_y(angle));
    /// ```
    pub fn turned_y(mut self, angle: Angle) -> Self {
        self.rotation = Rot3::about_y(angle);
        self
    }

    /// Set the uniform scale.
    #[must_use]
    /// ```
    /// use thrum::prelude::*;
    /// assert_eq!(Transform3d::ORIGIN.scaled(2).scale, Num::from(2));
    /// ```
    pub fn scaled(mut self, scale: impl Into<Num>) -> Self {
        self.scale = scale.into();
        self
    }

    /// Move immediately to a different point.
    /// ```
    /// use thrum::prelude::*;
    /// let mut at = Transform3d::ORIGIN;
    /// at.teleport(vec3(3, 4, 5));
    /// assert_eq!(at.position, vec3(3, 4, 5));
    /// ```
    pub fn teleport(&mut self, point: Vec3) {
        self.position = point;
    }
}

pub(crate) trait Transform: Component + Copy {
    const ORIGIN: Self;
    fn compose(self, local: Self) -> Self;
    fn local(self, world: Self) -> Self;
    fn scale(self) -> Num;
    fn residual(self, expected: Self, parent: Self) -> (Num, Num);
}

impl Transform for Transform2d {
    const ORIGIN: Self = Self::ORIGIN;

    fn compose(self, local: Self) -> Self {
        Self {
            position: self.position + self.turn.rotate(local.position * self.scale),
            turn: self.turn.plus(local.turn),
            scale: self.scale * local.scale,
        }
    }

    fn local(self, world: Self) -> Self {
        let delta = self.turn.inverse().rotate(world.position - self.position);
        Self {
            position: vec2(delta.x / self.scale, delta.y / self.scale),
            turn: self.turn.inverse().plus(world.turn),
            scale: world.scale / self.scale,
        }
    }

    fn scale(self) -> Num {
        self.scale
    }

    fn residual(self, expected: Self, parent: Self) -> (Num, Num) {
        let p = self.position;
        let e = expected.position;
        let b = parent.position;
        residual(
            &[p.x, p.y],
            &[e.x, e.y],
            &[b.x, b.y],
            self.turn,
            expected.turn,
            self.scale,
            expected.scale,
        )
    }
}

impl Transform for Transform3d {
    const ORIGIN: Self = Self::ORIGIN;

    fn compose(self, local: Self) -> Self {
        let turn = self.rotation.0;
        let plane = turn.rotate(vec2(local.position.x, -local.position.z) * self.scale);
        Self {
            position: vec3(
                self.position.x + plane.x,
                self.position.y + local.position.y * self.scale,
                self.position.z - plane.y,
            ),
            rotation: Rot3(turn.plus(local.rotation.0)),
            scale: self.scale * local.scale,
        }
    }

    fn local(self, world: Self) -> Self {
        let turn = self.rotation.0.inverse();
        let plane = turn.rotate(vec2(
            world.position.x - self.position.x,
            self.position.z - world.position.z,
        ));
        Self {
            position: vec3(
                plane.x / self.scale,
                (world.position.y - self.position.y) / self.scale,
                -plane.y / self.scale,
            ),
            rotation: Rot3(turn.plus(world.rotation.0)),
            scale: world.scale / self.scale,
        }
    }

    fn scale(self) -> Num {
        self.scale
    }

    fn residual(self, expected: Self, parent: Self) -> (Num, Num) {
        let p = self.position;
        let e = expected.position;
        let b = parent.position;
        residual(
            &[p.x, p.y, p.z],
            &[e.x, e.y, e.z],
            &[b.x, b.y, b.z],
            self.rotation.0,
            expected.rotation.0,
            self.scale,
            expected.scale,
        )
    }
}

fn maximum(a: Num, b: Num) -> Num {
    if a >= b { a } else { b }
}

fn max_abs(values: impl IntoIterator<Item = Num>) -> Num {
    values
        .into_iter()
        .fold(Num::ZERO, |a, b| maximum(a, Num(b.0.abs())))
}

fn residual(
    actual: &[Num],
    expected: &[Num],
    parent: &[Num],
    turn: Angle,
    expected_turn: Angle,
    scale: Num,
    expected_scale: Num,
) -> (Num, Num) {
    let delta = max_abs(actual.iter().zip(expected).map(|(&a, &b)| a - b));
    let extent = maximum(Num::ONE, max_abs(expected.iter().chain(parent).copied()));
    let basis = vec2(Num::ONE, Num::ZERO);
    let linear = turn.rotate(basis) * scale - expected_turn.rotate(basis) * expected_scale;
    let relative = max_abs([linear.x, linear.y, scale - expected_scale])
        / maximum(Num::ONE, Num(expected_scale.0.abs()));
    (maximum(delta / extent, relative), Num::ratio(1, 1 << 17))
}

pub(crate) fn preserve<T: Transform>(
    parent: T,
    before: T,
    child: crate::Entity,
) -> crate::Result<T> {
    let scale = Num(parent.scale().0.abs());
    if !(Num::ratio(1, 10_000)..=Num::from(10_000)).contains(&scale) {
        return Err(crate::Error::refused(
            "T0311",
            format!("{child:?}"),
            format!(
                "keep_world parent scale {} is outside 1/10,000 to 10,000",
                scale.float()
            ),
            "Use keep_local or a parent with a supported world scale.",
        ));
    }
    let local = parent.local(before);
    let recomposed = parent.compose(local);
    let (error, bound) = recomposed.residual(before, parent);
    if crate::data::Stored(local).is_finite()
        && crate::data::Stored(recomposed).is_finite()
        && error <= bound
    {
        Ok(local)
    } else {
        let error = if crate::data::Stored(recomposed).is_finite() {
            error.float().to_string()
        } else {
            String::from("non-finite")
        };
        Err(crate::Error::refused(
            "T0312",
            format!("{child:?}"),
            format!(
                "keep_world residual {error} exceeds bound {}",
                bound.float()
            ),
            "Use keep_local or choose a better-conditioned parent.",
        ))
    }
}
