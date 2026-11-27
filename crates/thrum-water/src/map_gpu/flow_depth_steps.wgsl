        let has_chop = any(row.flow_gains > vec2(0.0));
        // The sub-metre bands use a bounded refinement at bank angles,
        // where the raster already has the chart's standing relief.
        let limit = select(12u, 5u, has_chop);
        for (var i = 0u; i < limit; i++) {
            let shift = fma(direction, vec2(height), origin);
            if abs(height) > max(0.25, water.heights.x) { break; }
