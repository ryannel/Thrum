    let path = sun_path(input.world_position, light.direction_to_light.xyz);
    var sun_factor = path.factor;
    if shadowed {
        if path.submerged {
            sun_factor *= point_sun_visibility(path.exit, sun_path_shadow_sampler);
        } else {
            sun_factor *= sun_visibility(input,receiver);
        }
    }
