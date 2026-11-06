@group(0) @binding(1) var transmittance: texture_2d<f32>;
@group(0) @binding(2) var sky: texture_2d<f32>;
@group(0) @binding(3) var lut_sampler: sampler;
struct VertexOutput { @builtin(position) position: vec4<f32>, @location(0) ndc: vec2<f32> }
@vertex fn vertex(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return VertexOutput(vec4(positions[index],0.0,1.0),positions[index]);
}
struct SkyOutput { @location(0) colour: vec4<f32>, @location(1) motion: vec2<f32> }
@fragment fn fragment(input: VertexOutput) -> SkyOutput {
    let far=atmosphere.inverse_view_projection*vec4(input.ndc,0.0,1.0);
    let direction=normalize(far.xyz/far.w-atmosphere.camera_altitude.xyz);
    let sun=atmosphere.sun_lux.xyz;
    let uv=sky_uv(direction,sun);
    let size=vec2<f32>(textureDimensions(sky));
    var radiance=textureSampleLevel(sky,lut_sampler,clamp(uv,0.5/size,1.0-0.5/size),0.0).rgb;
    let angular_radius=0.00465421;
    let cosine=dot(direction,sun);
    let edge=max(fwidth(cosine),1e-8);
    let disk=smoothstep(cos(angular_radius)-edge,cos(angular_radius)+edge,cosine);
    let h=atmosphere.camera_altitude.w;
    if !blocked(h,direction.y) {
        let trans=textureSampleLevel(transmittance,lut_sampler,trans_uv(h,direction.y),0.0).rgb;
        radiance+=disk*trans/(PI*sin(angular_radius)*sin(angular_radius));
    }
    let stored=radiance*atmosphere.sun_lux.w*atmosphere.colour_pre_exposure.rgb*atmosphere.colour_pre_exposure.w;
    return SkyOutput(vec4(clamp(stored,vec3(0.0),vec3(65504.0)),1.0),vec2(0.0));
}
