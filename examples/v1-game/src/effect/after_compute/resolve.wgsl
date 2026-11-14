@group(2) @binding(0) var fresh:texture_2d<u32>;
@group(2) @binding(1) var previous:texture_2d<u32>;
@group(2) @binding(2) var next:texture_storage_2d<r32uint,write>;
@group(2) @binding(3) var<uniform> reset:vec4<u32>;
@compute @workgroup_size(4) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    let q=vec2<i32>(i32(id.x),0);var old=0u;
    if(reset.x==0u){old=textureLoad(previous,q,0).x;}
    textureStore(next,q,vec4(old+textureLoad(fresh,q,0).x));
}
