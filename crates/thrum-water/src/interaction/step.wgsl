struct Step {
    shift: vec2<i32>,
    current: vec2<f32>,
    old_base: u32,
    new_base: u32,
    cell_base: u32,
    region: u32,
    source_first: u32,
    source_count: u32,
    depth_index: u32,
    level: u32,
    metres: f32,
    depth: f32,
    seconds: f32,
    ticks: f32,
}
struct Cell { region: u32, low: u32, high: u32, blend: f32, shore_loss: f32 }
struct Patch {
    place: vec2<f32>,
    axis: vec2<f32>,
    footprint: vec2<f32>,
    impulse: f32,
    padding: f32,
}
@group(0) @binding(0) var<uniform> step: Step;
@group(0) @binding(1) var<storage, read_write> endpoints: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> work: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> cells: array<Cell>;
@group(0) @binding(4) var<storage, read> patches: array<Patch>;

const AREA = WAKE_SIDE * WAKE_SIDE;
const TAU = 6.283185307179586;
var<workgroup> row: array<vec4<f32>, WAKE_SIDE>;

fn rotate(v: vec4<f32>, rotation: vec2<f32>) -> vec4<f32> {
    return v * rotation.x + v.yxwz * vec4(-1.0, 1.0, -1.0, 1.0) * rotation.y;
}

fn old_state(at: vec2<u32>) -> vec2<f32> {
    let old = vec2<i32>(at) + step.shift;
    if any(old < vec2(0)) || any(old >= vec2(i32(WAKE_SIDE))) {
        return vec2(0.0);
    }
    return endpoints[step.old_base + u32(old.y) * WAKE_SIDE + u32(old.x)];
}

// Radix two, one row per workgroup. Each output is transposed so the same
// dispatch turns its columns into contiguous rows. Two transforms restore
// the original layout. Channel pairs carry h and phi as complex numbers.
fn transform(lane: u32, source: u32, destination: u32, line: u32, inverse: bool, load: bool) {
    for (var i = lane; i < WAKE_SIDE; i += 64u) {
        let reverse = reverseBits(i) >> 24u;
        let index = line * WAKE_SIDE + reverse;
        if load {
            let cell = cells[step.cell_base + index];
            var state = vec2(0.0);
            if cell.region == step.region {
                state = old_state(vec2(reverse, line));
            }
            row[i] = vec4(state.x, 0.0, state.y, 0.0);
        } else {
            row[i] = work[source + index];
        }
    }
    workgroupBarrier();
    let direction = select(-1.0, 1.0, inverse);
    for (var half = 1u; half < WAKE_SIDE; half *= 2u) {
        for (var pair = lane; pair < WAKE_SIDE / 2u; pair += 64u) {
            let j = pair % half;
            let lo = 2u * (pair - j) + j;
            let hi = lo + half;
            let phase = direction * TAU * f32(j) / f32(2u * half);
            let a = row[lo];
            let b = rotate(row[hi], vec2(cos(phase), sin(phase)));
            row[lo] = a + b;
            row[hi] = a - b;
        }
        workgroupBarrier();
    }
    let scale = select(1.0, 1.0 / f32(WAKE_SIDE), inverse);
    for (var i = lane; i < WAKE_SIDE; i += 64u) {
        work[destination + i * WAKE_SIDE + line] = scale * row[i];
    }
}

@compute @workgroup_size(64)
fn forward_row(@builtin(local_invocation_index) lane: u32, @builtin(workgroup_id) group: vec3<u32>) {
    transform(lane, 0u, 0u, group.x, false, true);
}
@compute @workgroup_size(64)
fn forward_column(@builtin(local_invocation_index) lane: u32, @builtin(workgroup_id) group: vec3<u32>) {
    transform(lane, 0u, AREA, group.x, false, false);
}
@compute @workgroup_size(64)
fn inverse_row(@builtin(local_invocation_index) lane: u32, @builtin(workgroup_id) group: vec3<u32>) {
    transform(lane, 0u, 2u * AREA, group.x, true, false);
}
@compute @workgroup_size(64)
fn inverse_column(@builtin(local_invocation_index) lane: u32, @builtin(workgroup_id) group: vec3<u32>) {
    transform(lane, 2u * AREA, 0u, group.x, true, false);
}

