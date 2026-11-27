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
