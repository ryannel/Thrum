@group(0) @binding(1) var transmittance: texture_2d<f32>;
@group(0) @binding(2) var multiple_scattering: texture_2d<f32>;
@group(0) @binding(3) var scatter_out: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(4) var trans_out: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(5) var lut_sampler: sampler;

// Aerial columns are interpolation support, including below the model's
// ground. The charts and the sky keep their original, clamped altitude.
fn aerial_altitude(p: vec3<f32>) -> f32 {
    let n = dot(p,p)+2.0*R*p.y;
    return n/(sqrt(max(0.0,R*R+n))+R);
}

// Continue the chart's near-surface optical-depth slope below its domain.
// Diffuse sources stay at the surface; the direct Sun alone dims with depth.
// A Sun hidden from the ground point above the sample remains hidden.
fn aerial_sun_pair(h: f32, mu: f32) -> array<vec4<f32>,2> {
    let surface = spectral_sun_pair(max(0.0,h),mu,transmittance,lut_sampler);
    if h >= 0.0 || mu < 0.0 { return surface; }
    let above = spectral_sun_pair(50.0,mu,transmittance,lut_sampler);
    let depth = vec4(-h/50.0);
    let lo = clamp(surface[0]/max(above[0],vec4(1e-20)),vec4(0.0),vec4(1.0));
    let hi = clamp(surface[1]/max(above[1],vec4(1e-20)),vec4(0.0),vec4(1.0));
    return array(surface[0]*pow(lo,depth),surface[1]*pow(hi,depth));
}

// The Sun's single-scattered part of lit_step: its multiple-scattering source is zero.
// Consumers that know terrain visibility along the ray attenuate only this part.
fn single_step(
    ray: SpectralRay, densities: vec3<f32>, phase: vec2<f32>, dt: f32,
    direct: array<vec4<f32>,2>,
) -> SpectralRay {
    let lo=scattering_step(densities,phase,dt,0u,direct[0],vec4(0.0));
    let hi=scattering_step(densities,phase,dt,1u,direct[1],vec4(0.0));
    return SpectralRay(extend_ray(ray.lo,lo),extend_ray(ray.hi,hi));
}
