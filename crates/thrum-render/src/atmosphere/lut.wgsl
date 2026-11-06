@group(0) @binding(1) var transmittance: texture_2d<f32>;
@group(0) @binding(2) var multiple_scattering: texture_2d<f32>;
@group(0) @binding(3) var output_lut: texture_storage_2d<rgba16float,write>;
@group(0) @binding(4) var lut_sampler: sampler;
fn sun_transmittance(p: vec3<f32>, sun: vec3<f32>) -> vec3<f32> {
    let h = altitude(p);
    let up = normalize(p+vec3(0.0,R,0.0));
    let mu = dot(up,sun);
    if blocked(h,mu) { return vec3(0.0); }
    return textureSampleLevel(transmittance,lut_sampler,trans_uv(h,mu),0.0).rgb;
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
        depth += extinction(altitude(vec3(0.0,h,0.0)+direction*(f32(i)+0.5)*step))*step;
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
            let ah=altitude(p);
            let ext=extinction(ah);
            let integral=safe_segment(scattering(ah),ext,dt);
            luminance+=throughput*integral*sun_transmittance(p,sun)/(4.0*PI*32.0);
            feedback+=throughput*integral/32.0;
            throughput*=exp(-ext*dt);
        }
        if ground>=0.0 {
            let p=vec3(0.0,h,0.0)+direction*distance;
            let n=normalize(p+vec3(0.0,R,0.0));
            luminance+=throughput*sun_transmittance(p+n*0.1,sun)*max(0.0,dot(n,sun))*(0.1/(PI*32.0));
            feedback+=throughput*(0.1/32.0);
        }
    }
    let multiple=luminance/max(vec3(0.001),vec3(1.0)-feedback);
    textureStore(output_lut,vec2<i32>(id.xy),vec4(multiple,1.0));
}
@compute @workgroup_size(8,8)
fn sky_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size=textureDimensions(output_lut);
    if any(id.xy>=size) { return; }
    let angles=sky_angles((vec2<f32>(id.xy)+0.5)/vec2<f32>(size));
    let sun=atmosphere.sun_lux.xyz;
    let azimuth=angles.x+atan2(sun.z,sun.x);
    let direction=vec3(cos(angles.y)*cos(azimuth),sin(angles.y),cos(angles.y)*sin(azimuth));
    let h=atmosphere.camera_altitude.w;
    let ground=ground_distance(h,direction.y);
    var distance=top_distance(h,direction.y);
    if ground>=0.0 { distance=ground; }
    let cosine=dot(direction,sun);
    let rayleigh_phase=3.0*(1.0+cosine*cosine)/(16.0*PI);
    let g=0.8;
    let mie_phase=(1.0-g*g)/(4.0*PI*pow(max(0.001,1.0+g*g-2.0*g*cosine),1.5));
    var throughput=vec3(1.0);
    var luminance=vec3(0.0);
    for(var step=0u;step<48u;step++) {
        let a=f32(step)/48.0;
        let b=f32(step+1u)/48.0;
        let dt=(b*b-a*a)*distance;
        let t=0.5*(a*a+b*b)*distance;
        let p=vec3(0.0,h,0.0)+direction*t;
        let ah=altitude(p);
        let ext=extinction(ah);
        let single=(vec3(5.802e-6,13.558e-6,33.100e-6)*exp(-ah/8000.0)*rayleigh_phase
            +vec3(3.996e-6)*exp(-ah/1200.0)*mie_phase)*sun_transmittance(p,sun);
        let mu=dot(normalize(p+vec3(0.0,R,0.0)),sun);
        let multiple=textureSampleLevel(multiple_scattering,lut_sampler,vec2(mu*0.5+0.5,ah/H),0.0).rgb;
        luminance+=throughput*safe_segment(single+scattering(ah)*multiple,ext,dt);
        throughput*=exp(-ext*dt);
    }
    if ground>=0.0 {
        let p=vec3(0.0,h,0.0)+direction*distance;
        let n=normalize(p+vec3(0.0,R,0.0));
        luminance+=throughput*sun_transmittance(p+n*0.1,sun)*max(0.0,dot(n,sun))*(0.1/PI);
    }
    textureStore(output_lut,vec2<i32>(id.xy),vec4(luminance,1.0));
}
