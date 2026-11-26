// A low deck is sampled from the weather columns by both integrators. Its
// horizontal plane stays fixed as the camera passes into and above the layer.
fn cloud_deck_active() -> bool {
    return cloud.low_deck != 0u;
}

fn cloud_deck_light(world: vec3<f32>, ray: vec3<f32>, true_sun: vec3<f32>,
                    distance: f32, transmission: f32) -> GroundLight {
    let point = world - vec3(cloud.drift.x, 0.0, cloud.drift.y);
    let column = cloud_column(point);
    if column.x <= 0.0 { return GroundLight(0.0, 0.0, vec3(0.0), 1.0); }
    let uv = point / 6400.0 + vec3(cloud.noise.x, 0.0, cloud.noise.y);
    let footprint = max(14.0, distance / f32(cloud.active_extent.y));
    let billow = textureSampleLevel(shape_noise, noise_sampler, uv, erosion_mip(footprint, 50.0)).r;
    let sigma = cloud_erode(column.x, cloud.physical.x, cloud.physical.y, billow, cloud.density_profile.x);
    if sigma <= 0.0 { return GroundLight(0.0, 0.0, vec3(0.0), 1.0); }
    let surface = ground_surface(world.xz).x;
    let depths = ground_sun_depths(world, surface);
    let angles = vec4(mapping.path_border.y, -ray.y, 0.0, 1.0);
    let kernel = cloud_sun_term(cloud_phases(ray, true_sun), depths.x, depths.xy,
        1.0, angles, distance, transmission, vec4(cloud.scattering.xyz, cloud.erosion.z));
    return GroundLight(sigma, sigma * kernel, sigma * cloud_ambient_light(column.yz), depths.z);
}
