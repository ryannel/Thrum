struct QueryPoint {
    position: vec2<f32>, radius: f32, dt: f32,
    level_current_bed: vec4<f32>,
    slope: vec2<f32>, body: u32, has_bed: u32,
    gains: vec4<f32>,
}
struct Component { vector: vec2<f32>, amplitude: f32, phase: f32, omega: f32, inverse_k: f32, padding: vec2<f32> }
struct Settings {
    origin: vec2<f32>, count: u32, components: u32,
    lengths_chop: vec4<f32>, heights: vec4<f32>, swash_margin: vec4<f32>,
}
struct CascadeSample {
    height: f32,
    slope: array<f32, 2>,
    differential: array<f32, 3>,
    velocity: array<f32, 3>,
}
struct Answer {
    height_slope_rate: vec4<f32>,
    velocity_depth: vec4<f32>,
    layers: vec4<f32>,
    current_bed: vec4<f32>,
    body_flags: vec4<u32>,
    cascades: array<CascadeSample, 3>,
    padding: f32,
}
struct Wave {
    displacement: vec3<f32>,
    slope: vec2<f32>,
    differential: vec3<f32>,
    velocity: vec3<f32>,
}
@group(0) @binding(0) var<uniform> settings: Settings;
@group(0) @binding(1) var<storage, read> points: array<QueryPoint>;
@group(0) @binding(2) var<storage, read> components: array<Component>;
@group(0) @binding(3) var<storage, read_write> answers: array<Answer>;
@group(0) @binding(4) var cascades: texture_2d_array<f32>;

fn world_slope(wave: Wave) -> vec2<f32> {
    let j = wave.differential + vec3(1.0, 0.0, 1.0);
    let det = max(0.01, j.x * j.z - j.y * j.y);
    return vec2(j.z * wave.slope.x - j.y * wave.slope.y, j.x * wave.slope.y - j.y * wave.slope.x) / det;
}

fn long_wave(point: QueryPoint, gain: f32) -> Wave {
    var wave: Wave;
    if gain == 0.0 { return wave; }
    let p = point.position - settings.origin;
    let chop = settings.lengths_chop.w;
    var offset = vec2(0.0);
    // The profile admits a contraction below 0.9. Solve in local offsets so
    // adding a submillimetre correction does not round at world coordinates.
    for (var iteration = 0u; iteration < 192u; iteration++) {
        var next = vec2(0.0);
        for (var i = 0u; i < settings.components; i++) {
            let c = components[i];
            let phase = dot(c.vector, p) + c.phase - c.omega * point.dt + dot(c.vector, offset);
            next += (gain * chop * c.amplitude * c.inverse_k * sin(phase)) * c.vector;
        }
        let residual = length(next - offset);
        offset = next;
        if residual < 0.00001 { break; }
    }
    for (var i = 0u; i < settings.components; i++) {
        let c = components[i];
        let k = c.vector;
        let phase = dot(k, p) + c.phase - c.omega * point.dt + dot(k, offset);
        let h = gain * c.amplitude * cos(phase);
        let s = gain * c.amplitude * sin(phase);
        let direction = chop * c.inverse_k * k;
        wave.displacement.y += h;
        wave.slope -= s * k;
        wave.differential -= h * chop * c.inverse_k * vec3(k.x * k.x, k.x * k.y, k.y * k.y);
        wave.velocity += c.omega * vec3(direction.x * h, s, direction.y * h);
    }
    return wave;
}

fn periodic_texel(uv: vec2<f32>, layer: i32, mip: i32) -> vec4<f32> {
    // Query tolerances apply to the exact box mips. Hardware uses an
    // implementation-defined precision for the interpolation weights.
    let extent = vec2<i32>(textureDimensions(cascades, mip));
    let coord = fract(uv) * vec2<f32>(extent) - 0.5;
    let corner = vec2<i32>(floor(coord));
    let weight = fract(coord);
    let mask = extent - 1;
    let a = textureLoad(cascades, corner & mask, layer, mip);
    let b = textureLoad(cascades, (corner + vec2(1, 0)) & mask, layer, mip);
    let c = textureLoad(cascades, (corner + vec2(0, 1)) & mask, layer, mip);
    let d = textureLoad(cascades, (corner + 1) & mask, layer, mip);
    return mix(mix(a, b, weight.x), mix(c, d, weight.x), weight.y);
}

