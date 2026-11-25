struct ShoreBody {
    stages: vec2<f32>,
    base: u32,
    count: u32,
}
struct SurfRecord {
    controls: vec4<f32>, // lag floor, ratio, cut cell, column tolerance
    bounds: vec4<f32>, // max throw, first throw index, minimum body level, first contour plane
    coarse: vec4<f32>, // fine cells per coarse cell, margin floor, bend factor
    bodies: array<ShoreBody, BODY_LIMIT>,
    shoaling: array<vec4<f32>, SHOALING_ROWS>,
}
@group(SURF_GROUP) @binding(SURF_FIRST) var<uniform> surf: SurfRecord;
@group(SURF_GROUP) @binding(SURF_FIRST + 1u) var surf_lags: texture_2d_array<f32>;
@group(SURF_GROUP) @binding(SURF_FIRST + 2u) var surf_exposures: texture_2d_array<f32>;
@group(SURF_GROUP) @binding(SURF_FIRST + 3u) var breaker_section: texture_2d<f32>;
@group(SURF_GROUP) @binding(SURF_FIRST + 4u) var bed_pyramid: texture_2d<f32>;

const SURF_TAU: f32 = 6.283185307179586;
const SWASH_C: f32 = 0.21213203435596425;

struct LagSample {
    value: f32,
    gradient: vec2<f32>,
    valid: bool,
}
struct ShoreSample {
    lag: f32,
    gradient: vec2<f32>,
    exposure: f32,
    valid: bool,
}
struct StageBlend {
    low: u32,
    high: u32,
    weight: f32,
}

fn surf_stages(body: ShoreBody, stage: f32) -> StageBlend {
    let z = clamp(stage, body.stages.x, body.stages.y);
    let last = body.count - 1u;
    let lo = min(u32(floor((z - body.stages.x) * 2.0)), last);
    let hi = min(lo + 1u, last);
    let low = body.stages.x + 0.5 * f32(lo);
    let high = min(body.stages.y, body.stages.x + 0.5 * f32(hi));
    let weight = select(0.0, (z - low) / max(1e-30, high - low), lo != hi);
    return StageBlend(body.base + lo, body.base + hi, clamp(weight, 0.0, 1.0));
}

fn lag_plane(cell: Cell, stage: u32, sector: u32, rung: u32) -> LagSample {
    let plane = sector * SURF_RUNGS + rung;
    let layer = i32(stage * SURF_LAG_LAYERS + plane / 4u);
    var corners = vec4(0.0);
    var complete = true;
    for (var i = 0u; i < 4u; i++) {
        let lag = textureLoad(surf_lags, cell.corners[i], layer, 0)[plane % 4u];
        if abs(lag) > 3.0e38 {
            if cell.weights[i] > 0.0 { return LagSample(0.0, vec2(0.0), false); }
            complete = false;
        } else {
            corners[i] = lag;
        }
    }
    let gradient = vec2(
        mix(corners.y - corners.x, corners.w - corners.z, cell.uv.y),
        mix(corners.z - corners.x, corners.w - corners.y, cell.uv.x)
    ) / map.origin_spacing_margin.z;
    return LagSample(dot(corners, cell.weights), select(vec2(0.0), gradient, complete), true);
}

fn lag_wavelength(cell: Cell, stage: u32, sector: u32, band: f32) -> LagSample {
    let rung = min(u32(floor(band)), SURF_RUNGS - 2u);
    let u = band - f32(rung);
    if u == 1.0 { return lag_plane(cell, stage, sector, rung + 1u); }
    let low = lag_plane(cell, stage, sector, rung);
    if u == 0.0 || !low.valid { return low; }
    let high = lag_plane(cell, stage, sector, rung + 1u);
    return LagSample(mix(low.value, high.value, u), mix(low.gradient, high.gradient, u), high.valid);
}

fn fitted_lag(cell: Cell, stages: StageBlend, sector: u32, band: f32) -> LagSample {
    if !cell.inside { return LagSample(0.0, vec2(0.0), true); }
    if stages.weight == 1.0 { return lag_wavelength(cell, stages.high, sector, band); }
    let low = lag_wavelength(cell, stages.low, sector, band);
    if stages.weight == 0.0 || !low.valid { return low; }
    let high = lag_wavelength(cell, stages.high, sector, band);
    return LagSample(mix(low.value, high.value, stages.weight), mix(low.gradient, high.gradient, stages.weight), high.valid);
}

fn shore_exposure(cell: Cell, stage: u32, sector: u32) -> f32 {
    let layer = i32(stage * SURF_EXPOSURE_LAYERS + sector / 4u);
    var exposure = 0.0;
    for (var i = 0u; i < 4u; i++) {
        exposure += cell.weights[i] * textureLoad(surf_exposures, cell.corners[i], layer, 0)[sector % 4u];
    }
    return exposure;
}