@compute @workgroup_size(64)
fn advance(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let index = invocation.x;
    let bin = vec2<i32>(i32(index % WAKE_SIDE), i32(index / WAKE_SIDE));
    let signed = select(bin, bin - vec2(i32(WAKE_SIDE)), bin >= vec2(i32(WAKE_SIDE / 2u)));
    let grid_k = vec2<f32>(signed) * (TAU / f32(WAKE_SIDE));
    let k = grid_k / step.metres;
    let magnitude = length(k);
    let divider = TAU / WAKE_DIVIDER_METRES;
    let short = smoothstep(0.8 * divider, 1.2 * divider, magnitude);
    let level = step.level & ~FINE_SPLIT;
    let fine = level == 2u;
    let band = select(short, 1.0 - short, level == 1u);
    let divider_fine = TAU / FINE_DIVIDER_METRES;
    let short_fine = smoothstep(0.8 * divider_fine, 1.2 * divider_fine, magnitude);
    let share = select(
        band,
        select(band - short_fine, short_fine, fine),
        (step.level & FINE_SPLIT) != 0u && level != 1u
    );
    // Taper the Nyquist bins; fractional translations of unpaired bins
    // otherwise make a real pressure or a real state complex.
    let nyquist = 1.0 - smoothstep(2.7, 3.14159265, max(abs(grid_k.x), abs(grid_k.y)));
    var impulse = vec2(0.0);
    if share > 0.0 && magnitude > 0.0 {
        for (var i = 0u; i < step.source_count; i++) {
            let source = patches[step.source_first + i];
            let across = vec2(-source.axis.y, source.axis.x);
            let q = vec2(dot(k, source.axis), dot(k, across)) * source.footprint;
            let phase = -dot(grid_k, source.place);
            let strength = -source.impulse * exp(-0.5 * dot(q, q));
            impulse += strength * vec2(cos(phase), sin(phase));
        }
        impulse *= 0.5 * share * nyquist / (WATER_DENSITY * step.metres * step.metres);
    }
    let start = work[AREA + index];
    let h = start.xy;
    let p = start.zw + impulse;
    // tanh has rounded to 1 above this argument. Keep it away from
    // implementations that evaluate the hyperbolic ratio using exp(2x).
    let dispersion = magnitude * select(tanh(min(magnitude * step.depth, 9.0)), 1.0, step.depth == 0.0);
    let restoring = WATER_GRAVITY + select(0.0, CAPILLARY_COEFFICIENT * magnitude * magnitude, fine);
    let omega = sqrt(restoring * dispersion);
    let angle = step.seconds * omega;
    let cosine = cos(angle);
    var sinc = step.seconds;
    if omega > 0.0 {
        sinc = sin(angle) / omega;
    }
    let travel = -dot(k, step.current) * step.seconds;
    let state = vec4(cosine * h + dispersion * sinc * p, cosine * p - restoring * sinc * h);
    work[index] = rotate(state, vec2(cos(travel), sin(travel))) + vec4(0.0, 0.0, impulse);
}

@compute @workgroup_size(64)
fn blend_depth(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let index = invocation.x;
    let cell = cells[step.cell_base + index];
    if cell.region != step.region {
        return;
    }
    let at = vec2(index % WAKE_SIDE, index / WAKE_SIDE);
    let edge = min(at, vec2(WAKE_SIDE - 1u) - at);
    let border = smoothstep(0.0, f32(WAKE_SIDE) / 8.0, f32(min(edge.x, edge.y)));
    let shore = select(1.0, cell.blend * cell.blend, cell.low == REST_DEPTH);
    let decay = pow(border * shore * 0.997, step.ticks) * exp(-cell.shore_loss * step.seconds);
    var weight = 0.0;
    if cell.low == step.depth_index {
        weight += 1.0 - cell.blend;
    }
    if cell.high == step.depth_index {
        weight += cell.blend;
    }
    var value = work[index].xz * weight;
    if cell.low == REST_DEPTH && cell.high == step.depth_index {
        let old = old_state(at);
        value += vec2(old.x, old.y - WATER_GRAVITY * step.seconds * old.x) * (1.0 - cell.blend);
    }
    endpoints[step.new_base + index] += decay * value;
}
