@group(1) @binding(0) var depth:texture_depth_2d;
struct Aux {inverse:mat4x4<f32>,extent:vec4<f32>,near_far:vec4<f32>,time:vec4<f32>,flags:vec4<u32>,rectangle:vec4<f32>}
@group(0) @binding(1) var<uniform> view:Aux;
@group(2) @binding(0) var fresh:texture_storage_2d<r32uint,write>;
@compute @workgroup_size(4) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    let xs=array<i32,4>(0,25,39,63);
    let at=vec2(min(xs[id.x],i32(view.extent.x)-1),i32(view.extent.y)/2);
    textureStore(fresh,vec2<i32>(i32(id.x),0),vec4(u32(textureLoad(depth,at,0)>0.0)));
}
