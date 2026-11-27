        let raw_flow = point.flow.xy;
        let swash = flow_swatch(point.gains * settings.heights, raw_flow, settings.swash_margin.x, settings.swash_margin.y);
        let flow_gains = limit_flow(raw_flow, depth, swash);
        var flow_height = 0.0;
        for (var band = 0; band < 2; band++) {
            let wave = flow_gains[band] * out.flow[band];
            flow_height += wave.x + point.carry.x * wave.w;
            slope += wave.yz;
            rate += wave.w;
            velocity += flow_gains[band] * out.flow_velocity[band].xyz;
        }
