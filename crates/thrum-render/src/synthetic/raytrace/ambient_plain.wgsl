fn ambient_indirect(base: vec3<f32>, metalness: f32, roughness: f32,
                     n: vec3<f32>, v: vec3<f32>, ao: f32,
                     world_position: vec3<f32>, pixel_position: vec4<f32>, traced: f32) -> vec3<f32> {
    // Preserve the authored fixed-light arithmetic and resource layout.
    return (ao*traced)*(1.0-metalness)*base*ambient_irradiance(n,world_position,pixel_position)/3.14159265359;
}
