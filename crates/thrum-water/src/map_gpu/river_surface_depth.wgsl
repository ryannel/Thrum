// Recover the rounding errors in a projected sum. A close camera on an
// island hundreds of metres from the origin otherwise loses depth precision
// when its translation nearly cancels the world position.
fn river_sum_error(a: vec4<f32>, b: vec4<f32>, sum: vec4<f32>) -> vec4<f32> {
    let b_part = sum - a;
    return (a - (sum - b_part)) + (b - b_part);
}

fn river_clip(world: vec3<f32>) -> vec4<f32> {
    let m = view.view_proj;
    let x = m[0] * world.x;
    let y = m[1] * world.y;
    let z = m[2] * world.z;
    let a = m[3] + x;
    let b = a + y;
    let c = b + z;
    let product_error = fma(m[0], vec4(world.x), -x)
        + fma(m[1], vec4(world.y), -y) + fma(m[2], vec4(world.z), -z);
    return c + (product_error + river_sum_error(m[3], x, a)
        + river_sum_error(a, y, b) + river_sum_error(b, z, c));
}

fn cross2(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

fn solve_horizontal(a: vec2<f32>, b: vec2<f32>, rhs: vec2<f32>, det: f32) -> vec2<f32> {
    return vec2(cross2(rhs, b), cross2(a, rhs)) / det;
}

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
    // Derivatives describe the rasterized triangle's map back to rest space.
    let x = dpdx(input.world);
    let y = dpdy(input.world);
    let rest_x = dpdx(input.rest);
    let rest_y = dpdy(input.rest);
    let rebuild_swell = input.surf_weight == 0.0;
    let mesh_level = input.river.x + select(0.0, input.river.y, rebuild_swell);
    let retained = input.world.y - mesh_level;
    let retained_gradient = vec2(dpdx(retained), dpdy(retained));
    let mesh_gradient = vec2(x.y, y.y);
    let mesh_det = cross2(x.xz, y.xz);
    var surface = input;
    var row = surface_pixel(input.rest, input.body);
    var depth = input.clip.z;
    let weight = river_refinement(input.rest, surface_body(input.body), input.river.z);
    let ndc = fma(input.clip.xy * view_aux.viewport_inv_viewport.zw, vec2(2.0, -2.0), vec2(-1.0, 1.0));
    let m = view.view_proj;
    let a = fma(-ndc, vec2(m[0].w), m[0].xy);
    let b = fma(-ndc, vec2(m[2].w), m[2].xy);
    let det = cross2(a, b);
    if weight > 0.0 && row.owned && abs(det) > 0.001 && abs(mesh_det) > 1e-12 {
        // At a fixed height, the pixel's clip x/w and y/w are two linear
        // constraints on world x/z. Work in offsets from the mesh hit.
        let clip = river_clip(input.world);
        let error = fma(-ndc, vec2(clip.w), clip.xy);
        let origin = solve_horizontal(a, b, -error, det);
        let vertical = fma(-ndc, vec2(m[1].w), m[1].xy);
        let direction = solve_horizontal(a, b, -vertical, det);
        var height = 0.0;
        var previous_height = 0.0;
        var previous_residual = 0.0;
        for (var i = 0u; i < 8u; i++) {
            let shift = fma(direction, vec2(height), origin);
            if length(vec3(shift, height)) > 0.25 { break; }
            let screen = solve_horizontal(x.xz, y.xz, shift, mesh_det);
            let rest = input.rest + screen.x * rest_x + screen.y * rest_y;
            let sample = surface_pixel(rest, input.body);
            if !sample.owned { break; }
            var physical_level = sample.level;
            if rebuild_swell { physical_level += river_swell(rest, input.river.z, sample); }
            let raster_height = dot(mesh_gradient, screen);
            let physical_height = (physical_level - mesh_level) + dot(retained_gradient, screen);
            let residual = mix(raster_height, physical_height, weight) - height;
            // Commit only intersections that stay within this body's coverage.
            surface.rest = rest;
            surface.world = input.world + vec3(shift.x, height, shift.y);
            row = sample;
            let corrected = fma(m[2].zw, vec2(shift.y),
                fma(m[1].zw, vec2(height), fma(m[0].zw, vec2(shift.x), clip.zw)));
            depth = clamp(corrected.x / corrected.y, 0.0, 1.0);
            if abs(residual) <= 0.000002 { break; }
            // Use the prior reading to estimate the residual's slope along
            // an oblique ray while limiting the step at cell creases.
            var step = residual;
            if i > 0u {
                let drop = previous_residual - residual;
                let rise = height - previous_height;
                if drop * rise > 0.0 { step *= clamp(rise / drop, 0.25, 4.0); }
            }
            previous_height = height;
            previous_residual = residual;
            height += step;
        }
    }
    let shaded = shade_water(surface, row);
    return RiverOutput(shaded.colour, shaded.motion, shaded.reactive, depth);
}
