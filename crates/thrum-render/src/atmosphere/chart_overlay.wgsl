@group(0) @binding(7) var overlay_faces: texture_2d_array<f32>;
@group(0) @binding(8) var<storage, read_write> overlay_chart: ChartLighting;
@compute @workgroup_size(64)
fn classify_main(@builtin(local_invocation_index) index: u32) {
    classify_overlay(index);
}
