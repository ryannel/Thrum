fn multiple_at(h: f32, sun: vec3<f32>, bank: u32) -> vec4<f32> {
    var luminance = vec4(0.0);
    var feedback = vec4(0.0);
    for(var ray=0u;ray<32u;ray++) {
        let mu = 1.0-2.0*(f32(ray)+0.5)/32.0;
        let phi = f32(ray)*2.39996323;
        let radial = sqrt(1.0-mu*mu);
        let direction = vec3(radial*cos(phi),mu,radial*sin(phi));
        let ground = ground_distance(h,mu);
        var distance = top_distance(h,mu);
        if ground>=0.0 { distance=ground; }
        let dt = distance/20.0;
        var throughput = vec4(1.0);
        for(var step=0u;step<20u;step++) {
            let p=vec3(0.0,h,0.0)+direction*(f32(step)+0.5)*dt;
            let ah=altitude(p);
            let densities=medium_densities(ah);
            let scatter=RAYLEIGH_BETA[bank]*densities.x+MIE_SCATTERING*densities.y;
            let segment=integrate_segment(scatter,extinction(densities,bank),dt);
            let direct=spectral_sun(ah,local_sun_mu(p,sun),bank,transmittance,lut_sampler);
            luminance+=throughput*segment.radiance*direct/(4.0*PI*32.0);
            feedback+=throughput*segment.radiance/32.0;
            throughput*=segment.transmission;
        }
        if ground>=0.0 {
            let p=vec3(0.0,h,0.0)+direction*distance;
            let n=normalize(p+vec3(0.0,R,0.0));
            let receiver=p+n*0.1;
            let direct=spectral_sun(altitude(receiver),local_sun_mu(receiver,sun),bank,transmittance,lut_sampler);
            luminance+=throughput*direct*max(0.0,dot(n,sun))*(0.1/(PI*32.0));
            feedback+=throughput*(0.1/32.0);
        }
    }
    return luminance/max(vec4(0.001),vec4(1.0)-feedback);
}
@compute @workgroup_size(LUT_WORKGROUP_EDGE, LUT_WORKGROUP_EDGE)
fn multiple_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = plane_size(textureDimensions(output_lut),SPECTRAL_PLANES);
    if any(id.xy>=size) { return; }
    let uv = (vec2<f32>(id.xy)+0.5)/vec2<f32>(size);
    let h = uv.y*H;
    let sun_mu = uv.x*2.0-1.0;
    let sun = vec3(sqrt(max(0.0,1.0-sun_mu*sun_mu)),sun_mu,0.0);
    for(var bank=0u;bank<SPECTRAL_PLANES;bank++) {
        let pos=vec2<i32>(id.xy+vec2(0u,bank*size.y));
        textureStore(output_lut,pos,multiple_at(h,sun,bank)*MULTIPLE_LUT_GAIN);
    }
}
