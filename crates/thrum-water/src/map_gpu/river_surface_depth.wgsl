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
    // The column's map row fixes its gain for the whole inverse, as in the
    // body query. The displaced mesh's rest coordinate can carry a different
    // level or gain, especially where swell crosses a shallow tidal reach.
    let lod = long_lod(point, spacing);
    let low = u32(lod);
    let blend = fract(lod);
    let gain = row.gains.x;
    var offset = vec2(0.0);
    for (var i = 0u; i < 12u; i++) {
        let rest = point + offset;
        var wave = long_level(rest, low);
        if blend > 0.0 {
            wave = add_field(add_field(Field(vec4(0.0), vec4(0.0), vec4(0.0)),
                wave, 1.0 - blend), long_level(rest, low + 1u), blend);
        }
        let residual = -gain * wave.displacement.xz - offset;
        let error = length(residual);
        if error < 0.00001 { break; }
        // Use the cached horizontal derivative to solve the two-dimensional
        // inverse. Limit the update near a compressed crest.
        let j = vec3(1.0, 0.0, 1.0) + gain * vec3(wave.slope.zw, wave.velocity.w);
        let det = max(0.01, j.x * j.z - j.y * j.y);
        let step = solve_horizontal(j.xy, j.yz, residual, det);
        offset += step * min(1.0, 4.0 * error / max(length(step), 0.00001));
    }
    let rest = point + offset;
    var height = river_swell_level(rest, low);
    if blend > 0.0 {
        height = mix(height, river_swell_level(rest, low + 1u), blend);
    }
    let fade = 1.0 - smoothstep(4000.0, 8000.0, length(rest - view.camera_xyz_time.xz));
    return height * row.gains.x * fade;
}
