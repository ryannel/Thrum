    if density > 0.0 && cloud.physical.x > 0.0 && medium.envelope > 0.0 && cloud.physical.y > 0.0 {
        density = placed_erode(density, cloud.physical.x * medium.envelope, cloud.physical.y,
            cloud_billow(cloud_shape_uv(point), footprint), cloud.density_profile.x);
    }
    return vec4(density, sample.y * above, sample.y * (1.0 - above), top);
}