fn shore_sample(cell: Cell, point: vec2<f32>, stages: StageBlend, direction: u32,
    wavelength: f32, use_gradient: bool) -> ShoreSample {
    if !cell.inside || wavelength < 20.0 || wavelength > 640.0 {
        return ShoreSample(0.0, vec2(0.0), 1.0, cell.inside);
    }
    let sector = direction % SURF_SECTORS;
    let band = log2(wavelength / 20.0);
    var exposure = shore_exposure(cell, stages.low, sector);
    if stages.weight > 0.0 {
        exposure = mix(exposure, shore_exposure(cell, stages.high, sector), stages.weight);
    }
    let base = fitted_lag(cell, stages, sector, band);
    if !base.valid { return ShoreSample(0.0, vec2(0.0), exposure, false); }
    if !use_gradient { return ShoreSample(base.value, vec2(0.0), exposure, true); }
    let step = map.origin_spacing_margin.z * 0.01;
    // The model's symmetric difference equals the bilinear derivative
    // unless it crosses a cell boundary. Only those probes need more reads.
    var gradient = base.gradient;
    for (var axis = 0u; axis < 2u; axis++) {
        if cell.uv[axis] >= 0.01 && cell.uv[axis] <= 0.99 { continue; }
        gradient[axis] = 0.0;
        var offset = vec2(0.0);
        offset[axis] = step;
        let lo = fitted_lag(map_cell(point - offset), stages, sector, band);
        let hi = fitted_lag(map_cell(point + offset), stages, sector, band);
        if lo.valid && hi.valid { gradient[axis] = (hi.value - lo.value) / (2.0 * step); }
    }
    return ShoreSample(base.value, gradient, exposure, true);
}

fn surf_shoaling(k0: f32, depth: f32) -> f32 {
    let kd = k0 * depth;
    if kd >= 20.0 { return 1.0; }
    var x = max(kd, sqrt(kd));
    for (var i = 0u; i < 6u; i++) {
        let t = tanh(x);
        x -= (x * t - kd) / (t + x * (1.0 - t * t));
    }
    let r = 2.0 * x;
    return sqrt(1.0 / (tanh(x) * (1.0 + r / sinh(r))));
}

fn section_row(row: i32, phase: f32) -> vec3<f32> {
    let size = i32(textureDimensions(breaker_section).x);
    let sample = fract(phase / SURF_TAU) * f32(size);
    let i = i32(floor(sample)) % size;
    let a = textureLoad(breaker_section, vec2(i, row), 0).xy;
    let b = textureLoad(breaker_section, vec2((i + 1) % size, row), 0).xy;
    return vec3(mix(a, b, fract(sample)), (b.y - a.y) * f32(size) / SURF_TAU);
}

fn surf_section(phase: f32, index: f32) -> vec3<f32> {
    let end = i32(textureDimensions(breaker_section).y) - 1;
    var lo = 0;
    var hi = end;
    while hi - lo > 1 {
        let i = (lo + hi) / 2;
        if index < textureLoad(breaker_section, vec2(0, i), 0).z {
            hi = i;
        } else {
            lo = i;
        }
    }
    let a = textureLoad(breaker_section, vec2(0, lo), 0).z;
    let b = textureLoad(breaker_section, vec2(0, hi), 0).z;
    return mix(section_row(lo, phase), section_row(hi, phase), clamp((index - a) / (b - a), 0.0, 1.0));
}

fn contour_plane(cell: Cell, stage: u32) -> vec3<f32> {
    let layer = i32(surf.bounds.w) + i32(stage);
    var result = vec3(0.0);
    for (var i = 0u; i < 4u; i++) {
        result += cell.weights[i] * textureLoad(surf_exposures, cell.corners[i], layer, 0).xyz;
    }
    return result;
}

fn shore_contour(cell: Cell, stages: StageBlend) -> vec3<f32> {
    let low = contour_plane(cell, stages.low);
    if stages.weight == 0.0 { return low; }
    return mix(low, contour_plane(cell, stages.high), stages.weight);
}

fn contour_short_weight(depth: f32, swash: f32, coverage: f32, thickness: f32) -> f32 {
    // Short geometry needs clearance in the outer overlap as well as the
    // inner profile. Leave shading normals to the pixel path.
    let depth_weight = smoothstep(0.85, 1.3, depth / swash);
    let clearance = smoothstep(0.12, 0.4, thickness / swash);
    return mix(1.0, depth_weight * clearance, smoothstep(0.0, 1.0, coverage));
}

