@group(0) @binding(1) var transmittance: texture_2d<f32>;
@group(0) @binding(2) var multiple_scattering: texture_2d<f32>;
@group(0) @binding(3) var output_lut: texture_storage_2d<rgba16float,write>;
@group(0) @binding(4) var lut_sampler: sampler;
@group(0) @binding(5) var ground_irradiance: texture_2d<f32>;
fn sun_transmittance(p: vec3<f32>, sun: vec3<f32>) -> vec3<f32> {
    let h=altitude(p);
    let mu=local_sun_mu(p,sun);
    return sample_sun_transmittance(h,mu,transmittance,lut_sampler);
}
@compute @workgroup_size(8,8)
fn transmittance_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(output_lut);
    if any(id.xy>=size) { return; }
    let params = trans_params((vec2<f32>(id.xy)+0.5)/vec2<f32>(size));
    let h = params.x;
    let mu = params.y;
    let direction = vec3(sqrt(max(0.0,1.0-mu*mu)),mu,0.0);
    let step = top_distance(h,mu)/64.0;
    var depth = vec3(0.0);
    for(var i=0u;i<64u;i++) {
        depth += medium(altitude(vec3(0.0,h,0.0)+direction*(f32(i)+0.5)*step)).extinction*step;
    }
    textureStore(output_lut,vec2<i32>(id.xy),vec4(exp(-depth),1.0));
}
@compute @workgroup_size(8,8)
fn multiple_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(output_lut);
    if any(id.xy>=size) { return; }
    let uv = (vec2<f32>(id.xy)+0.5)/vec2<f32>(size);
    let h = uv.y*H;
    let sun_mu = uv.x*2.0-1.0;
    let sun = vec3(sqrt(max(0.0,1.0-sun_mu*sun_mu)),sun_mu,0.0);
    var luminance = vec3(0.0);
    var feedback = vec3(0.0);
    for(var ray=0u;ray<32u;ray++) {
        let mu = 1.0-2.0*(f32(ray)+0.5)/32.0;
        let phi = f32(ray)*2.39996323;
        let radial = sqrt(1.0-mu*mu);
        let direction = vec3(radial*cos(phi),mu,radial*sin(phi));
        let ground = ground_distance(h,mu);
        var distance = top_distance(h,mu);
        if ground>=0.0 { distance=ground; }
        let dt = distance/20.0;
        var throughput = vec3(1.0);
        for(var step=0u;step<20u;step++) {
            let p=vec3(0.0,h,0.0)+direction*(f32(step)+0.5)*dt;
            let air=medium(altitude(p));
            let segment=integrate_segment(air.rayleigh+air.mie,air.extinction,dt);
            luminance+=throughput*segment.radiance*sun_transmittance(p,sun)/(4.0*PI*32.0);
            feedback+=throughput*segment.radiance/32.0;
            throughput*=segment.transmission;
        }
        if ground>=0.0 {
            let p=vec3(0.0,h,0.0)+direction*distance;
            let n=normalize(p+vec3(0.0,R,0.0));
            luminance+=throughput*sun_transmittance(p+n*0.1,sun)*max(0.0,dot(n,sun))*(0.1/(PI*32.0));
            feedback+=throughput*(0.1/32.0);
        }
    }
    let multiple=luminance/max(vec3(0.001),vec3(1.0)-feedback);
    textureStore(output_lut,vec2<i32>(id.xy),vec4(multiple*MULTIPLE_LUT_GAIN,1.0));
}
// Shared unit-Sun volume transport. No terminal ground, disk or unscattered beam;
// GroundE integrates this same body upward from h=0 without depending on Sky.
fn sky_transport(h: f32, direction: vec3<f32>, sun: vec3<f32>, distance: f32) -> Segment {
    let phase=scattering_phase(dot(direction,sun));
    var throughput=vec3(1.0);
    var luminance=vec3(0.0);
    for(var step=0u;step<48u;step++) {
        let a=f32(step)/48.0;
        let b=f32(step+1u)/48.0;
        let dt=(b*b-a*a)*distance;
        let t=0.5*(a*a+b*b)*distance;
        let p=vec3(0.0,h,0.0)+direction*t;
        let segment=lit_segment(p,sun,phase,dt,transmittance,multiple_scattering,lut_sampler);
        luminance+=throughput*segment.radiance;
        throughput*=segment.transmission;
    }
    return Segment(luminance,throughput);
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
    var luminance=transport.radiance;
    if ray.terminal_ground!=0u {
        let p=vec3(0.0,h,0.0)+direction*distance;
        let n=normalize(p+vec3(0.0,R,0.0));
        let local_sun_mu=dot(n,sun);
        let direct=sun_transmittance(p+n*0.1,sun)*max(0.0,local_sun_mu);
        let indirect=textureSampleLevel(ground_irradiance,lut_sampler,ground_irradiance_uv(local_sun_mu),0.0).rgb/GROUND_IRRADIANCE_GAIN;
        luminance+=transport.transmission*(direct+indirect)*(0.1/PI);
    }
    textureStore(output_lut,vec2<i32>(id.xy),vec4(luminance*SKY_LUT_GAIN,1.0));
}
