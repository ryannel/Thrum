// Two neighbouring sectors occupy exactly three packed lag layers. Process one
// such bank at a time, so the cell table need not be live for all eight sectors.
struct CacheLag {
    value: vec4<f32>,
    dx: vec4<f32>,
    dz: vec4<f32>,
    valid: vec4<u32>,
}

const CACHE_LAG_VALID = 1u;
const CACHE_LAG_DX = 2u;
const CACHE_LAG_DZ = 4u;

fn weighted_corners(cell: Cell, a: vec4<f32>, b: vec4<f32>, c: vec4<f32>, d: vec4<f32>) -> vec4<f32> {
    return a * cell.weights.x + b * cell.weights.y + c * cell.weights.z + d * cell.weights.w;
}

struct CacheLagCorners {
    a: vec4<f32>,
    b: vec4<f32>,
    c: vec4<f32>,
    d: vec4<f32>,
    invalid_a: vec4<bool>,
    invalid_b: vec4<bool>,
    invalid_c: vec4<bool>,
    invalid_d: vec4<bool>,
}

fn cache_lag_corners(cell: Cell, layer: i32, footprint: u32) -> CacheLagCorners {
    cache_observe(footprint, 4u);
    let a = textureLoad(surf_lags, cell.corners[0], layer, 0);
    let b = textureLoad(surf_lags, cell.corners[1], layer, 0);
    let c = textureLoad(surf_lags, cell.corners[2], layer, 0);
    let d = textureLoad(surf_lags, cell.corners[3], layer, 0);
    let bad_a = abs(a) > vec4(3.0e38);
    let bad_b = abs(b) > vec4(3.0e38);
    let bad_c = abs(c) > vec4(3.0e38);
    let bad_d = abs(d) > vec4(3.0e38);
    return CacheLagCorners(select(a, vec4(0.0), bad_a), select(b, vec4(0.0), bad_b),
        select(c, vec4(0.0), bad_c), select(d, vec4(0.0), bad_d), bad_a, bad_b, bad_c, bad_d);
}

struct CacheLagValue {
    value: vec4<f32>,
    valid: vec4<u32>,
}

fn cache_lag_value(cell: Cell, corners: CacheLagCorners) -> CacheLagValue {
    let invalid = (select(vec4(0u), vec4(1u), corners.invalid_a) * u32(cell.weights.x > 0.0))
        | (select(vec4(0u), vec4(1u), corners.invalid_b) * u32(cell.weights.y > 0.0))
        | (select(vec4(0u), vec4(1u), corners.invalid_c) * u32(cell.weights.z > 0.0))
        | (select(vec4(0u), vec4(1u), corners.invalid_d) * u32(cell.weights.w > 0.0));
    return CacheLagValue(weighted_corners(cell, corners.a, corners.b, corners.c, corners.d),
        select(vec4(0u), vec4(CACHE_LAG_VALID), invalid == vec4(0u)));
}

fn cache_lag_probe(probe: Cell, cell: Cell, corners: CacheLagCorners, layer: i32, footprint: u32) -> CacheLagValue {
    if !probe.inside { return CacheLagValue(vec4(0.0), vec4(CACHE_LAG_VALID)); }
    if all(probe.corners[0] == cell.corners[0]) {
        return cache_lag_value(probe, corners);
    }
    return cache_lag_value(probe, cache_lag_corners(probe, layer, footprint));
}

struct CacheFootprint {
    cell: Cell,
    point: vec2<f32>,
    x_edge: bool,
    z_edge: bool,
}

