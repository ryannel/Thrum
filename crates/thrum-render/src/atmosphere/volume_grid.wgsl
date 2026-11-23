fn volume_distance(layer: u32) -> f32 {
    if layer <= VOLUME_NEAR_LAYERS {
        let t = f32(layer) / f32(VOLUME_NEAR_LAYERS);
        return VOLUME_NEAR_DISTANCE * t * t;
    }
    let t = f32(layer - VOLUME_NEAR_LAYERS) / f32(VOLUME_INTERVALS - VOLUME_NEAR_LAYERS);
    return VOLUME_NEAR_DISTANCE + (32000.0 - VOLUME_NEAR_DISTANCE) * t * t;
}

fn volume_coordinate(distance: f32) -> f32 {
    if distance <= VOLUME_NEAR_DISTANCE {
        return sqrt(distance / VOLUME_NEAR_DISTANCE) * f32(VOLUME_NEAR_LAYERS);
    }
    let t = sqrt((distance - VOLUME_NEAR_DISTANCE) / (32000.0 - VOLUME_NEAR_DISTANCE));
    return f32(VOLUME_NEAR_LAYERS) + t * f32(VOLUME_INTERVALS - VOLUME_NEAR_LAYERS);
}
