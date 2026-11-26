struct ShoreBirth {
    // Film and air before redistribution, share eligible to move, front-face weight.
    rates: vec4<f32>,
    // Direction to the receiving neighbour, local phase, folded-lip weight.
    face: vec4<f32>,
    body: u32,
    wet: u32,
    uprush: vec2<f32>,
}
@group(0) @binding(17) var<storage, read_write> shore_births: array<ShoreBirth>;

fn surf_component(index: u32) -> Component { return column_component(index); }

@compute @workgroup_size(8, 8)
fn prepare_shore_births(@builtin(global_invocation_id) id: vec3<u32>) {
    let level = id.z;
    let local = vec2<i32>(id.xy);
    let spacing = FOAM_SPACING[level];
    let point = (vec2<f32>(foam_settings.origins[level].xy + local) + 0.5) * spacing;
    let answer = map_sample(point);
    var birth: ShoreBirth;
    birth.body = answer.body;
    if answer.has_bed && answer.body != NO_BODY {
        let depth = answer.row.water.x - answer.bed;
        let before = answer.row.gains * water.heights;
        let swash = min(map.swash.x * length(before), map.origin_spacing_margin.w);
        let cap = 0.6 * max(0.0, depth + swash);
        let floor_depth = max(depth, surf.controls.x);
        // For x tanh(x) = kd, x >= sqrt(kd) and tanh(x) >= x / sqrt(1 + x*x).
        // Dropping the group-velocity correction gives this shoaling ceiling.
        let kd = (SURF_TAU / 640.0) * floor_depth;
        let amplification = sqrt(sqrt(1.0 + 1.0 / kd));
        let long_crest = foam_settings.long_bounds.x * answer.row.gains.x * amplification;
        let can_break = 2.0 * long_crest > surf.controls.y * floor_depth;
        let can_lose_power = any(before.yzw > vec3(cap)) || before.x * amplification > cap;
        if (can_break || can_lose_power) && any(before > vec4(0.0)) {
            let gains = limit_gains(answer.row.gains, depth, water.heights, map.swash.x,
                map.origin_spacing_margin.w);
            let dt = foam_settings.camera.z;
            let stage = map.bodies[answer.body].level.y;
            let chop = foam_settings.camera.w;
            let sample = surf_forward(point, answer.body, answer.row, stage, foam_settings.camera.xy,
                dt, chop, water.heights, foam_settings.banks.w);
            let short = foam_column(point, vec4(0.0, gains.yzw * sample.short_weight), dt, answer.row.water.yz);
            let wake = wake_at(point, dt);
            let thickness = depth + sample.displacement.y + short.displacement.y + wake.x;
            let wet = thickness > 0.0;
            birth.wet = select(1u, 2u, wet);
            if wet {
                if level == 0u && swash > 0.0 && depth < 0.27 * swash {
                    let bed_slope = swash_bed_slope(map_cell(point));
                    let surface_slope = short.slope + wake.yz + sample.swash_rates.xy
                        - sample.swash_rates.w * bed_slope;
                    let upward = short.velocity.y + wake.w + sample.swash_rates.z;
                    birth.uprush = swash_drift(surface_slope - bed_slope, upward, depth, swash);
                }
                let short_loss = lost_wave_power(vec4(0.0, answer.row.gains.yzw),
                    vec4(0.0, gains.yzw), water.heights);
                let distributed = 0.39 * (short_loss + sample.loss);
                // Phase increases shoreward. A falling face lies just ahead
                // of its crest; restrict neighbours to that same face.
                let direction = round(sample.direction * select(-1.0, 1.0, sample.phase < 0.55));
                birth.rates = vec4(distributed, distributed * 0.22,
                    sample.loss * 0.39 * breaking_strength(sample), breaking_face(sample));
                birth.face = vec4(direction, sample.phase, sample.lip);
            }
        }
    }
    shore_births[foam_index(0u, level, local)] = birth;
}

fn moved_shore_birth(source: ShoreBirth, destination: ShoreBirth) -> vec2<f32> {
    if source.body != destination.body || destination.wet != 2u { return vec2(0.0); }
    let difference = destination.face.z - source.face.z;
    if abs(difference) >= 0.85 { return vec2(0.0); }
    let face = destination.rates.w - source.rates.w;
    let lip = destination.rates.w * destination.face.w - source.rates.w * source.face.w;
    return source.rates.z * vec2(0.65, 0.20) * clamp(2.0 * vec2(face, lip), vec2(0.0), vec2(1.0));
}

fn shore_rates(level: u32, local: vec2<i32>, distributed: f32) -> vec2<f32> {
    let source = shore_births[foam_index(0u, level, local)];
    if source.wet == 0u { return vec2(distributed, 0.22 * distributed); }
    if source.wet != 2u { return vec2(0.0); }
    var rates = source.rates.xy;
    let forward = local + vec2<i32>(source.face.xy);
    if source.rates.z > 0.0 && foam_inside(forward) {
        let moved = moved_shore_birth(source, shore_births[foam_index(0u, level, forward)]);
        rates.x -= moved.x + moved.y;
    }
    if source.rates.w > 0.0 {
        for (var z = -1; z <= 1; z++) {
            for (var x = -1; x <= 1; x++) {
                let offset = vec2(x, z);
                let neighbour = local + offset;
                if all(offset == vec2(0)) || !foam_inside(neighbour) { continue; }
                let other = shore_births[foam_index(0u, level, neighbour)];
                if all(vec2<i32>(other.face.xy) == -offset) {
                    rates += moved_shore_birth(other, source);
                }
            }
        }
    }
    return rates;
}

fn foam_wet(level: u32, local: vec2<i32>, plain: bool) -> bool {
    let wet = shore_births[foam_index(0u, level, local)].wet;
    return select(wet == 2u, plain, wet == 0u);
}

fn stranded_shore(local: vec2<i32>) -> bool {
    return shore_births[foam_index(0u, 0u, local)].wet == 1u;
}

fn shore_uprush(level: u32, local: vec2<i32>, plain: vec2<f32>) -> vec2<f32> {
    let sample = shore_births[foam_index(0u, level, local)];
    return select(sample.uprush, plain, sample.wet == 0u);
}
