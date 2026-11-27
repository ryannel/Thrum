struct GroundLight {
    extinction: f32,
    solar_kernel: f32,
    ambient: vec3<f32>,
    surface_visibility: f32,
}

// Allocate the directional source only the radiance left by the ambient
// source. Scaling the kernel preserves sunlight's colour and phase response.
fn ground_solar_kernel(fog: GroundLight, solar: vec3<f32>, sky_limit: vec4<f32>) -> f32 {
    if sky_limit.w == 0.0 { return fog.solar_kernel; }
    let remaining = max(vec3(0.0), sky_limit.rgb * fog.extinction - fog.ambient);
    let fraction = remaining / max(vec3(1e-12), solar * fog.solar_kernel);
    let limit = clamp(min(fraction.x, min(fraction.y, fraction.z)), 0.0, 1.0);
    return fog.solar_kernel * mix(1.0, limit, sky_limit.w);
}
