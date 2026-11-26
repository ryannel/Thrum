    for(var band=0u;band<3u;band++) {
        let fade=1.0-smoothstep(water.lengths[band]/128.0,water.lengths[band]/32.0,spacing);
        let gain = row.gains[band+1u]*fade;
        if gain > 0.0 && water.heights[band+1u] > 0.0 {
            f=add_field(f,transported_field(motion,band),gain);
        }
    }
