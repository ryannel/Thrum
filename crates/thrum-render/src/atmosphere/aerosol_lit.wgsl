fn aerosol_multiple_uv(h: f32, mu: f32) -> vec2<f32> {
    let height = max(0.0, h) / H;
    // At the default coefficients the joint volume reads the retained chart.
    // Live charts concentrate rows in the aerosol layer near the ground.
    let fixed = aerosol.surface_extinction.w == MIE_EXTINCTION &&
        aerosol.extinction[1].x == MIE_EXTINCTION;
    return vec2(mu * 0.5 + 0.5, select(sqrt(height), height, fixed));
}

fn lit_step(
    ray: SpectralRay, densities: vec3<f32>, h: f32, mu: f32, phase: vec2<f32>, dt: f32,
    direct: array<vec4<f32>,2>, multiple: texture_2d<f32>, linear: sampler,
) -> SpectralRay {
    let uv = aerosol_multiple_uv(h, mu);
    let lo = scattering_step(densities, phase, dt, 0u, direct[0],
        spectral_lut(uv, 0u, multiple, linear) / MULTIPLE_LUT_GAIN);
    let hi = scattering_step(densities, phase, dt, 1u, direct[1],
        spectral_lut(uv, 1u, multiple, linear) / MULTIPLE_LUT_GAIN);
    return SpectralRay(extend_ray(ray.lo, lo), extend_ray(ray.hi, hi));
}
