struct RiverOutput {
    @location(0) colour: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @location(2) reactive: f32,
    @builtin(frag_depth) depth: f32,
}

@fragment fn river_fragment(input: VertexOut) -> RiverOutput {
    // The vertices still resolve wave geometry. Replace only the interpolated
    // still level with the sampled chart level, finding the hit on the same ray.
    // Derivatives give the local triangle's mapping from world to rest space.
    let x = dpdx(input.world);
    let y = dpdy(input.world);
    let rest_x = dpdx(input.rest);
    let rest_y = dpdy(input.rest);
    let level_gradient = vec2(dpdx(input.level), dpdy(input.level));
    let normal = cross(x, y);
    let ray = normalize(input.world - view.camera_xyz_time.xyz);
    let facing = dot(normal, ray);
    let area_squared = dot(normal, normal);
    var surface = input;
    var row = surface_pixel(input.rest, input.body);
    var depth = input.clip.z;
    // Preserve coverage at a grazing silhouette, where a small height
    // correction could otherwise jump across another part of the sheet.
    if row.owned && facing * facing > 0.0001 * area_squared {
        let ray_per_height = normal.y / facing;
        let tangent = ray * ray_per_height - vec3(0.0, 1.0, 0.0);
        let screen_per_height = vec2(dot(cross(tangent, y), normal),
            dot(cross(x, tangent), normal)) / area_squared;
        let rest_per_height = rest_x * screen_per_height.x + rest_y * screen_per_height.y;
        let level_per_height = dot(level_gradient, screen_per_height);
        var height = 0.0;
        for (var i = 0u; i < 3u; i++) {
            let residual = row.level - (input.level + height * level_per_height + height);
            if abs(residual) <= 0.000004 { break; }
            let next = height + residual;
            let distance = next * ray_per_height;
            if abs(distance) > 0.25 { break; }
            let rest = input.rest + next * rest_per_height;
            let sample = surface_pixel(rest, input.body);
            if !sample.owned { break; }
            height = next;
            row = sample;
            surface.rest = rest;
            surface.world = input.world + distance * ray;
            // Fragment position w is the reciprocal of the original clip w.
            // Project the offset as a vector to retain the rasterizer's precision.
            let delta_clip = view.view_proj * vec4(distance * ray, 0.0);
            depth = clamp((input.clip.z + delta_clip.z * input.clip.w) /
                (1.0 + delta_clip.w * input.clip.w), 0.0, 1.0);
        }
    }
    let shaded = shade_water(surface, row);
    return RiverOutput(shaded.colour, shaded.motion, shaded.reactive, depth);
}
