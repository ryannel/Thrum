struct Pixel { colour:vec4<f32>, depth:vec4<f32> }
struct Observation { record:SecondaryViewRecord, pixels:array<Pixel> }
@group(2) @binding(0) var colour:texture_2d<f32>;
@group(2) @binding(1) var depth:texture_depth_2d;
@group(2) @binding(2) var<uniform> record:SecondaryViewRecord;
@group(2) @binding(3) var<storage,read_write> observation:Observation;
@compute @workgroup_size(8,8)
fn observe(@builtin(global_invocation_id) id:vec3<u32>) {
    let extent=vec2<u32>(record.extent.zw);
    if any(id.xy>=extent) {return;}
    if id.x==0u && id.y==0u { observation.record=record; }
    let index=id.y*extent.x+id.x;
    observation.pixels[index]=Pixel(textureLoad(colour,vec2<i32>(id.xy),0),vec4(textureLoad(depth,vec2<i32>(id.xy),0),0.0,0.0,0.0));
}
