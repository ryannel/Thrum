@vertex fn probe_vertex(@builtin(vertex_index) index:u32)->@builtin(position) vec4<f32> {
    let positions=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(positions[index],0.5,1.0);
}
fn air_texel(pixel:vec4<f32>)->vec4<f32> {
    let xy=vec2<u32>(pixel.xy);
    let index=xy.y*256u+xy.x;
    if index>=32u*32u*33u*2u {return vec4(0.0);}
    let local=index%(32u*32u*33u);
    let layer=i32(local/(32u*32u));
    let coord=vec2<i32>(i32(local%32u),i32((local/32u)%32u));
    if index<32u*32u*33u {return textureLoad(aerial_scatter,coord,layer,0);}
    return textureLoad(aerial_transmittance,coord,layer,0);
}
struct ProbeOutput { @location(0) colour:vec4<f32>, @location(1) motion:vec2<f32>, @location(2) reactive:f32 }
@fragment fn probe_main(@builtin(position) pixel:vec4<f32>)->ProbeOutput {
    return ProbeOutput(air_texel(pixel),vec2(0.0),0.0);
}
@fragment fn probe_view(@builtin(position) pixel:vec4<f32>)->@location(0) vec4<f32> {
    return air_texel(pixel);
}
