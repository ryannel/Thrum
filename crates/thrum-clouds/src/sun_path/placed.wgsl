fn placed_sun_depth(midpoint: vec3<f32>, height_span: f32) -> f32 {
    let centre = midpoint - vec3(p.weather[0].x, 0.0, p.weather[0].y);
    let span = height_span / p.direction.y;
    let reach = abs(p.direction.xz) * (0.5 * span);
    let bounds = placed_instances.bounds;
    if placed_instances.count.x == 0u || any(centre.xz + reach < bounds.xy)
        || any(centre.xz - reach > bounds.zw) {
        return 0.0;
    }
    let count = u32(ceil(span / 7.5));
    let step = span / f32(count);
    var tau = 0.0;
    for (var j = 0u; j < count; j++) {
        let offset = p.direction.xyz * ((f32(j) + 0.5) * step - 0.5 * span);
        let point = centre + offset;
        let sigma = placed_sample(point, 0.0).x;
        if sigma > 0.0 && p.physical.z > 0.0 && p.physical.w > 0.0 {
            let uv = (midpoint + offset) / 6400.0 + vec3(p.erosion.y, 0.0, p.erosion.z);
            let billow = textureSampleLevel(shape_noise, noise_sampler, uv, 0.0).r;
            tau += cloud_erode(sigma, p.physical.z, p.physical.w, billow, p.erosion.x) * step;
        } else {
            tau += sigma * step;
        }
    }
    return tau;
}
