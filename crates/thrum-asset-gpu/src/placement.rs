//! Where a cooked mesh stands: the placement, its builder and the rigid-transform check.

use std::fmt;

use crate::CookedMaterial;

/// Relative tolerance of the rigid-transform check: axis lengths and right angles may differ
/// by this fraction of the scale, which absorbs `f32` rounding in hand-built matrices.
const RIGID_TOLERANCE: f64 = 1e-3;

/// Uniform scales the kit supports: a conservative range in which the model matrix's inverse
/// (uploaded for normals), the mesh's world bounds and shading keep their `f32` precision. It is
/// not where `f32` overflows; the explicit inverse check enforces finiteness.
const SCALE_RANGE: std::ops::RangeInclusive<f32> = 1e-4..=1e4;

/// Furthest the kit supports a placement's origin from the world origin on any axis, in metres:
/// a conservative range, since `f32` world positions lose precision with distance.
const MAX_DISTANCE: f64 = 1e6;

/// A cooked mesh, its base-colour texture and where it stands in the world.
///
/// Build one with [`MeshPlacement::new`] and the placement methods, which only turn, scale
/// uniformly and move. Finite inputs still have to pass the bounds and rigidity checks below, and
/// a turn that names no rotation is refused by name:
///
/// ```
/// use thrum_asset_gpu::{CookedMaterial, MeshPlacement};
///
/// let stone = MeshPlacement::new("meshes/rock", "textures/white")
///     .rotate_degrees([0.0, 0.0, 1.0], 90.0) // stand the mesh's +X axis upright
///     .yaw_degrees(30.0) // then turn it 30° clockwise seen from above
///     .scale(1.5)
///     .at([4.0, 0.0, -2.0])
///     .on_ground() // its lowest point rests at Y = 0
///     .material(CookedMaterial { base_colour: [0.4, 0.4, 0.36, 1.0], ..Default::default() });
/// ```
///
/// A transform written by hand must be rigid with one uniform positive scale (rotation, scale
/// and translation only), because normals, culling bounds and shadow bounds are transformed by
/// it. Its scale must lie within 0.0001–10000 and its origin within 1,000 km of the world
/// origin on each axis. [`crate::CookedAssets::new`] refuses any other transform with a
/// [`PlacementError`] naming the placement, the cause and the fix.
#[derive(Clone, Debug)]
pub struct MeshPlacement {
    /// Manifest name of the mesh product: its source path under the cooked folder without the
    /// extension, such as `meshes/rock` for `assets/meshes/rock.gltf`.
    pub mesh: String,
    /// Manifest name of its base-colour texture, such as `textures/white` for
    /// `assets/textures/white.png`.
    pub texture: String,
    /// Column-major object-to-world transform: rotation, one uniform positive scale and
    /// translation, in metres.
    pub transform: [[f32; 4]; 4],
    /// Material factors applied with the cooked texture.
    pub material: CookedMaterial,
    /// Raise or lower the mesh so that its lowest point, after rotation and scale, sits at the
    /// transform's height (the ground when that is zero) instead of its origin. It uses the
    /// cooked mesh's vertices, so it follows edits to the asset; a mesh of more than 65,536
    /// vertices uses its bounding box instead, which can leave a turned mesh slightly raised.
    pub rest_on_ground: bool,
    /// The first turn that named no rotation, kept for [`crate::CookedAssets::new`] to refuse by
    /// name; the transform is left as it was before that turn.
    refused_turn: Option<([f32; 3], f32)>,
}

impl MeshPlacement {
    /// The mesh and texture at the world origin, unrotated, at scale 1, with the default
    /// material.
    pub fn new(mesh: impl Into<String>, texture: impl Into<String>) -> Self {
        Self {
            mesh: mesh.into(),
            texture: texture.into(),
            transform: glam::Mat4::IDENTITY.to_cols_array_2d(),
            material: CookedMaterial::default(),
            rest_on_ground: false,
            refused_turn: None,
        }
    }

    /// Put the mesh's origin at `position`, in metres. Rotation and scale are kept.
    #[must_use]
    pub fn at(mut self, position: [f32; 3]) -> Self {
        self.transform[3][..3].copy_from_slice(&position);
        self
    }

    /// Turn the mesh about the world's vertical axis by `degrees`, clockwise seen from above
    /// like the Sun's azimuth: 90 turns its north (-Z) side to face east (+X).
    #[must_use]
    pub fn yaw_degrees(self, degrees: f32) -> Self {
        self.rotate_degrees([0.0, 1.0, 0.0], -degrees)
    }