fn cache_lag_stage(at: CacheFootprint, layer: i32, footprint: u32) -> CacheLag {
    let cell = at.cell;
    let corners = cache_lag_corners(cell, layer, footprint);
    let sample = cache_lag_value(cell, corners);
    let complete = !(corners.invalid_a | corners.invalid_b | corners.invalid_c | corners.invalid_d);
    let spacing = map.origin_spacing_margin.z;
    let step = spacing * 0.01;
    let edge = select(footprint, CACHE_NEIGHBOUR, footprint == CACHE_REST);
    var valid = sample.valid;
    var dx: vec4<f32>;
    if at.x_edge {
        let lo = cache_lag_probe(map_cell(at.point - vec2(step, 0.0)), cell, corners, layer, edge);
        let hi = cache_lag_probe(map_cell(at.point + vec2(step, 0.0)), cell, corners, layer, edge);
        dx = (hi.value - lo.value) / (2.0 * step);
        valid |= (lo.valid & hi.valid) * CACHE_LAG_DX;
    } else {
        dx = select(vec4(0.0), mix(corners.b - corners.a, corners.d - corners.c, cell.uv.y) / spacing, complete);
        valid |= vec4(CACHE_LAG_DX);
    }
    var dz: vec4<f32>;
    if at.z_edge {
        let lo = cache_lag_probe(map_cell(at.point - vec2(0.0, step)), cell, corners, layer, edge);
        let hi = cache_lag_probe(map_cell(at.point + vec2(0.0, step)), cell, corners, layer, edge);
        dz = (hi.value - lo.value) / (2.0 * step);
        valid |= (lo.valid & hi.valid) * CACHE_LAG_DZ;
    } else {
        dz = select(vec4(0.0), mix(corners.c - corners.a, corners.d - corners.b, cell.uv.x) / spacing, complete);
        valid |= vec4(CACHE_LAG_DZ);
    }
    return CacheLag(sample.value, dx, dz, valid);
}

fn cache_lag_layer(at: CacheFootprint, stages: StageBlend, plane: u32, footprint: u32) -> CacheLag {
    if stages.weight == 1.0 {
        return cache_lag_stage(at, i32(stages.high * SURF_LAG_LAYERS + plane), footprint);
    }
    let lo = cache_lag_stage(at, i32(stages.low * SURF_LAG_LAYERS + plane), footprint);
    if stages.weight == 0.0 { return lo; }
    let hi = cache_lag_stage(at, i32(stages.high * SURF_LAG_LAYERS + plane), footprint);
    return CacheLag(mix(lo.value, hi.value, stages.weight), mix(lo.dx, hi.dx, stages.weight),
        mix(lo.dz, hi.dz, stages.weight), lo.valid & hi.valid);
}

fn cache_exposure_stage(cell: Cell, layer: i32, footprint: u32) -> vec4<f32> {
    cache_observe(footprint, 4u);
    return weighted_corners(cell, textureLoad(surf_exposures, cell.corners[0], layer, 0),
        textureLoad(surf_exposures, cell.corners[1], layer, 0),
        textureLoad(surf_exposures, cell.corners[2], layer, 0),
        textureLoad(surf_exposures, cell.corners[3], layer, 0));
}

fn cache_exposure(cell: Cell, stages: StageBlend, bank: u32, footprint: u32) -> vec4<f32> {
    if stages.weight == 1.0 {
        return cache_exposure_stage(cell, i32(stages.high * SURF_EXPOSURE_LAYERS + bank / 2u), footprint);
    }
    let lo = cache_exposure_stage(cell, i32(stages.low * SURF_EXPOSURE_LAYERS + bank / 2u), footprint);
    if stages.weight == 0.0 { return lo; }
    let hi = cache_exposure_stage(cell, i32(stages.high * SURF_EXPOSURE_LAYERS + bank / 2u), footprint);
    return mix(lo, hi, stages.weight);
}

struct FittedCacheLag {
    value: vec3<f32>,
    valid: u32,
}

fn cache_lag_rung(lo: CacheLag, mid: CacheLag, hi: CacheLag, rung: u32) -> FittedCacheLag {
    var layer = lo;
    if rung >= 4u { layer = mid; }
    if rung >= 8u { layer = hi; }
    let channel = rung % 4u;
    return FittedCacheLag(vec3(layer.value[channel], layer.dx[channel], layer.dz[channel]), layer.valid[channel]);
}

fn cache_lag_fit(lo: CacheLag, mid: CacheLag, hi: CacheLag, sector: u32, band: f32) -> FittedCacheLag {
    let rung = min(u32(floor(band)), SURF_RUNGS - 2u);
    let first = sector % 2u * SURF_RUNGS + rung;
    let u = band - f32(rung);
    if u == 1.0 { return cache_lag_rung(lo, mid, hi, first + 1u); }
    let a = cache_lag_rung(lo, mid, hi, first);
    if u == 0.0 { return a; }
    let b = cache_lag_rung(lo, mid, hi, first + 1u);
    return FittedCacheLag(mix(a.value, b.value, u), a.valid & b.valid);
}

