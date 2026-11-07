@group(0) @binding(0) var<uniform> light_view_projection: mat4x4<f32>;

@vertex fn vertex(
    @location(0) position: vec3<f32>,
    @location(3) model_x: vec4<f32>,
    @location(4) model_y: vec4<f32>,
    @location(5) model_z: vec4<f32>,
    @location(6) model_w: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4(model_x, model_y, model_z, model_w);
    return light_view_projection * (model * vec4(position, 1.0));
}
