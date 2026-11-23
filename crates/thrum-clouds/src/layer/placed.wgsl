fn cloud_placed(point: vec3<f32>, footprint: f32) -> vec4<f32> {
    let medium = placed_sample(point - vec3(cloud.weather[0].x, 0.0, cloud.weather[0].y), footprint);
    let sample = medium.column;
    if sample.x <= 0.0 {
        return vec4(0.0);
    }
    let top = sample.w;
    let h = clamp((point.y - sample.z) / (top - sample.z), 0.0, 1.0);
    let profile = placed_instances.optical.zw;
    let integral = 1.0 - (profile.x + profile.y) * 0.5;
    let above = clamp((cloud_fade_integral(1.0 - h, profile.y)
        - max(0.0, 0.5 * profile.x - cloud_fade_integral(h, profile.x))) / integral, 0.0, 1.0);
    let density = cloud_detail(sample.x, point, footprint, cloud.physical.x * medium.envelope);
    return vec4(density, sample.y * above, sample.y * (1.0 - above), top);
}
