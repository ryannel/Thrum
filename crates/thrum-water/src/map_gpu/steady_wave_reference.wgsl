        let a = mix(textureLoad(bed_map, first, layer, 0),
            textureLoad(bed_map, first + CORNERS[1], layer, 0), uv.x);
        let b = mix(textureLoad(bed_map, first + CORNERS[2], layer, 0),
            textureLoad(bed_map, first + CORNERS[3], layer, 0), uv.x);
        let sample = mix(a, b, uv.y);
        // As a joined speed tends to zero, both departure reads meet the
        // receiving field continuously. Unjoined water keeps its reference.
        let scale = clamp(sample.zw * current / max(sample.zw * sample.zw, vec2(1e-12)),
            vec2(0.0), vec2(1.0));
        return vec4(sample.xy * scale, record.track.zw);
