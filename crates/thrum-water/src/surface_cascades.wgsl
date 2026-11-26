    for(var band=0u;band<3u;band++) {
        let fade=1.0-smoothstep(water.lengths[band]/128.0,water.lengths[band]/32.0,spacing);
        f=add_field(f,transported_field(motion,band),row.gains[band+1u]*fade);
    }
