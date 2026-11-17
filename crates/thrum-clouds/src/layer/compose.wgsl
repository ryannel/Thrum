struct View {raster:mat4x4<f32>,previous:mat4x4<f32>,camera:vec4<f32>,exposure:vec4<f32>,unjittered:mat4x4<f32>,extra:vec4<u32>}
struct Aux {inverse:mat4x4<f32>,extent:vec4<f32>,near_far:vec4<f32>,time:vec4<f32>,flags:vec4<u32>,rectangle:vec4<f32>}
@group(0) @binding(0) var<uniform> view:View;
@group(0) @binding(1) var<uniform> aux:Aux;
@group(1) @binding(0) var depth:texture_depth_2d;
struct CloudParams {weather:array<vec4<f32>,4>,physical:vec4<f32>,erosion:vec4<f32>,drift:vec4<f32>,heights:vec4<f32>,settings:vec4<u32>}
@group(2) @binding(0) var<uniform> cloud:CloudParams;
@group(2) @binding(1) var transport:texture_2d<f32>;
@group(2) @binding(2) var front:texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {
    let p=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));return vec4(p[i],0.0,1.0);
}
struct Output {@location(0) colour:vec4<f32>,@location(1) motion:vec2<f32>,@location(2) reactive:f32}
@fragment fn fragment(@builtin(position) position:vec4<f32>)->Output {
    let uv=position.xy/aux.extent.xy;
    let at=vec2<i32>(clamp(uv*vec2<f32>(textureDimensions(transport)),vec2(0.0),vec2<f32>(textureDimensions(transport))-1.0));
    let layer=textureLoad(transport,at,0);let front_value=textureLoad(front,at,0);
    if front_value.y<=0.0 {discard;}
    let far=aux.inverse*vec4(uv*vec2(2.0,-2.0)+vec2(-1.0,1.0),0.0,1.0);
    let ray=normalize(far.xyz/far.w-view.camera.xyz);
    let point=view.camera.xyz+ray*front_value.x*1000.0;
    let raster=view.raster*vec4(point,1.0);
    if textureLoad(depth,vec2<i32>(position.xy),0)>raster.z/raster.w {discard;}
    return Output(layer,vec2(65504.0),front_value.y);
}