struct SurfMoments {
    field: vec2<f32>,
    rate: vec2<f32>,
    re_vector: vec2<f32>,
    im_vector: vec2<f32>,
    horizontal: vec2<f32>,
    variance: f32,
}

fn surf_moments(point: vec2<f32>, cell: Cell, stages: StageBlend, transformed: bool,
    floor_depth: f32, fade: f32, gain: f32, anchor: vec2<f32>, dt: f32, chop: f32,
    count: u32, local: bool) -> SurfMoments {
    var result: SurfMoments;
    for (var i = 0u; i < count; i++) {
        let wave = surf_component(i);
        if wave.amplitude == 0.0 { continue; }
        let k0 = 1.0 / wave.inverse_k;
        let wavelength = SURF_TAU * wave.inverse_k;
        var amplitude = wave.amplitude * gain;
        var lag = 0.0;
        var vector = wave.vector;
        if cell.inside && wavelength >= 20.0 && wavelength <= 640.0 {
            var shore = ShoreSample(0.0, vec2(0.0), 1.0, true);
            if transformed {
                shore = shore_sample(cell, point, stages, wave.shore_sector, wavelength, local);
            }
            amplitude *= sqrt(max(0.0, shore.exposure));
            if shore.valid {
                lag = k0 * shore.lag;
                if local {
                    amplitude *= mix(1.0, wave_shoaling(k0, floor_depth), fade);
                    vector += k0 * shore.gradient;
                }
            }
        }
        let phase = dot(wave.vector, point - anchor) + wave.phase - wave.omega * dt + lag;
        let moment = amplitude * vec2(cos(phase), sin(phase));
        result.field += moment;
        result.rate += vec2(moment.y, -moment.x) * wave.omega;
        result.variance += 0.5 * amplitude * amplitude;
        if local {
            result.re_vector += moment.x * vector;
            result.im_vector += moment.y * vector;
            result.horizontal -= chop * moment.y * vector / max(length(vector), 1.0e-20);
        }
    }
    return result;
}

// The still-water contour keeps the approaching phase with no shoaling.
fn contour_phase(sample: SurfMoments, swash: f32) -> vec2<f32> {
    let field = sample.field;
    let rate = sample.rate;
    let variance = sample.variance;
    let envelope = length(field);
    if envelope == 0.0 { return vec2(0.0); }
    let scale = min(1.0, 0.6 * swash / (4.0 * sqrt(variance)));
    let u = clamp(scale * envelope / (SWASH_C * swash), 0.0, 1.0);
    let weight = u * u * (3.0 - 2.0 * u);
    let envelope_rate = dot(field, rate) / envelope;
    let q = field.x / envelope;
    let weight_rate = 6.0 * u * (1.0 - u) * scale * envelope_rate / (SWASH_C * swash);
    return vec2(q * weight, q * weight_rate + weight * (rate.x - q * envelope_rate) / envelope);
}

fn swash_wet_floor(thickness: f32, swash: f32) -> f32 {
    let floor = 0.08 * swash;
    let knee = 0.15 * swash;
    let t = clamp((thickness - floor) / knee, 0.0, 1.0);
    return select(floor + knee * t * t * (2.0 - t), thickness, t == 1.0);
}

struct SurfPoint {
    displacement: vec3<f32>,
    index: f32,
    phase: f32,
    loss: f32,
    variance: f32,
    direction: vec2<f32>,
    section_weight: f32,
    lip: f32,
    short_weight: f32,
    swash_rates: vec4<f32>, // slope xz, upward velocity, height change per metre of depth
    section_throw: vec2<f32>,
    phase_step: f32,
}

fn breaking_strength(sample: SurfPoint) -> f32 {
    return sample.section_weight * smoothstep(1.0, 1.35, sample.index);
}

fn breaking_face(sample: SurfPoint) -> f32 {
    return breaking_strength(sample) * smoothstep(-0.10, 0.30, sample.phase) *
        (1.0 - smoothstep(0.55, 0.90, sample.phase));
}

