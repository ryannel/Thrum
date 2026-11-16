@group(1) @binding(1) var scene_colour:texture_2d<f32>;
@group(2) @binding(0) var<storage,read_write> stored:array<vec4<f32>>;
@group(2) @binding(1) var view_colour:texture_2d<f32>;
@compute @workgroup_size(8,8)
fn copy_main(@builtin(global_invocation_id) id:vec3<u32>) {
    let index=id.y*256u+id.x;
    if index<32u*32u*33u*2u {stored[index]=textureLoad(scene_colour,vec2<i32>(id.xy),0);}
}
@compute @workgroup_size(8,8)
fn copy_view(@builtin(global_invocation_id) id:vec3<u32>) {
    let index=id.y*256u+id.x;
    if index<32u*32u*33u*2u {stored[index]=textureLoad(view_colour,vec2<i32>(id.xy),0);}
}
