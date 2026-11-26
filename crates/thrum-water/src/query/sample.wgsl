struct QueryPoint {
    position: vec2<f32>, radius: f32, dt: f32,
    level_current_bed: vec4<f32>,
    slope: vec2<f32>, body: u32, flags: u32,
    gains: vec4<f32>,
    dent: vec4<f32>,
    surf_stage: vec4<f32>,
    fine_bounds: vec4<i32>,
    coarse_grid: vec4<i32>,
    carry: vec4<f32>,
}
struct Settings {
    origin: vec2<f32>, count: u32, components: u32,
    lengths_chop: vec4<f32>, heights: vec4<f32>, swash_margin: vec4<f32>,
    coarse_jobs: vec4<u32>,
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
    cut_fine_nodes: u32,
    cut_coarse_nodes: u32,
    padding: array<u32, 3>,
    wake: vec4<f32>,
}
@group(0) @binding(0) var<uniform> settings: Settings;
@group(0) @binding(1) var<storage, read> points: array<QueryPoint>;
@group(0) @binding(2) var<storage, read> components: array<Component>;
@group(0) @binding(3) var<storage, read_write> answers: array<Answer>;
@group(0) @binding(4) var cascades: texture_2d_array<f32>;

fn column_component(index: u32) -> Component { return components[index]; }

fn cascade_wave(point: QueryPoint, band: u32) -> Wave {
    let extent = settings.lengths_chop[band];
    let lod = clamp(log2(max(1.0, point.radius * 512.0 / extent)), 0.0, 8.0);
    return carried_column(point.position, point.level_current_bed.yz, settings.swash_margin.zw,
        extent, band, lod);
}

struct QueryWave { wave: Wave, cut_fine_nodes: u32, cut_coarse_nodes: u32, }

const QUERY_WIDTH = 64u;

fn query_gains(point: QueryPoint) -> vec4<f32> {
    let row = point.level_current_bed;
    let depth = select(1e30, row.x - row.w, (point.flags & 1u) != 0u);
    return limit_gains(point.gains, depth, settings.heights, settings.swash_margin.x, settings.swash_margin.y);
}

fn plain_query(point: QueryPoint, gain: f32) -> QueryWave {
    return QueryWave(long_wave(point.position - settings.origin, point.dt, gain,
        settings.lengths_chop.w, settings.components), 0u, 0u);
}

fn write_answer(index: u32, point: QueryPoint, gains: vec4<f32>, result: QueryWave) {
    let row = point.level_current_bed;
    var out: Answer;
    out.body_flags = vec4(point.body, 0u, point.flags & 1u, 0u);
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
        let depth = select(1e30, row.x - row.w, (point.flags & 1u) != 0u);
        let long = result.wave;
        out.cut_fine_nodes = result.cut_fine_nodes;
        out.cut_coarse_nodes = result.cut_coarse_nodes;
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
                out.layers[band + 1u] = gain * wave.displacement.y + point.carry.x * band_rate;
                slope += band_slope;
                velocity += gain * wave.velocity;
                rate += band_rate;
            }
        }
        out.wake = wake_at(point.position, min(point.carry.y, 0.0)) + point.dent;
        out.wake.x += max(point.carry.y, 0.0) * out.wake.w;
        if any(out.wake != vec4(0.0)) { out.body_flags.w |= 16u; }
        let height = row.x + dot(out.layers, vec4(1.0)) + out.wake.x;
        slope += out.wake.yz;
        rate += out.wake.w;
        velocity.y += out.wake.w;
        out.height_slope_rate = vec4(height, slope, rate);
        out.velocity_depth = vec4(velocity, depth);
        out.body_flags.y = u32((point.flags & 1u) == 0u || height > row.w);
        for (var layer = 0u; layer < 4u; layer++) {
            if gains[layer] * settings.heights[layer] > 0.0 { out.body_flags.w |= 1u << layer; }
        }
    }
    answers[index] = out;
}
