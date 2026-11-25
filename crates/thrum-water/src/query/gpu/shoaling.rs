//! A small immutable dimensionless shoaling table for the surf query.
use wgpu::util::DeviceExt;

const ROWS: usize = 256;
const MIN_LOG_DEPTH: f64 = -20.0;
const MAX_LOG_DEPTH: f64 = 5.0;
const SCALE: f64 = (4 * ROWS - 1) as f64 / (MAX_LOG_DEPTH - MIN_LOG_DEPTH);
pub(super) const BYTES: u64 = (4 * ROWS * size_of::<f32>()) as u64;

pub(super) fn source() -> String {
    format!(
        "const QUERY_SHOALING_ROWS = {ROWS}u;\nconst QUERY_SHOALING_MIN = {MIN_LOG_DEPTH:.1};\n        const QUERY_SHOALING_SCALE = {SCALE:.9};\n"
    ) + include_str!("../shoaling.wgsl")
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the query evaluates its shader in f32"
)]
pub(super) fn buffer(device: &wgpu::Device) -> wgpu::Buffer {
    let table: [f32; 4 * ROWS] = std::array::from_fn(|i| {
        let depth = (MIN_LOG_DEPTH + i as f64 / SCALE).exp2();
        thrum_water_model::shoaling(1.0, depth) as f32
    });
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("water query shoaling"),
        contents: bytemuck::cast_slice(&table),
        usage: wgpu::BufferUsages::UNIFORM,
    })
}
