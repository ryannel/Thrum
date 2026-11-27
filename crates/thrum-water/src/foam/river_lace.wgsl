    var lace: f32;
    if map.flow_layers[row.body].x != 0u && length(row.current) > 0.05 {
        let direction = normalize(row.current);
        let motion = river_surface_motion(point.xz, row);
        lace = river_foam_cover(stocks, motion.departure, direction, footprint);
        if motion.blend < 1.0 {
            let other = river_foam_cover(stocks, motion.second, direction, footprint);
            lace = mix(other, lace, motion.blend);
        }
    } else if any(row.current != vec2(0.0)) {
        let reading = carried_points(point.xz, row.current, water.carriage.xy);
        let cycle = u32(water.carriage.z);
        let other = cycle + u32(water.carriage.x >= 0.5);
        let renewal = 1.4 * smoothstep(0.0, 0.8, length(row.current));
        let a = reading.first + renewal * foam_renewal(cycle, 11.0);
        let b = reading.second + renewal * foam_renewal(other, 32.0);
        lace = mix(ranked_cover(stocks, b, footprint),
            ranked_cover(stocks, a, footprint), reading.weight);
    } else {
        lace = ranked_cover(stocks, point.xz, footprint);
    }
