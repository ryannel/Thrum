    if density > 0.0 && cloud.physical.x > 0.0 && medium.envelope > 0.0 && cloud.physical.y > 0.0 {
        let uv = point / 800.0 + vec3(cloud.noise.z, 0.0, cloud.noise.w);
        let detail = textureSampleLevel(detail_noise, noise_sampler, uv, erosion_mip(footprint, 12.5)).r;
        density = placed_erode(density, cloud.physical.x * medium.envelope,
            cloud.physical.y, cloud_billow(cloud_shape_uv(point), footprint), detail);
    }
    return vec4(density, sample.y * above, sample.y * (1.0 - above), top);
}
