    if point.body != 0xffffffffu {
        let depth = select(1e30, row.x - row.w, (point.flags & 1u) != 0u);
        let long = result.wave;
        out.cut_fine_nodes = result.cut_fine_nodes;
        out.cut_coarse_nodes = result.cut_coarse_nodes;
        let long_slope = world_slope(long);
        var rate = long.velocity.y - dot(long_slope, long.velocity.xz);
        var slope = point.slope + long_slope;
        var velocity = long.velocity + vec3(row.y, 0.0, row.z);
        out.layers.x = long.displacement.y;
        for (var band = 0u; band < 3u; band++) {
            let gain = gains[band + 1u];
            if gain > 0.0 {
                let carried = bands[band];
                let wave = carried.wave;
                let band_slope = cascade_world_slope(wave.slope, carried.differential, gain);
                // The cascades are sampled at a fixed column in their periodic lattice.
                let band_rate = gain * (wave.velocity.y + out.advection_rate[band]);
                out.layers[band + 1u] = gain * wave.displacement.y + point.carry.x * band_rate;
                slope += band_slope;
                velocity += gain * wave.velocity;
                rate += band_rate;
            }
        }