struct CacheMoments {
    now: SurfMoments,
    before: SurfMoments,
}

fn accumulate_cache_moment(result: ptr<function, SurfMoments>, amplitude: f32, phase: f32, omega: f32,
    vector: vec2<f32>, chop: f32, local: bool) {
    let moment = amplitude * vec2(cos(phase), sin(phase));
    (*result).field += moment;
    (*result).rate += vec2(moment.y, -moment.x) * omega;
    (*result).variance += 0.5 * amplitude * amplitude;
    if local {
        (*result).re_vector += moment.x * vector;
        (*result).im_vector += moment.y * vector;
        (*result).horizontal -= chop * moment.y * vector / max(length(vector), 1.0e-20);
    }
}

fn cache_moments(column: SurfColumn, point: vec2<f32>, row: MapRow, dt: f32, current: bool, both: bool, local: bool, footprint: u32) -> CacheMoments {
    var position = point;
    var cell = column.cell;
    if !local {
        position += column.contour.xy / column.contour.z;
        cell = map_cell(position);
    }
    let chop = cache_settings.chop_dt.x;
    let relative = position - cache_settings.anchor;
    let edges = (cell.uv < vec2(0.01)) | (cell.uv > vec2(0.99));
    let at = CacheFootprint(cell, position, local && edges.x, local && edges.y);
    let transformed = cell.inside && column.transformed;
    var result: CacheMoments;
    var first = 0u;
    var exposure = vec4(1.0);
    var exposure_group = 2u;
    for (var bank = 0u; bank < 5u; bank++) {
        var end = cache_settings.components;
        if bank < 4u { end = cache_settings.component_ends[bank]; }
        if end == first { continue; }
        var lo: CacheLag;
        var mid: CacheLag;
        var hi: CacheLag;
        if transformed && bank < 4u {
            // Both banks at this exposure layer share its four corner reads.
            if bank / 2u != exposure_group {
                exposure_group = bank / 2u;
                exposure = cache_exposure(cell, column.stages, bank, footprint);
            }
            lo = cache_lag_layer(at, column.stages, bank * 3u, footprint);
            mid = cache_lag_layer(at, column.stages, bank * 3u + 1u, footprint);
            hi = cache_lag_layer(at, column.stages, bank * 3u + 2u, footprint);
        }
        for (var i = first; i < end; i++) {
            let wave = surf_component(i);
            let k = 1.0 / wave.inverse_k;
            var amplitude = wave.amplitude * row.gains.x;
            var lag = 0.0;
            var vector = wave.vector;
            if transformed && bank < 4u {
                let fitted = cache_lag_fit(lo, mid, hi, wave.shore_sector, log2(SURF_TAU * wave.inverse_k / 20.0));
                amplitude *= sqrt(max(0.0, exposure[wave.shore_sector % 4u]));
                if (fitted.valid & CACHE_LAG_VALID) != 0u {
                    lag = k * fitted.value.x;
                    if local {
                        amplitude *= mix(1.0, wave_shoaling(k, column.floor_depth), column.fade);
                        let gradient = select(vec2(0.0), fitted.value.yz,
                            (vec2(fitted.valid) & vec2(CACHE_LAG_DX, CACHE_LAG_DZ)) != vec2(0u));
                        vector += k * gradient;
                    }
                }
            } else if cell.inside && bank < 4u && local {
                amplitude *= mix(1.0, wave_shoaling(k, column.floor_depth), column.fade);
            }
            let phase = dot(wave.vector, relative) + wave.phase;
            let current_phase = select(phase + wave.omega * dt, phase, current);
            accumulate_cache_moment(&result.now, amplitude, current_phase + lag, wave.omega, vector, chop, local);
            if both && dt != 0.0 {
                accumulate_cache_moment(&result.before, amplitude, phase + wave.omega * dt + lag,
                    wave.omega, vector, chop, local);
            }
        }
        first = end;
    }
    if both && dt == 0.0 { result.before = result.now; }
    return result;
}
