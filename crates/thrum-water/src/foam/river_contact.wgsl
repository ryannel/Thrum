    // As the waterline crosses an opaque rock, feather surface film into
    // the same shallow contact interval as the transparent sheet.
    if FOAM_ENABLED && OPTICS_ENABLED && map.flow_layers[row.body].x != 0u {
        whitewater.y *= smoothstep(0.015, 0.095, paths.edge_depth);
    }
