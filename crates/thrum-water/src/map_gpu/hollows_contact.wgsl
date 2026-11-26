    if map.bodies[surface_body(input.body)].layers.x == 3u {
        // The cell's bed height determines ownership. At the waterline, use
        // the same opaque triangles that clip the horizontal sheet for optics.
        let pixel = vec2<u32>(input.clip.xy);
        let distance = gather_depth(pixel);
        if distance > 0.0 {
            row.bed = gather_point(pixel, distance).y;
        }
    }
