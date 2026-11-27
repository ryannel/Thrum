fn overlay_meter_direction(number: u32) -> vec3<f32> {
    let mu = (f32(number / 16u) + 0.5) / 16.0;
    let phi = 2.0 * PI * (f32(number % 16u) + 0.5) / 16.0;
    let radius = sqrt(max(0.0, 1.0 - mu * mu));
    return vec3(radius * cos(phi), mu, radius * sin(phi));
}
