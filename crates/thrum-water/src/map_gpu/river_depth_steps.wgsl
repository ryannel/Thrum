        for (var i = 0u; i < 12u; i++) {
            let shift = fma(direction, vec2(height), origin);
            if abs(height) > max(0.25, water.heights.x) { break; }
