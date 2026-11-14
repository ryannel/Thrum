@group(2) @binding(0) var result:texture_2d<u32>;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {
    let p=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));return vec4(p[i],0.0,1.0);
}
struct Output {@location(0) colour:vec4<f32>,@location(1) motion:vec2<f32>,@location(2) reactive:f32}
@fragment fn fragment()->Output {
    return Output(vec4(vec3<f32>(f32(textureLoad(result,vec2(0,0),0).x),f32(textureLoad(result,vec2(1,0),0).x),f32(textureLoad(result,vec2(2,0),0).x))/16.0,1.0),vec2(65504.0),1.0);
}
