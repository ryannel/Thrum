@group(0) @binding(0) var<uniform> caster_view: mat4x4<f32>;

@vertex fn floating_shadow_vertex(
    @location(0) position: vec3<f32>,
    @location(3) model0: vec4<f32>,
    @location(4) model1: vec4<f32>,
    @location(5) model2: vec4<f32>,
    @location(6) model3: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4(model0, model1, model2, model3);
    return caster_view * model * vec4(position, 1.0);
}
