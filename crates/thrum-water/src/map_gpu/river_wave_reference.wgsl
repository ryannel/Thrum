            let a = textureLoad(bed_map, first, layer, 0);
            let b = textureLoad(bed_map, first + CORNERS[1], layer, 0);
            let c = textureLoad(bed_map, first + CORNERS[2], layer, 0);
            let d = textureLoad(bed_map, first + CORNERS[3], layer, 0);
            let zero = vec4(0.0);
            // Authored current and reference velocity can cancel at different
            // points between opposing lanes. Scale each direction separately.
            let forward = mix(mix(max(a, zero), max(b, zero), uv.x),
                mix(max(c, zero), max(d, zero), uv.x), uv.y);
            let backward = mix(mix(min(a, zero), min(b, zero), uv.x),
                mix(min(c, zero), min(d, zero), uv.x), uv.y);
            let gain_forward = clamp(current / max(forward.zw, vec2(1e-6)), vec2(0.0), vec2(1.0));
            let gain_backward = clamp(-current / max(-backward.zw, vec2(1e-6)), vec2(0.0), vec2(1.0));
            return vec4(gain_forward * forward.xy + gain_backward * backward.xy, record.track.zw);
