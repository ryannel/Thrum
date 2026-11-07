//! Where a cooked mesh stands: the placement, its builder and the rigid-transform check.

use std::fmt;

use crate::CookedMaterial;

/// Relative tolerance of the rigid-transform check: axis lengths and right angles may differ
/// by this fraction of the scale, which absorbs `f32` rounding in hand-built matrices.
const RIGID_TOLERANCE: f32 = 1e-3;

/// A cooked mesh, its base-colour texture and where it stands in the world.
///
/// Build one with [`MeshPlacement::new`] and the placement methods, which always produce a
/// valid transform:
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
/// it. [`crate::CookedAssets::new`] refuses any other transform with a [`PlacementError`]
/// naming the placement, the cause and the fix.
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
    #[must_use]
    pub fn rotate_degrees(mut self, axis: [f32; 3], degrees: f32) -> Self {
        let turn =
            glam::Mat4::from_axis_angle(glam::Vec3::from(axis).normalize(), degrees.to_radians());
        let position = self.transform[3];
        let linear = turn * glam::Mat4::from_cols_array_2d(&self.transform);
        self.transform = linear.to_cols_array_2d();
        self.transform[3] = position;
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
        let matrix = glam::Mat4::from_cols_array_2d(&self.transform);
        if !matrix.is_finite() {
            return refuse(Fault::NotFinite);
        }
        if matrix.row(3) != glam::Vec4::W {
            return refuse(Fault::Projective);
        }
        let axes = [matrix.x_axis, matrix.y_axis, matrix.z_axis].map(glam::Vec4::truncate);
        let scales = axes.map(glam::Vec3::length);
        let largest = scales.into_iter().fold(0.0_f32, f32::max);
        if scales.iter().any(|scale| *scale <= f32::EPSILON) {
            return refuse(Fault::Flattened(scales));
        }
        if scales
            .iter()
            .any(|scale| (scale - largest).abs() > RIGID_TOLERANCE * largest)
        {
            return refuse(Fault::Stretched(scales));
        }
        let skew = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .map(|(a, b)| axes[a].dot(axes[b]).abs())
            .fold(0.0_f32, f32::max);
        if skew > RIGID_TOLERANCE * largest * largest {
            return refuse(Fault::Sheared);
        }
        if axes[0].cross(axes[1]).dot(axes[2]) < 0.0 {
            return refuse(Fault::Mirrored);
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
    NotFinite,
    Projective,
    Flattened([f32; 3]),
    Stretched([f32; 3]),
    Sheared,
    Mirrored,
}

impl fmt::Display for PlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { index, mesh, fault } = self;
        write!(f, "mesh placement {index} ({mesh:?}) ")?;
        match fault {
            Fault::NotFinite => f.write_str("has a NaN or infinite transform value"),
            Fault::Projective => f.write_str("has a bottom transform row other than 0, 0, 0, 1"),
            Fault::Flattened([x, y, z]) => write!(f, "scales its axes to zero ({x}, {y}, {z})"),
            Fault::Stretched([x, y, z]) => {
                write!(f, "scales its axes unevenly (X {x}, Y {y}, Z {z})")
            }
            Fault::Sheared => f.write_str("shears: its axes are not at right angles"),
            Fault::Mirrored => f.write_str("mirrors the mesh (a negative scale)"),
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
