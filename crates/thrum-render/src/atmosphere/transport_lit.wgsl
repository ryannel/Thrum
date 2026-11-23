fn lit_step(
    ray: SpectralRay, densities: vec3<f32>, h: f32, mu: f32, phase: vec2<f32>, dt: f32,
    direct: array<vec4<f32>,2>, multiple: texture_2d<f32>, linear: sampler,
) -> SpectralRay {
    let uv=vec2(mu*0.5+0.5,h/H);
    let lo=scattering_step(densities,phase,dt,0u,direct[0],
        spectral_lut(uv,0u,multiple,linear)/MULTIPLE_LUT_GAIN);
    let hi=scattering_step(densities,phase,dt,1u,direct[1],
        spectral_lut(uv,1u,multiple,linear)/MULTIPLE_LUT_GAIN);
    return SpectralRay(extend_ray(ray.lo,lo),extend_ray(ray.hi,hi));
}