    /// Turn the mesh about a world `axis` through its origin by `degrees`, counterclockwise
    /// looking down the axis toward the origin (the right-hand rule). Turns apply in call order.
    ///
    /// Any finite, nonzero axis names its turn, however long or short; the turned transform is
    /// then checked like any other. A zero or non-finite axis, or a
    /// non-finite angle (for example one computed from a bad input), names no turn: the
    /// placement keeps its transform and [`crate::CookedAssets::new`] refuses it with a
    /// [`PlacementError`] naming the turn.
    #[must_use]
    pub fn rotate_degrees(mut self, axis: [f32; 3], degrees: f32) -> Self {
        if self.refused_turn.is_some() {
            return self;
        }
        // In f64, where the squared length of any finite f32 axis neither overflows nor
        // underflows to zero, so any finite nonzero axis names its direction.
        let wide_axis = axis.map(f64::from);
        let length = dot(wide_axis, wide_axis).sqrt();
        if !(above(length, 0.0) && length.is_finite() && degrees.is_finite()) {
            self.refused_turn = Some((axis, degrees));
            return self;
        }
        let unit = wide_axis.map(|value| value / length);
        let (sin, cos) = f64::from(degrees).to_radians().sin_cos();
        // Rodrigues' rotation of each linear column: v cos + (u × v) sin + u (u · v)(1 - cos).
        for column in &mut self.transform[..3] {
            let v = [column[0], column[1], column[2]].map(f64::from);
            let across = cross(unit, v);
            let along = dot(unit, v) * (1.0 - cos);
            for row in 0..3 {
                column[row] = narrow(v[row] * cos + across[row] * sin + unit[row] * along);
            }
        }
        self
    }

    /// Multiply the mesh's size by `factor`, about its origin. Scale is uniform: to stretch one
    /// axis, reshape the mesh in its source file.
    #[must_use]
    pub fn scale(mut self, factor: f32) -> Self {
        for column in &mut self.transform[..3] {
            for value in &mut column[..3] {
                *value *= factor;
            }
        }
        self
    }

    /// Rest the mesh's lowest point at the placement's height; see [`Self::rest_on_ground`].
    #[must_use]
    pub fn on_ground(mut self) -> Self {
        self.rest_on_ground = true;
        self
    }

    /// Set the material factors.
    #[must_use]
    pub fn material(mut self, material: CookedMaterial) -> Self {
        self.material = material;
        self
    }

    /// Refuse a transform that is not rigid with one uniform positive scale.
    pub(crate) fn check(&self, index: usize) -> Result<(), PlacementError> {
        let refuse = |fault| {
            Err(PlacementError {
                index,
                mesh: self.mesh.clone(),
                fault,
            })
        };
        if let Some((axis, degrees)) = self.refused_turn {
            return refuse(Fault::NoTurn(axis, degrees));
        }
        let matrix = glam::Mat4::from_cols_array_2d(&self.transform);
        if !matrix.is_finite() {
            return refuse(Fault::NotFinite);
        }
        if matrix.row(3) != glam::Vec4::W {
            return refuse(Fault::Projective);
        }
        // Every product and sum of finite `f32` values is finite in `f64`, so these lengths, dot
        // products and the determinant cannot overflow into infinity or NaN and slip past a
        // comparison. Each test below states what is accepted, so a NaN would be refused.
        let axes = [matrix.x_axis, matrix.y_axis, matrix.z_axis].map(|axis| wide(axis.truncate()));
        let lengths = axes.map(|axis| dot(axis, axis).sqrt());
        let largest = lengths.into_iter().fold(0.0_f64, f64::max);
        let smallest = lengths.into_iter().fold(f64::INFINITY, f64::min);
        let scales = lengths.map(reported);
        if !above(smallest, f64::from(f32::EPSILON)) {
            return refuse(Fault::Flattened(scales));
        }
        if !at_most(largest - smallest, RIGID_TOLERANCE * largest) {
            return refuse(Fault::Stretched(scales));
        }
        let skew = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .map(|(a, b)| dot(axes[a], axes[b]).abs())
            .fold(0.0_f64, f64::max);
        if !at_most(skew, RIGID_TOLERANCE * largest * largest) {
            return refuse(Fault::Sheared);
        }
        if !above(dot(cross(axes[0], axes[1]), axes[2]), 0.0) {
            return refuse(Fault::Mirrored);
        }
        // Compared at the precision the scale was written in, so `.scale(1e-4)` is accepted.
        if !SCALE_RANGE.contains(&reported(largest)) {
            return refuse(Fault::ScaleOutOfRange(reported(largest)));
        }
        let origin = matrix.w_axis.truncate();
        if !wide(origin).iter().all(|value| value.abs() <= MAX_DISTANCE) {
            return refuse(Fault::TooFar(origin.to_array()));
        }
        // Within those bounds the inverse is finite; this guards the derived transform itself.
        if !matrix.inverse().is_finite() {
            return refuse(Fault::NotInvertible);
        }
        Ok(())
    }
}

