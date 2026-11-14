// Independent box reference: sum 32 distinct raster offsets in f32 scene-linear
// storage, then use the same fp16 pre-tone seam and sole SDR stage as TAAU.
struct Params {
    inverse: mat4x4<f32>, previous_view: mat4x4<f32>, view: mat4x4<f32>,
    state: vec4<f32>, jitter: vec4<f32>,
}
@group(0) @binding(0) var colour: texture_2d<f32>;
@group(0) @binding(4) var history: texture_2d<f32>;
@group(0) @binding(6) var output: texture_storage_2d<rgba16float,write>;
@group(0) @binding(7) var sum: texture_storage_2d<rgba32float,write>;
@group(0) @binding(9) var<uniform> params: Params;
@compute @workgroup_size(8,8)
fn accumulate(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= textureDimensions(colour)) { return; }
    let at = vec2<i32>(id.xy);
    var total = textureLoad(colour,at,0).rgb;
    if params.jitter.z > 0.0 { total += textureLoad(history,at,0).rgb; }
    textureStore(sum,at,vec4(total,1.0));
    textureStore(output,at,vec4(total/(params.jitter.z+1.0),1.0));
}
