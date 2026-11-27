    let path = sun_path(input.world_position, light.direction_to_light.xyz);
    var sun_factor = path.factor;
    if shadowed {
        if path.weight >= 1.0 {
            sun_factor *= point_sun_visibility(path.exit, sun_path_shadow_sampler);
        } else if path.weight > 0.0 {
            sun_factor *= mix(sun_visibility(input,receiver), point_sun_visibility(path.exit, sun_path_shadow_sampler), path.weight);
        } else {
            sun_factor *= sun_visibility(input,receiver);
        }
    }
