struct RiverOutput {
    @location(0) colour: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @location(2) reactive: f32,
    @builtin(frag_depth) depth: f32,
}

// Cubic reconstruction of the long cache preserves swell curvature between
// the metre-spaced samples. Both packed planes contribute to the height.
fn river_swell_level(point: vec2<f32>, level: u32) -> f32 {
    let coord = (point - water.cache[level].xy) / water.cache[level].z - 0.5;
    let base = floor(coord);
    let t = fract(coord);
    // Four-point Lagrange weights keep the smoothly varying swell in f32.
    // The filtered texture path can round the packed half-floats separately.
    let weights = array(
        -t * (t - 1.0) * (t - 2.0) / 6.0,
        (t + 1.0) * (t - 1.0) * (t - 2.0) / 2.0,
        -(t + 1.0) * t * (t - 2.0) / 2.0,
        (t + 1.0) * t * (t - 1.0) / 6.0
    );
    let layer = i32(level * 5u);
    let first = vec2<i32>(base) - 1;
    var height = 0.0;
    for (var z = 0; z < 4; z++) {
        var scan = 0.0;
        for (var x = 0; x < 4; x++) {
            let at = clamp(first + vec2(x, z), vec2(0), vec2(127));
            let wave = textureLoad(long_field, at, layer, 0).y;
            let residual = textureLoad(long_field, at, layer + 4, 0).z;
            scan += weights[x].x * (wave + residual);
        }
        height += weights[z].y * scan;
    }
    return height;
}

fn river_swell(point: vec2<f32>, spacing: f32, row: SurfaceRow) -> f32 {
    if row.gains.x * water.heights.x == 0.0 { return 0.0; }
    let lod = long_lod(point, spacing);
    let low = u32(lod);
    var height = river_swell_level(point, low);
    if fract(lod) > 0.0 && low < 7u {
        height = mix(height, river_swell_level(point, low + 1u), fract(lod));
    }
    let fade = 1.0 - smoothstep(4000.0, 8000.0, length(point - view.camera_xyz_time.xz));
    return height * row.gains.x * fade;
}

fn river_refinement(point: vec2<f32>, id: u32, spacing: f32) -> f32 {
    if map.bodies[id].joins[0].w > 0.0 { return 1.0; }
    if spacing > 2.0 { return 0.0; }
    var weight = 0.0;
    // A receiving sheet shares the correction at the exact ownership edge.
    // Fade to its ordinary mesh beyond the strip that spans that edge.
    for (var i = id + 1u; i < map.size_count_sea.z; i++) {
        let chart = map.bodies[i];
        if chart.joins[0].w == 0.0 { continue; }
        for (var e = 0u; e < chart.layers.w; e++) {
            if u32(chart.joins[e].x) != id { continue; }
            let segment = chart.ends[e];
            let delta = segment.zw - segment.xy;
            let offset = point - segment.xy;
            let distance = length(offset - delta * clamp(dot(offset, delta) / dot(delta, delta), 0.0, 1.0));
            weight = max(weight, 1.0 - smoothstep(spacing, 3.0 * spacing, distance));
        }
    }
    return weight;
}

@fragment fn river_fragment(input: VertexOut) -> RiverOutput {
    // Intersect the pixel ray with the sampled chart and swell. The mesh
    // carries the short waves and wake and supplies an initial intersection.
    let x = dpdx(input.world);
    let y = dpdy(input.world);
    let rest_x = dpdx(input.rest);
    let rest_y = dpdy(input.rest);
    let rebuild_swell = input.surf_weight == 0.0;
    let mesh_level = input.river.x + select(0.0, input.river.y, rebuild_swell);
    let level_gradient = vec2(dpdx(mesh_level), dpdy(mesh_level));
    let normal = cross(x, y);
    let ray = normalize(input.world - view.camera_xyz_time.xyz);
    let facing = dot(normal, ray);
    let area_squared = dot(normal, normal);
    var surface = input;
    var row = surface_pixel(input.rest, input.body);
    var depth = input.clip.z;
    // Preserve coverage at a grazing silhouette, where a small height
    // correction could otherwise jump across another part of the sheet.
    let weight = river_refinement(input.rest, surface_body(input.body), input.river.z);
    if weight > 0.0 && row.owned && facing * facing > 0.0001 * area_squared {
        let ray_per_height = normal.y / facing;
        let tangent = ray * ray_per_height - vec3(0.0, 1.0, 0.0);
        let screen_per_height = vec2(dot(cross(tangent, y), normal),
            dot(cross(x, tangent), normal)) / area_squared;
        let rest_per_height = rest_x * screen_per_height.x + rest_y * screen_per_height.y;
        let level_per_height = dot(level_gradient, screen_per_height);
        var height = 0.0;
        var previous_height = 0.0;
        var previous_residual = 0.0;
        for (var i = 0u; i < 8u; i++) {
            var physical_level = row.level;
            if rebuild_swell { physical_level += river_swell(surface.rest, input.river.z, row); }
            let raster_level = fma(height, level_per_height, mesh_level);
            let residual = fma(weight, physical_level - raster_level, -height);
            if abs(residual) <= 0.000004 { break; }
            // The second reading also estimates the slope of the residual
            // along the ray, which accelerates oblique hits over bank slopes.
            var step = residual;
            if i > 0u {
                let drop = previous_residual - residual;
                let rise = height - previous_height;
                if drop * rise > 0.0 {
                    step *= clamp(rise / drop, 0.25, 4.0);
                }
            }
            let next = height + step;
            let distance = next * ray_per_height;
            if abs(distance) > 0.25 { break; }
            let rest = input.rest + next * rest_per_height;
            let sample = surface_pixel(rest, input.body);
            if !sample.owned { break; }
            previous_height = height;
            previous_residual = residual;
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
