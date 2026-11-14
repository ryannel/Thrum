@group(2) @binding(0) var result:texture_2d<u32>;
@group(2) @binding(1) var output:texture_storage_2d<r32uint,write>;
@compute @workgroup_size(4) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    let q=vec2<i32>(i32(id.x),0);textureStore(output,q,vec4(textureLoad(result,q,0).x+100u));
}
