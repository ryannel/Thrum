struct SkyMotion {
    camera_delta: mat4x4<f32>,
    current: mat4x4<f32>,
    extent: vec4<f32>,
    solar_delta: mat4x4<f32>,
}
@group(0) @binding(5) var<uniform> sky_motion: SkyMotion;
fn infinity_motion(direction: vec3<f32>, disk: f32) -> vec2<f32> {
    let current = sky_motion.current * vec4(direction, 0.0);
    // Evaluate the projective difference directly. Equal matrices have an exact
    // zero delta, avoiding cancellation noise from two separately divided clips.
    let delta = select(sky_motion.camera_delta * vec4(direction, 0.0),
        sky_motion.solar_delta * vec4(direction, 0.0), disk > 0.0);
    let previous_w = current.w + delta.w;
    if current.w <= 0.0 || previous_w <= 0.0 { return vec2(MAX_FINITE_HALF); }
    let difference = (delta.xy * current.w - current.xy * delta.w)
        / (current.w * previous_w);
    return difference * vec2(0.5, -0.5) * sky_motion.extent.xy;
}
@group(0) @binding(1) var transmittance: texture_2d<f32>;
@group(0) @binding(2) var sky: texture_2d<f32>;
@group(0) @binding(3) var lut_sampler: sampler;
@group(0) @binding(4) var<uniform> sky_exposure: vec4<f32>;
struct VertexOutput { @builtin(position) position: vec4<f32>, @location(0) ndc: vec2<f32> }
@vertex fn vertex(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return VertexOutput(vec4(positions[index],0.0,1.0),positions[index]);
}
struct SkyOutput { @location(0) colour: vec4<f32>, @location(1) motion: vec2<f32>, @location(2) reactive: f32 }
