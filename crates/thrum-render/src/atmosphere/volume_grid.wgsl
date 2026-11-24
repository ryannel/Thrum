fn volume_grid_distance(layer: u32, layers: u32) -> f32 {
    let near = layers / 2u;
    if layer <= near {
        let t = f32(layer) / f32(near);
        return VOLUME_NEAR_DISTANCE * t * t;
    }
    let t = f32(layer - near) / f32(layers - near);
    return VOLUME_NEAR_DISTANCE + (32000.0 - VOLUME_NEAR_DISTANCE) * t * t;
}

fn volume_grid_coordinate(distance: f32, layers: u32) -> f32 {
    let near = layers / 2u;
    if distance <= VOLUME_NEAR_DISTANCE {
        return sqrt(distance / VOLUME_NEAR_DISTANCE) * f32(near);
    }
    let t = sqrt((distance - VOLUME_NEAR_DISTANCE) / (32000.0 - VOLUME_NEAR_DISTANCE));
    return f32(near) + t * f32(layers - near);
}

fn volume_distance(layer: u32) -> f32 {
    return volume_grid_distance(layer, VOLUME_INTERVALS);
}

fn volume_coordinate(distance: f32) -> f32 {
    return volume_grid_coordinate(distance, VOLUME_INTERVALS);
}
