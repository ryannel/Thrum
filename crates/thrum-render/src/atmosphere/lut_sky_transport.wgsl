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
        let mu=local_sun_mu(p,sun);
        let direct=spectral_sun_pair(ah,mu,transmittance,lut_sampler);
        ray=lit_step(ray,medium_densities(ah),ah,mu,phase,dt,direct,multiple_scattering,lut_sampler);
    }
    return ray;
}
fn ground_radiance(h: f32, mu: f32, bank: u32) -> vec4<f32> {
    let direct=spectral_sun(h,mu,bank,transmittance,lut_sampler)*max(0.0,mu);
    let indirect=spectral_lut(ground_irradiance_uv(mu),bank,ground_irradiance,lut_sampler)/GROUND_IRRADIANCE_GAIN;
    return (direct+indirect)*(0.1/PI);
}

