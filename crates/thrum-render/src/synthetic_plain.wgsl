fn indirect_radiance(base: vec3<f32>, metalness: f32, roughness: f32,
                     n: vec3<f32>, v: vec3<f32>, ao: f32) -> vec3<f32> {
    // Preserve the authored fixed-light arithmetic and resource layout.
    return ao*(1.0-metalness)*base*ambient_irradiance(n)/3.14159265359;
}

fn aerial_colour(colour: vec3<f32>, world_position: vec3<f32>, screen_position: vec4<f32>) -> vec3<f32> {
    return colour;
}
