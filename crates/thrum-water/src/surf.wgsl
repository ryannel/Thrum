struct ShoreBody {
    stages: vec2<f32>,
    base: u32,
    count: u32,
}
struct SurfRecord {
    controls: vec4<f32>, // lag floor, ratio, cut cell, column tolerance
    bounds: vec4<f32>, // max normalized throw, first throw index, minimum body level
    bodies: array<ShoreBody, BODY_LIMIT>,
}
@group(SURF_GROUP) @binding(SURF_FIRST) var<uniform> surf: SurfRecord;
@group(SURF_GROUP) @binding(SURF_FIRST + 1u) var surf_lags: texture_2d_array<f32>;
@group(SURF_GROUP) @binding(SURF_FIRST + 2u) var surf_exposures: texture_2d_array<f32>;
@group(SURF_GROUP) @binding(SURF_FIRST + 3u) var breaker_section: texture_2d<f32>;
@group(SURF_GROUP) @binding(SURF_FIRST + 4u) var bed_pyramid: texture_2d<f32>;

const SURF_TAU: f32 = 6.283185307179586;

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

fn shore_sample(cell: Cell, point: vec2<f32>, body: u32, stage: f32, direction: u32, wavelength: f32) -> ShoreSample {
    let record = surf.bodies[body];
    if record.count == 0u || !cell.inside || wavelength < 20.0 || wavelength > 640.0 {
        return ShoreSample(0.0, vec2(0.0), 1.0, cell.inside);
    }
    let sector = direction % SURF_SECTORS;
    let band = log2(wavelength / 20.0);
    let stages = surf_stages(record, stage);
    let base = fitted_lag(cell, stages, sector, band);
    if !base.valid { return ShoreSample(0.0, vec2(0.0), 1.0, false); }
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
    let exposure = shore_exposure(cell, stages.low, sector);
    if stages.weight == 0.0 { return ShoreSample(base.value, gradient, exposure, true); }
    return ShoreSample(base.value, gradient, mix(exposure, shore_exposure(cell, stages.high, sector), stages.weight), true);
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

fn section_row(row: i32, phase: f32) -> vec2<f32> {
    let size = i32(textureDimensions(breaker_section).x);
    let sample = fract(phase / SURF_TAU) * f32(size);
    let i = i32(floor(sample)) % size;
    let a = textureLoad(breaker_section, vec2(i, row), 0).xy;
    let b = textureLoad(breaker_section, vec2((i + 1) % size, row), 0).xy;
    return mix(a, b, fract(sample));
}

fn surf_section(phase: f32, index: f32) -> vec2<f32> {
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

struct SurfPoint {
    displacement: vec3<f32>,
    index: f32,
    phase: f32,
    loss: f32,
}

fn surf_forward(
    point: vec2<f32>, body: u32, row: MapRow, stage: f32, anchor: vec2<f32>,
    dt: f32, chop: f32, heights: vec4<f32>, count: u32,
) -> SurfPoint {
    let cell = map_cell(point);
    let depth = select(1.0e30, row.water.x - cell.bed, cell.inside);
    let floor_depth = max(depth, surf.controls.x);
    let swash = min(map.swash.x * length(heights * row.gains), map.origin_spacing_margin.w);
    let draw = (0.6 / (2.0 * sqrt(2.0) - 0.6)) * swash;
    var fade = f32(depth > 0.0);
    if swash > 0.0 { fade = smoothstep(draw, 0.5 * swash, depth); }
    var field = vec2(0.0);
    var re_vector = vec2(0.0);
    var im_vector = vec2(0.0);
    var horizontal = vec2(0.0);
    var variance = 0.0;
    for (var i = 0u; i < count; i++) {
        let wave = surf_component(i);
        let k = wave.vector;
        let k0 = 1.0 / wave.inverse_k;
        let wavelength = SURF_TAU * wave.inverse_k;
        var gain = 1.0;
        var lag = 0.0;
        var vector = k;
        if fade > 0.0 && wavelength >= 20.0 && wavelength <= 640.0 && cell.inside {
            let shore = shore_sample(cell, point, body, stage, wave.shore_sector, wavelength);
            if shore.valid {
                gain = mix(1.0, surf_shoaling(k0, floor_depth) * sqrt(max(0.0, shore.exposure)), fade);
                lag = k0 * shore.lag * fade;
                vector += k0 * shore.gradient * fade;
            }
        }
        let phase = dot(k, point - anchor) + wave.phase - wave.omega * dt + lag;
        let amplitude = wave.amplitude * row.gains.x * gain;
        let moment = amplitude * vec2(cos(phase), sin(phase));
        field += moment;
        variance += 0.5 * amplitude * amplitude;
        re_vector += moment.x * vector;
        im_vector += moment.y * vector;
        horizontal -= chop * moment.y * vector / max(length(vector), 1.0e-20);
    }
    let amplitude = length(field);
    if amplitude == 0.0 && variance == 0.0 {
        return SurfPoint(vec3(0.0), 0.0, 0.0, 0.0);
    }
    let phase = atan2(field.y, field.x);
    let index = 2.0 * amplitude / (surf.controls.y * floor_depth);
    let scale = min(1.0, 0.6 * max(0.0, depth + swash) / max(4.0 * sqrt(variance), 1.0e-30));
    var height = field.x;
    if fade > 0.0 && index > 0.0 {
        let section = surf_section(phase, index);
        height = mix(height, section.x * amplitude, fade);
        let direction = field.x * re_vector + field.y * im_vector;
        horizontal += section.y * amplitude * fade * direction / max(length(direction), 1.0e-20);
    }
    return SurfPoint(scale * vec3(horizontal.x, height, horizontal.y), index, phase, variance * (1.0 - scale * scale));
}
