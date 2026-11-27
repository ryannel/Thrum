// Preserve the low deck’s horizontal lighting plane in the distant trace and overlay.
// The near field samples every layer, regardless of the surface below it.
fn cloud_deck_active() -> bool {
    return cloud.low_deck != 0u;
}

fn cloud_deck_light(world: vec3<f32>, ray: vec3<f32>, true_sun: vec3<f32>,
                    distance: f32, transmission: f32) -> GroundLight {
    let point = world - vec3(cloud.drift.x, 0.0, cloud.drift.y);
    return cloud_coarse_layer_light(world, point, ray, true_sun, distance, transmission, vec4(0.0));
}

fn cloud_near_light(world: vec3<f32>, ray: vec3<f32>, true_sun: vec3<f32>,
                    distance: f32, transmission: f32) -> GroundLight {
    let point = world - vec3(cloud.drift.x, 0.0, cloud.drift.y);
    let placed = cloud_coarse_placed(point, max(14.0, distance / f32(cloud.active_extent.y)));
    return cloud_coarse_layer_light(world, point, ray, true_sun, distance, transmission, placed);
}

fn cloud_coarse_layer_light(world: vec3<f32>, point: vec3<f32>, ray: vec3<f32>,
                            true_sun: vec3<f32>, distance: f32, transmission: f32,
                            placed: vec4<f32>) -> GroundLight {
    let column = cloud_column(point);
    var sigma = placed.x;
    if column.x > 0.0 {
        let footprint = max(14.0, distance / f32(cloud.active_extent.y));
        let billow = cloud_billow(cloud_shape_uv(point), footprint);
        sigma += cloud_erode(column.x, cloud.physical.x, cloud.physical.y, billow, cloud.density_profile.x);
    }
    if sigma <= 0.0 { return GroundLight(0.0, 0.0, vec3(0.0), 1.0); }
    let surface = ground_surface(world.xz).x;
    let ground = ground_sun_depths(world, surface);
    // Read the deck on the cloud band's height grid. The surface-relative
    // ground band supplies only the added fog depth and terrain visibility.
    let column_depths = cloud_window_depths(world, world.y, cloud.heights.xy);
    let toward_sun = column_depths.x + ground.w;
    let depths = vec2(toward_sun, max(toward_sun, column_depths.y));
    let angles = vec4(mapping.path_border.y, -ray.y, 0.0, 1.0);
    let kernel = cloud_sun_term(cloud_phases(ray, true_sun), depths.x, depths.xy,
        1.0, angles, distance, transmission, vec4(cloud.scattering.xyz, cloud.erosion.z));
    return GroundLight(sigma, sigma * kernel, sigma * cloud_ambient_light(column.yz + placed.yz), ground.z);
}


fn cloud_layer_at_height(height: f32) -> bool {
    return cloud.heights.x < cloud.heights.y && height >= cloud.heights.x && height <= cloud.heights.y;
}