fn surf_forward(
    point: vec2<f32>, body: u32, row: MapRow, stage: f32, anchor: vec2<f32>,
    dt: f32, chop: f32, heights: vec4<f32>, count: u32,
) -> SurfPoint {
    var empty: SurfPoint;
    empty.short_weight = 1.0;
    if row.gains.x == 0.0 || heights.x == 0.0 || count == 0u { return empty; }
    let cell = map_cell(point);
    let depth = select(1.0e30, row.water.x - cell.bed, cell.inside);
    let floor_depth = max(depth, surf.controls.x);
    let swash = min(map.swash.x * length(heights * row.gains), map.origin_spacing_margin.w);
    if depth + swash <= 0.0 {
        // With no remaining height and the section faded out, the input
        // variance is the plain layer's. Keep it for interpolation at the
        // wet edge, where the bed cuts the distant foam estimate.
        let rms = heights.x * row.gains.x * 0.25;
        empty.variance = rms * rms;
        return empty;
    }
    let draw = SWASH_C * swash / (1.0 - SWASH_C);
    var fade = f32(depth > 0.0);
    if swash > 0.0 { fade = smoothstep(draw, 0.5 * swash, depth); }
    let record = surf.bodies[body];
    let transformed = cell.inside && record.count > 0u;
    var stages: StageBlend;
    if transformed { stages = surf_stages(record, stage); }
    let ratio = depth / max(swash, 1e-20);
    var contour = vec3(0.0);
    if transformed && swash > 0.0 && depth < 4.0 * map.origin_spacing_margin.w {
        contour = shore_contour(cell, stages);
    }
    let support = smoothstep(0.0, 1.0, contour.z);
    let inner = support * (1.0 - smoothstep(0.35, 0.85, ratio));
    var moments: array<SurfMoments, 2>;
    let count_stations = select(1u, 2u, inner > 0.0);
    for (var station = 0u; station < count_stations; station++) {
        let local = station == 0u;
        var position = point;
        var at = cell;
        if !local {
            position += contour.xy / contour.z;
            at = map_cell(position);
        }
        moments[station] = surf_moments(position, at, stages, transformed && at.inside,
            floor_depth, fade, row.gains.x, anchor, dt, chop, count, local);
    }
    let ordinary = moments[0];
    let field = ordinary.field;
    let variance = ordinary.variance;
    var horizontal = ordinary.horizontal;
    let amplitude = length(field);
    let phase = atan2(field.y, field.x);
    let index = 2.0 * amplitude / (surf.controls.y * floor_depth);
    let scale = min(1.0, 0.6 * max(0.0, depth + swash) / max(4.0 * sqrt(variance), 1.0e-30));
    let gradient = field.x * ordinary.re_vector + field.y * ordinary.im_vector;
    let gradient_length = length(gradient);
    let direction = gradient / max(gradient_length, 1.0e-20);
    let phase_step = gradient_length / max(dot(field, field), 1.0e-20);
    var height = field.x;
    var lip = 0.0;
    var section_throw = vec2(0.0);
    if fade > 0.0 && index > 0.0 {
        let section = surf_section(phase, index);
        height = mix(height, section.x * amplitude, fade);
        section_throw = section.y * amplitude * fade * direction;
        horizontal += section_throw;
        // Project the front-face tangent along its local direction of travel.
        // A forward departure alone also occurs on sections that never fold.
        let tangent = 1.0 + phase_step * amplitude * scale *
            (fade * section.z - chop * cos(phase));
        lip = smoothstep(0.0, 0.35, -tangent) * f32(section.y > 0.0);
    }
    // The swash is below the section's fade interval. Its leading-order
    // slope comes from the arriving phase and the local depth limiter.
    var height_per_depth = select(0.0, scale * height / max(depth + swash, 1e-20), scale < 1.0);
    var slope = -scale * ordinary.im_vector;
    var height_dt = scale * ordinary.rate.x;
    height *= scale;
    horizontal *= scale;
    section_throw *= scale;
    var short_weight = 1.0;
    if support > 0.0 {
        horizontal *= 1.0 - inner;
        section_throw *= 1.0 - inner;
        if inner > 0.0 {
            let q = contour_phase(moments[1], swash);
            let factor = SWASH_C * (depth + swash);
            height = mix(height, factor * q.x, inner);
            height_dt = mix(height_dt, factor * q.y, inner);
            height_per_depth = mix(height_per_depth, SWASH_C * q.x, inner);
            slope *= 1.0 - inner;
            fade *= 1.0 - inner;
            lip *= 1.0 - inner;
        }
        var thickness = depth + height;
        if ratio > 0.35 {
            let displaced = map_cell(point + horizontal);
            if displaced.inside {
                thickness = row.water.x - displaced.bed + height;
                let merge = support * smoothstep(0.35, 0.45, ratio);
                let lift = merge * (swash_wet_floor(thickness, swash) - thickness);
                height += lift;
                thickness += lift;
            }
        }
        short_weight = contour_short_weight(depth, swash, contour.z, thickness);
    }
    return SurfPoint(vec3(horizontal.x, height, horizontal.y), index, phase,
        variance * (1.0 - scale * scale), variance, direction, fade, lip, short_weight,
        vec4(slope, height_dt, height_per_depth), section_throw, phase_step);
}
