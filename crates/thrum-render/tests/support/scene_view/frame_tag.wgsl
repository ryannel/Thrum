@vertex fn vertex(@builtin(vertex_index) index:u32)->@builtin(position) vec4<f32> {
    let positions=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(positions[index],0.5,1.0);
}
struct Output { @location(0) colour:vec4<f32>, @location(1) motion:vec2<f32>, @location(2) reactive:f32 }
@fragment fn fragment()->Output {return Output(vec4(0.2,0.1,0.4,1.0),vec2(0.0),1.0);}
