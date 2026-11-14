struct View { raster:mat4x4<f32>, previous:mat4x4<f32>, camera:vec4<f32>, exposure:vec4<f32>, unjittered:mat4x4<f32>, extra:vec4<u32> }
struct Aux { inverse:mat4x4<f32>, extent:vec4<f32>, near_far:vec4<f32>, time:vec4<f32>, flags:vec4<u32>, rectangle:vec4<f32> }
@group(0) @binding(0) var<uniform> view:View;
@group(0) @binding(1) var<uniform> aux:Aux;
@group(1) @binding(0) var depth:texture_depth_2d;
struct SlabParams {offsets:vec4<f32>, origin_scale:vec4<f32>}
@group(2) @binding(0) var<uniform> params:SlabParams;
@group(2) @binding(1) var field:texture_2d<u32>;
@group(2) @binding(2) var volume:texture_3d<u32>;
@group(2) @binding(3) var layers:texture_2d_array<u32>;
@group(2) @binding(4) var cube:texture_2d_array<u32>;
struct Vertex { @builtin(position) position:vec4<f32>, @location(0) current:vec4<f32>, @location(1) previous:vec4<f32> }
@vertex fn vertex(@builtin(vertex_index) i:u32)->Vertex {
    let corners=array<vec2<f32>,6>(vec2(-4.0,-1.0),vec2(4.0,-1.0),vec2(-4.0,5.0),vec2(-4.0,5.0),vec2(4.0,-1.0),vec2(4.0,5.0));
    let p=vec4(vec3(corners[i]*params.origin_scale.w,0.0)+params.origin_scale.xyz+vec3(params.offsets.x,0.0,0.0),1.0);
    let old=vec4(vec3(corners[i]*params.origin_scale.w,0.0)+params.origin_scale.xyz+vec3(params.offsets.y,0.0,0.0),1.0);
    return Vertex(view.raster*p,view.unjittered*p,view.previous*old);
}
struct Output { @location(0) colour:vec4<f32>, @location(1) motion:vec2<f32>, @location(2) reactive:f32 }
@fragment fn fragment(v:Vertex)->Output {
    let at=vec2<i32>(v.position.xy);
    if(textureLoad(depth,at,0)>v.position.z){discard;}
    let q=at%vec2(32);
    let value=textureLoad(field,q,0).x+textureLoad(volume,vec3(q%vec2(8),0),0).x+textureLoad(layers,q%vec2(2),0,2).x+textureLoad(cube,q%vec2(8),0,0).x;
    let tint=vec3(0.002,0.008,0.01)*f32(value%17u)/17.0;
    var motion=vec2(65504.0);
    if(params.offsets.z>0.0 && v.current.w>0.0 && v.previous.w>0.0){motion=(v.previous.xy/v.previous.w-v.current.xy/v.current.w)*vec2(0.5,-0.5)*aux.extent.xy;}
    return Output(colour(at,tint),motion,select(1.0,0.35,params.offsets.z>0.0));
}
