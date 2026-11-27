    let body_index = surface_body(input.body);
    let hollow_surface = map.bodies[body_index].layers.x == 3u;
    if hollow_surface {
        // The cell's bed height determines ownership. At the waterline, use
        // the same opaque triangles that clip the horizontal sheet for optics.
        let pixel = vec2<u32>(input.clip.xy);
        let distance = gather_depth(pixel);
        if distance > 0.0 {
            row.bed = gather_point(pixel, distance).y;
        }
    }
