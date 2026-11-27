    let flow_gains = row.flow_gains;
    for (var band = 0; band < 2; band++) {
        let gain = flow_gains[band];
        if gain <= 0.0 { continue; }
        let scale = FLOW_PATCHES[band];
        let layer = FLOW_FIRST + band * 5;
        var m = filtered_layer(cascades, periodic, motion.departure / scale, departure_dx / scale, departure_dy / scale, layer);
        if motion.blend < 1.0 {
            let b = filtered_layer(cascades, periodic, motion.second / scale, second_dx / scale, second_dy / scale, layer);
            m.slope = mix(b.slope, m.slope, motion.blend);
            m.second = mix(b.second, m.second, motion.blend);
        }
        slope += gain * m.slope;
        covariance += gain * gain * (m.second - vec3(m.slope.x * m.slope.x, m.slope.x * m.slope.y, m.slope.y * m.slope.y));
    }
