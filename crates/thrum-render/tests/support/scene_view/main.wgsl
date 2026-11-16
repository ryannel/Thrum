struct Pixel { colour: vec4<f32>, depth: vec4<f32> }
struct Observation { reserved: array<vec4<f32>,20>, pixels: array<Pixel> }
@group(1) @binding(0) var depth: texture_depth_2d;
@group(1) @binding(1) var colour: texture_2d<f32>;
@group(2) @binding(0) var<storage,read_write> observation: Observation;
@compute @workgroup_size(8,8)
fn observe(@builtin(global_invocation_id) id: vec3<u32>) {
    let extent = textureDimensions(colour);
    if any(id.xy >= extent) { return; }
    observation.pixels[id.y*extent.x+id.x] = Pixel(textureLoad(colour,vec2<i32>(id.xy),0),vec4(textureLoad(depth,vec2<i32>(id.xy),0),0.0,0.0,0.0));
}