/// A mesh placement whose transform cooked meshes cannot draw. Its message names the placement,
/// the cause and the fix.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacementError {
    index: usize,
    mesh: String,
    fault: Fault,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Fault {
    NoTurn([f32; 3], f32),
    NotFinite,
    Projective,
    Flattened([f32; 3]),
    Stretched([f32; 3]),
    Sheared,
    Mirrored,
    ScaleOutOfRange(f32),
    TooFar([f32; 3]),
    NotInvertible,
}

/// `value > limit`, false for NaN: the check accepts only what it can order.
fn above(value: f64, limit: f64) -> bool {
    value.partial_cmp(&limit) == Some(std::cmp::Ordering::Greater)
}

/// `value <= limit`, false for NaN.
fn at_most(value: f64, limit: f64) -> bool {
    value
        .partial_cmp(&limit)
        .is_some_and(std::cmp::Ordering::is_le)
}

fn wide(v: glam::Vec3) -> [f64; 3] {
    v.to_array().map(f64::from)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A transform value computed in f64, stored at the transform's own precision.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the cast stores at the transform's f32 precision; a component turned beyond f32's \
              range becomes infinite, and the checked placement refuses it as not finite"
)]
fn narrow(value: f64) -> f32 {
    value as f32
}

/// A length for a message, at the precision the transform was written in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the lengths of finite f32 axes are reported back in f32, the transform's own precision"
)]
fn reported(length: f64) -> f32 {
    length as f32
}

impl fmt::Display for PlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { index, mesh, fault } = self;
        write!(f, "mesh placement {index} ({mesh:?}) ")?;
        match fault {
            Fault::NoTurn(axis, degrees) => {
                return write!(
                    f,
                    "turns {degrees}° about the axis {axis:?}, which names no rotation; give \
                     rotate_degrees a nonzero, finite axis and a finite angle (yaw_degrees a \
                     finite angle)"
                );
            }
            Fault::NotFinite => f.write_str("has a NaN or infinite transform value"),
            Fault::Projective => f.write_str("has a bottom transform row other than 0, 0, 0, 1"),
            Fault::Flattened([x, y, z]) => write!(f, "scales its axes to zero ({x}, {y}, {z})"),
            Fault::Stretched([x, y, z]) => {
                write!(f, "scales its axes unevenly (X {x}, Y {y}, Z {z})")
            }
            Fault::Sheared => f.write_str("shears: its axes are not at right angles"),
            Fault::Mirrored => f.write_str("mirrors the mesh (a negative scale)"),
            Fault::ScaleOutOfRange(scale) => {
                return write!(
                    f,
                    "scales the mesh by {scale}, outside the supported {}–{}; resize the mesh \
                     in its source file so that its scale stays in that range",
                    SCALE_RANGE.start(),
                    SCALE_RANGE.end()
                );
            }
            Fault::TooFar([x, y, z]) => {
                return write!(
                    f,
                    "places its origin at ({x}, {y}, {z}), more than {MAX_DISTANCE} m from the \
                     world origin on an axis, beyond the supported range; move the scene nearer \
                     the origin, where positions keep their precision"
                );
            }
            Fault::NotInvertible => f.write_str("has a transform with no representable inverse"),
        }?;
        f.write_str(
            "; cooked meshes need a rigid transform with one uniform positive scale, because \
             normals and bounds are transformed by it. Build it with MeshPlacement::new(..)\
             .rotate_degrees/.yaw_degrees/.scale/.at, and reshape or mirror the mesh in its \
             source file instead",
        )
    }
}

impl std::error::Error for PlacementError {}

/// The placement's model matrix, lifted or lowered onto the ground when it asks to rest there.
/// `rest_points` are the mesh's own vertices, or its bounding-box corners for a large mesh.
pub(crate) fn placed_model(placement: &MeshPlacement, rest_points: &[glam::Vec3]) -> glam::Mat4 {
    let mut model = glam::Mat4::from_cols_array_2d(&placement.transform);
    if placement.rest_on_ground {
        let lowest = rest_points
            .iter()
            .map(|&point| model.transform_point3(point).y)
            .fold(f32::INFINITY, f32::min);
        if lowest.is_finite() {
            model.w_axis.y += model.w_axis.y - lowest;
        }
    }
    model
}

#[cfg(test)]
mod tests;
