// Surface visibility and fog optical depth affect illumination only.
fn cloud_layer_ground_path(world: vec3<f32>) -> vec2<f32> {
    if world.y >= ground_frame.ground_heights.z + ground_light_window.top_border.x {
        return vec2(1.0, 0.0);
    }
    return ground_sun_depths(world, ground_surface(world.xz).x).zw;
}

fn cloud_near_light(world: vec3<f32>, ray: vec3<f32>, true_sun: vec3<f32>,
                    distance: f32, transmission: f32) -> GroundLight {
    let point = world - vec3(cloud.drift.x, 0.0, cloud.drift.y);
    let footprint = max(14.0, distance / f32(cloud.active_extent.y));
    let column = cloud_column(point);
    let placed = cloud_coarse_placed(point, footprint);
    let sigma = cloud_eroded_density(column.x, point, footprint, cloud.physical.x,
        column.w, cloud.density_profile.x) + placed.x;
    if sigma <= 0.0 { return GroundLight(0.0, 0.0, vec3(0.0), 1.0); }
    let ground = cloud_layer_ground_path(world);
    // Read the layer on the cloud band's height grid. The surface-relative
    // ground band supplies only the added fog depth and terrain visibility.
    let column_depths = cloud_window_depths(world, world.y, cloud.heights.xy);
    let toward_sun = column_depths.x + ground.y;
    let depths = vec2(toward_sun, max(toward_sun, column_depths.y));
    let angles = vec4(mapping.path_border.y, -ray.y, 0.0, 1.0);
    let kernel = cloud_sun_term(cloud_phases(ray, true_sun), depths.x, depths.xy,
        1.0, angles, distance, transmission, vec4(cloud.scattering.xyz, cloud.erosion.z));
    return GroundLight(sigma, sigma * kernel, sigma * cloud_ambient_light(column.yz + placed.yz), ground.x);
}

fn cloud_layer_at_height(height: f32) -> bool {
    return cloud.heights.x < cloud.heights.y && height >= cloud.heights.x && height <= cloud.heights.y;
}
