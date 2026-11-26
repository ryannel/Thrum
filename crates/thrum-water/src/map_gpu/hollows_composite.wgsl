    if map.bodies[surface_body(input.body)].layers.x == 3u {
        // The gathered ground is already transmitted in the optical result.
        // Only soften its sub-millimetre contact with the opaque triangles.
        opacity = smoothstep(0.0, 0.001, paths.edge_depth);
    }
