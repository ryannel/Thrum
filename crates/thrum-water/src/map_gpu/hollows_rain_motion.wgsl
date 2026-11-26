    let row = surface_pixel(vertex.xy, slot);
    let motion = cascade_motion(vertex.xy, row.current, row.track, water.carriage.w, water.carriage.y);
    let wave = rain_geometry(vertex.xy, rectangle.origin_spacing.z, row, motion);
    let world = vec3(vertex.x, levels.x + wave.x, vertex.y);
    let current = view.unjittered_view_proj * vec4(world, 1.0);
    let previous = view.previous_view_proj * vec4(vertex.x, levels.y + wave.x - wave.y * water.level_dt_reset.y, vertex.y, 1.0);
