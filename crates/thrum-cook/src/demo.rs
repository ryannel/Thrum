//! Deterministic demo sources for tests, benchmarks and the edit-to-visible demonstration:
//! a displaced glTF grid, a checker PNG and a cloud-like density volume.

use std::path::Path;

use crate::texture::encode_volume;

/// Write `<name>.gltf` and `<name>.bin`: an `n`×`n` quad grid of side two metres whose height is
/// a smooth bump scaled by `amplitude`.
///
/// # Errors
/// I/O failure.
pub fn write_grid_gltf(
    directory: &Path,
    name: &str,
    n: u32,
    amplitude: f32,
) -> std::io::Result<()> {
    let side = n + 1;
    let mut positions = Vec::with_capacity((side * side) as usize);
    let mut uvs = Vec::with_capacity(positions.capacity());
    for row in 0..side {
        for column in 0..side {
            let (u, v) = (column as f32 / n as f32, row as f32 / n as f32);
            let (x, z) = (u * 2.0 - 1.0, v * 2.0 - 1.0);
            let height =
                amplitude * (-(x * x + z * z) * 2.0).exp() * (6.0 * x).cos() * (6.0 * z).cos();
            positions.push([x, height, z]);
            uvs.push([u, v]);
        }
    }
    let mut indices = Vec::with_capacity((n * n * 6) as usize);
    for row in 0..n {
        for column in 0..n {
            let a = row * side + column;
            indices.extend_from_slice(&[a, a + side, a + 1, a + 1, a + side, a + side + 1]);
        }
    }
    let mut bin = Vec::new();
    bin.extend_from_slice(bytemuck::cast_slice(&positions));
    let uv_offset = bin.len();
    bin.extend_from_slice(bytemuck::cast_slice(&uvs));
    let index_offset = bin.len();
    bin.extend_from_slice(bytemuck::cast_slice(&indices));
    let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in &positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    let json = format!(
        r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],
"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0,"TEXCOORD_0":1}},"indices":2}}]}}],
"buffers":[{{"uri":"{name}.bin","byteLength":{len}}}],
"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":{uv_offset}}},{{"buffer":0,"byteOffset":{uv_offset},"byteLength":{uv_len}}},{{"buffer":0,"byteOffset":{index_offset},"byteLength":{index_len}}}],
"accessors":[{{"bufferView":0,"componentType":5126,"count":{count},"type":"VEC3","min":[{min0},{min1},{min2}],"max":[{max0},{max1},{max2}]}},
{{"bufferView":1,"componentType":5126,"count":{count},"type":"VEC2"}},{{"bufferView":2,"componentType":5125,"count":{index_count},"type":"SCALAR"}}]}}"#,
        len = bin.len(),
        uv_len = index_offset - uv_offset,
        index_len = bin.len() - index_offset,
        count = positions.len(),
        index_count = indices.len(),
        min0 = min[0],
        min1 = min[1],
        min2 = min[2],
        max0 = max[0],
        max1 = max[1],
        max2 = max[2],
    );
    std::fs::write(directory.join(format!("{name}.bin")), bin)?;
    std::fs::write(directory.join(format!("{name}.gltf")), json)
}

/// Write a `size`×`size` sRGB checker PNG alternating `colour` with white.
///
/// # Errors
/// I/O or encoding failure.
pub fn write_checker_png(path: &Path, size: u32, colour: [u8; 3]) -> std::io::Result<()> {
    let image = image::RgbaImage::from_fn(size, size, |x, y| {
        if ((x / (size / 8).max(1)) + (y / (size / 8).max(1))).is_multiple_of(2) {
            image::Rgba([colour[0], colour[1], colour[2], 255])
        } else {
            image::Rgba([235, 235, 235, 255])
        }
    });
    image.save(path).map_err(std::io::Error::other)
}

/// Write a `size`³ cloud-like density volume: a soft-edged ellipsoid modulated by value noise.
///
/// # Errors
/// I/O failure.
pub fn write_cloud_volume(path: &Path, size: u32, seed: u32) -> std::io::Result<()> {
    let mut voxels = Vec::with_capacity((size * size * size) as usize);
    let s = size as f32;
    for z in 0..size {
        for y in 0..size {
            for x in 0..size {
                let p = [
                    x as f32 / s * 2.0 - 1.0,
                    y as f32 / s * 2.0 - 1.0,
                    z as f32 / s * 2.0 - 1.0,
                ];
                let shape = 1.0 - (p[0] * p[0] + 2.5 * p[1] * p[1] + p[2] * p[2]).sqrt();
                let noise = value_noise(x as f32 / 6.0, y as f32 / 6.0, z as f32 / 6.0, seed);
                let density = (shape * 1.6 + (noise - 0.5) * 0.9).clamp(0.0, 1.0);
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "density is clamped to [0, 1]"
                )]
                voxels.push((density * 255.0).round() as u8);
            }
        }
    }
    std::fs::write(path, encode_volume(size, size, size, &voxels))
}

fn lattice(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x.cast_unsigned()).wrapping_mul(0x8da6_b343)
        ^ (y.cast_unsigned()).wrapping_mul(0xd816_3841)
        ^ (z.cast_unsigned()).wrapping_mul(0xcb1a_b31f)
        ^ seed.wrapping_mul(0x1656_67b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h & 0xffff) as f32 / 65535.0
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "lattice coordinates of small demo volumes"
)]
fn value_noise(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (xi, yi, zi) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
    let (fx, fy, fz) = (
        smooth(x - x.floor()),
        smooth(y - y.floor()),
        smooth(z - z.floor()),
    );
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let corner = |dx: i32, dy: i32, dz: i32| lattice(xi + dx, yi + dy, zi + dz, seed);
    mix(
        mix(
            mix(corner(0, 0, 0), corner(1, 0, 0), fx),
            mix(corner(0, 1, 0), corner(1, 1, 0), fx),
            fy,
        ),
        mix(
            mix(corner(0, 0, 1), corner(1, 0, 1), fx),
            mix(corner(0, 1, 1), corner(1, 1, 1), fx),
            fy,
        ),
        fz,
    )
}
