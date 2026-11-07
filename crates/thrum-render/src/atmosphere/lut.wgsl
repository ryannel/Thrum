@group(0) @binding(1) var transmittance: texture_2d<f32>;
@group(0) @binding(2) var multiple_scattering: texture_2d<f32>;
@group(0) @binding(3) var output_lut: texture_storage_2d<rgba16float,write>;
@group(0) @binding(4) var lut_sampler: sampler;
@group(0) @binding(5) var ground_irradiance: texture_2d<f32>;
@compute @workgroup_size(8,8)
fn transmittance_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = plane_size(textureDimensions(output_lut),TRANS_PLANES);
    if any(id.xy>=size) { return; }
    let params = trans_params((vec2<f32>(id.xy)+0.5)/vec2<f32>(size));
    let h = params.x;
    let mu = params.y;
    let direction = vec3(sqrt(max(0.0,1.0-mu*mu)),mu,0.0);
    let step = top_distance(h,mu)/64.0;
    var columns = vec3(0.0);
    for(var i=0u;i<64u;i++) {
        columns += medium_densities(altitude(vec3(0.0,h,0.0)+direction*(f32(i)+0.5)*step))*step;
    }
    let lo=exp(-extinction(columns,0u));
    let hi=exp(-extinction(columns,1u));
    textureStore(output_lut,vec2<i32>(id.xy),vec4(project_spectrum(lo,hi),1.0));
    textureStore(output_lut,vec2<i32>(id.xy+vec2(0u,size.y)),lo);
    textureStore(output_lut,vec2<i32>(id.xy+vec2(0u,2u*size.y)),hi);
}
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
            let scatter=RAYLEIGH_BETA[bank]*densities.x+3.996e-6*densities.y;
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
@compute @workgroup_size(8,8)
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
// Shared unit-Sun volume transport. No terminal ground, disk or unscattered beam;
// GroundE integrates this same body upward from h=0 without depending on Sky.
fn sky_transport(h: f32, direction: vec3<f32>, sun: vec3<f32>, distance: f32) -> SpectralRay {
    let phase=scattering_phase(dot(direction,sun));
    var ray=empty_ray();
    for(var step=0u;step<48u;step++) {
        let a=f32(step)/48.0;
        let b=f32(step+1u)/48.0;
        let dt=(b*b-a*a)*distance;
        let t=0.5*(a*a+b*b)*distance;
        let p=vec3(0.0,h,0.0)+direction*t;
        let ah=altitude(p);
        ray=lit_step(ray,medium_densities(ah),ah,local_sun_mu(p,sun),phase,dt,
            transmittance,multiple_scattering,lut_sampler);
    }
    return ray;
}
fn ground_radiance(h: f32, mu: f32, bank: u32) -> vec4<f32> {
    let direct=spectral_sun(h,mu,bank,transmittance,lut_sampler)*max(0.0,mu);
    let indirect=spectral_lut(ground_irradiance_uv(mu),bank,ground_irradiance,lut_sampler)/GROUND_IRRADIANCE_GAIN;
    return (direct+indirect)*(0.1/PI);
}

@compute @workgroup_size(8,8)
fn sky_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size=textureDimensions(output_lut);
    if any(id.xy>=size) { return; }
    let ray=sky_view_ray((vec2<f32>(id.xy)+0.5)/vec2<f32>(size),id.y);
    let sun=atmosphere.sun_lux.xyz;
    let direction=ray.direction;
    let h=atmosphere.camera_altitude.w;
    let distance=ray.distance;
    let transport=sky_transport(h,direction,sun,distance);
    var lo=transport.lo.radiance;
    var hi=transport.hi.radiance;
    if ray.terminal_ground!=0u {
        let p=vec3(0.0,h,0.0)+direction*distance;
        let n=normalize(p+vec3(0.0,R,0.0));
        let receiver_h=altitude(p+n*0.1);
        let mu=dot(n,sun);
        lo+=transport.lo.transmission*ground_radiance(receiver_h,mu,0u);
        hi+=transport.hi.transmission*ground_radiance(receiver_h,mu,1u);
    }
    textureStore(output_lut,vec2<i32>(id.xy),vec4(project_spectrum(lo,hi)*SKY_LUT_GAIN,1.0));
}
