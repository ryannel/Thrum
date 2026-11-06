// The target is Rgba8Unorm: this shader performs the sole sRGB encoding.
// This pre-AA diagnostic loads one exact fp16 texel, with no filtering/sampler.
struct Display {
    view_proj: mat4x4<f32>,
    previous_view_proj: mat4x4<f32>,
    camera_xyz_time: vec4<f32>,
    pre_exposure_inv_exposure_jitter: vec4<f32>,
}
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var<uniform> display: Display;
@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(positions[index],0.0,1.0);
}
fn srgb_encode(linear: vec3<f32>) -> vec3<f32> {
    return select(1.055*pow(linear,vec3(1.0/2.4))-vec3(0.055),12.92*linear,linear<=vec3(0.0031308));
}
@fragment fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let stored = textureLoad(scene,vec2<i32>(position.xy),0).rgb;
    // Scaling first avoids overflow for valid large gains: algebraically this is
    // C_e/(1+Y(C_e)), followed by the specified shared maximum-channel gamut map.
    let peak = max(1.0,max(stored.r,max(stored.g,stored.b)));
    let scaled = stored/peak;
    let reciprocal_scale = (display.pre_exposure_inv_exposure_jitter.x*display.pre_exposure_inv_exposure_jitter.y)/peak;
    let denominator = max(reciprocal_scale+dot(scaled,vec3(0.2126,0.7152,0.0722)),max(scaled.r,max(scaled.g,scaled.b)));
    // A black scene has zero denominator only when a subnormal reciprocal flushes.
    let mapped = scaled/max(denominator,1e-30);
    return vec4(srgb_encode(mapped),1.0);
}
