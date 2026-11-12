struct ExposureSample { values: vec4<f32>, metadata: vec4<u32> }
@group(0) @binding(0) var<uniform> exposure: ExposureSample;
@vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(p[i],0.0,1.0);
}
// Convert a retained storage-domain sample without reinterpreting its scale.
fn rescale_storage(value: f32, previous_p: f32, current_p: f32) -> f32 {
    return value * (current_p / previous_p);
}
// The inverse gain is the reciprocal of tone exposure. p belongs to this
// immutable GPU sample, never a delayed CPU estimate of the meter.
@fragment fn resolve() -> @location(0) f32 {
    return rescale_storage(1.0 / exposure.values.y, exposure.values.x, 1.0);
}
@group(0) @binding(1) var depth: texture_depth_2d;
@group(0) @binding(2) var motion: texture_2d<f32>;
@group(0) @binding(3) var reactive: texture_2d<f32>;
// A submitted diagnostic consumer of the same reconstruction inputs. RGBA16F
// retains actual motion, with reactive and reversed-Z in the first two lanes.
@fragment fn witness(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let at = vec2<i32>(p.xy);
    return vec4(textureLoad(reactive,at,0).r, textureLoad(depth,at,0), textureLoad(motion,at,0).xy);
}