fn periodic_field(uv: vec2<f32>, layer: i32, lod: f32) -> vec4<f32> {
    let low = i32(lod);
    let a = periodic_texel(uv, layer, low);
    if lod == f32(low) { return a; }
    return mix(a, periodic_texel(uv, layer, low + 1), fract(lod));
}

fn cascade_wave(point: QueryPoint, band: u32) -> Wave {
    let extent = settings.lengths_chop[band];
    let uv = point.position / extent;
    let layer = i32(band * 5u);
    let lod = clamp(log2(max(1.0, point.radius * 512.0 / extent)), 0.0, 8.0);
    let displacement = periodic_field(uv, layer, lod).xyz
        + periodic_field(uv, layer + 4, lod).yzw;
    let slope = periodic_field(uv, layer + 1, lod);
    let velocity = periodic_field(uv, layer + 2, lod);
    return Wave(displacement, slope.xy, vec3(slope.zw, velocity.w), velocity.xyz);
}

@compute @workgroup_size(64)
fn sample_query(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let index = invocation.x;
    if index >= settings.count { return; }
    let point = points[index];
    let row = point.level_current_bed;
    var out: Answer;
    out.body_flags = vec4(point.body, 0u, point.has_bed, 0u);
    out.current_bed = vec4(row.yzw, 0.0);
    // A stage change can make a dry or sheltered point gain wave support
    // before its use tick. Keep the actual frame samples for that case.
    var bands: array<Wave, 3>;
    for (var band = 0u; band < 3u; band++) {
        if settings.heights[band + 1u] > 0.0 {
            let wave = cascade_wave(point, band);
            bands[band] = wave;
            out.cascades[band] = CascadeSample(
                wave.displacement.y,
                array(wave.slope.x, wave.slope.y),
                array(wave.differential.x, wave.differential.y, wave.differential.z),
                array(wave.velocity.x, wave.velocity.y, wave.velocity.z),
            );
        }
    }
    if point.body != 0xffffffffu {
        let depth = select(1e30, row.x - row.w, point.has_bed != 0u);
        let gains = limit_gains(point.gains, depth, settings.heights, settings.swash_margin.x, settings.swash_margin.y);
        let long = long_wave(point, gains.x);
        let long_slope = world_slope(long);
        var rate = long.velocity.y - dot(long_slope, long.velocity.xz);
        var slope = point.slope + long_slope;
        var velocity = long.velocity + vec3(row.y, 0.0, row.z);
        out.layers.x = long.displacement.y;
        for (var band = 0u; band < 3u; band++) {
            let gain = gains[band + 1u];
            if gain > 0.0 {
                let wave = bands[band];
                let band_slope = world_slope(Wave(wave.displacement, gain * wave.slope, gain * wave.differential, wave.velocity));
                // The cascades are sampled at a fixed column in their periodic lattice.
                let band_rate = gain * wave.velocity.y;
                out.layers[band + 1u] = gain * wave.displacement.y + point.dt * band_rate;
                slope += band_slope;
                velocity += gain * wave.velocity;
                rate += band_rate;
            }
        }
        let height = row.x + dot(out.layers, vec4(1.0));
        out.height_slope_rate = vec4(height, slope, rate);
        out.velocity_depth = vec4(velocity, depth);
        out.body_flags.y = u32(point.has_bed == 0u || height > row.w);
        for (var layer = 0u; layer < 4u; layer++) {
            if gains[layer] * settings.heights[layer] > 0.0 { out.body_flags.w |= 1u << layer; }
        }
    }
    answers[index] = out;
}
