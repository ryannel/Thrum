//! Geometry, bed and shelter used by the opt-in valley patch.

use thrum_rain_model::{Canopy, Ground, Profile, RainError, Shelter, Tree};
use thrum_water::wet_ground::{GroundCell, HollowsGround};
use thrum_water_model::{Bed, NO_HOLLOW};

const ORIGIN: [f64; 2] = [-4.0, 2.0];
const SIZE: [usize; 2] = [72, 56];
const SPACING: f64 = 0.125;
const PATCH: &[u8] = include_bytes!("../../assets/rain/rain-patch.bin");

// The centres lie on each cell's shared triangle edge. Read the same f32
// heights the cooker gets from the patch mesh.
pub(super) fn bed() -> Result<Ground, String> {
    let [width, height] = SIZE;
    let vertex = |x, z| {
        let offset = 12 * (z * (width + 1) + x) + 4;
        f64::from(f32::from_le_bytes(
            PATCH[offset..offset + 4]
                .try_into()
                .expect("authored patch height"),
        ))
    };
    let values = (0..height)
        .flat_map(|z| (0..width).map(move |x| f64::midpoint(vertex(x + 1, z), vertex(x, z + 1))))
        .collect();
    Ground::new(ORIGIN, SPACING, SIZE, values).map_err(|error| error.to_string())
}

pub(super) fn map_bed(tree: &Tree) -> Result<Bed, String> {
    Bed::new(
        ORIGIN.map(|coordinate| coordinate + SPACING / 2.0),
        SPACING,
        SIZE,
        tree.ground().heights().to_vec(),
    )
    .map_err(|error| format!("{error:?}"))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "prepared ground lattice and heights for WGSL"
)]
pub(super) fn wetness_grid(tree: &Tree) -> HollowsGround {
    let cells = tree
        .ground()
        .heights()
        .iter()
        .zip(tree.labels())
        .map(|(&bed, &hollow)| GroundCell {
            // The authored skirt intersects the original floor. An
            // unlabelled cell uses the open store on both meshes, so the
            // separate film pass cannot paint the lattice's outer rectangle.
            bed: if hollow == NO_HOLLOW {
                bed as f32 - 1.0
            } else {
                bed as f32
            },
            hollow,
            pit: if hollow == NO_HOLLOW {
                bed
            } else {
                tree.node(hollow).lower
            } as f32,
            class: 0,
        })
        .collect();
    HollowsGround {
        origin: ORIGIN.map(|v| v as f32),
        spacing: SPACING as f32,
        size: SIZE.map(|v| v as u32),
        cells,
    }
}

// The exposed shelf and the first three stops fall within this prepared
// shelter grid. More distant ground uses the open store.
pub(super) fn canopy(profile: &Profile) -> Result<Canopy, RainError> {
    const LOWER: [f64; 2] = [-56.0, -210.0];
    const UPPER: [f64; 2] = [30.0, 22.0];
    const SIZE: [usize; 2] = [344, 928];
    const BOULDER: &[u8] = include_bytes!("../../assets/exit/exit-boulder.bin");
    const STANDING: &[u8] = include_bytes!("../../assets/exit/exit-standing-rock.bin");
    const ROCK: &[u8] = include_bytes!("../../assets/showcase/rock.bin");
    let mut triangles = Vec::new();
    let mut add = |bytes, count, position: [f64; 3], angle: f32, scale: f32| {
        let (sin, cos) = angle.sin_cos();
        let (s, c, scale) = (f64::from(sin), f64::from(cos), f64::from(scale));
        for local in rock_triangles(bytes, count) {
            let triangle = local.map(|[x, y, z]| {
                [
                    position[0] + (x * c + z * s) * scale,
                    position[1] + y * scale,
                    position[2] + (z * c - x * s) * scale,
                ]
            });
            let [a, b, c] = triangle;
            // Keep upward-facing facets. The lower hull lies beneath the
            // sheltered underside and would make exposed sides read as dry.
            if (b[2] - a[2]) * (c[0] - a[0]) > (b[0] - a[0]) * (c[2] - a[2]) {
                triangles.push(triangle);
            }
        }
    };
    add(ROCK, 114, [3.8, 240.645_977_762_758_75, -7.0], 0.0, 1.0);
    for prop in &crate::exit_scene::layout().entities {
        let position = prop.position();
        if position[0] < LOWER[0] - 6.0
            || position[0] > UPPER[0] + 6.0
            || position[2] < LOWER[1] - 6.0
            || position[2] > UPPER[1] + 6.0
        {
            continue;
        }
        let (bytes, count) = match prop.mesh_row() {
            0 => (BOULDER, 26),
            3 => (STANDING, 26),
            _ => continue,
        };
        let (angle, scale) = prop.orientation();
        add(bytes, count, position, angle, scale);
    }
    Canopy::prepare(
        LOWER,
        SIZE,
        &[Shelter {
            name: "valley rock overhangs".to_owned(),
            triangles,
            share: 0.07,
        }],
        profile,
    )
}

fn rock_triangles(bytes: &[u8], vertices: usize) -> impl Iterator<Item = [[f64; 3]; 3]> + '_ {
    bytes[vertices * 32..]
        .as_chunks::<6>()
        .0
        .iter()
        .map(move |triangle| {
            std::array::from_fn(|i| {
                let index = usize::from(u16::from_le_bytes([triangle[i * 2], triangle[i * 2 + 1]]));
                std::array::from_fn(|axis| {
                    let start = 12 * index + 4 * axis;
                    f64::from(f32::from_le_bytes(
                        bytes[start..start + 4].try_into().expect("rock position"),
                    ))
                })
            })
        })
}

#[cfg(all(target_os = "macos", feature = "proving"))]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the cooked mesh inputs are already f32"
)]
pub(super) fn boulder_samples() -> impl Iterator<Item = ([f32; 3], [f32; 3])> {
    rock_triangles(include_bytes!("../../assets/exit/exit-boulder.bin"), 26).filter_map(|t| {
        let [a, b, c] = t.map(|p| glam::Vec3::from_array(p.map(|v| v as f32)));
        let normal = (b - a).cross(c - a).normalize_or_zero();
        (normal.y > 0.0).then(|| (((a + b + c) / 3.0).to_array(), normal.to_array()))
    })
}
