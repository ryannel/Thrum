const CACHE_MESH_NODE = 0u;
const CACHE_LATTICE_NODE = 1u;
const CACHE_REST = 2u;
const CACHE_CONTOUR = 3u;
const CACHE_NEIGHBOUR = 4u;
const CACHE_RING_EDGE = 5u;
const CACHE_PREVIOUS_STAGE = 6u;
const CACHE_LATTICE = 7u;

struct CacheSettings {
    anchor: vec2<f32>,
    count: u32,
    components: u32,
    heights: vec4<f32>,
    chop_dt: vec2<f32>,
    shore_size: vec2<u32>,
    camera_planes: array<vec4<f32>, 4>,
    vertical_bounds: vec4<f32>,
    component_ends: vec4<u32>,
}
struct CacheSample {
    current: vec4<f32>,
    previous: vec4<f32>,
    short_weights: vec2<f32>,
    unresolved_throw: vec2<u32>,
}
@group(0) @binding(0) var<uniform> cache_settings: CacheSettings;
@group(0) @binding(1) var<storage, read> mesh: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> components: array<Component>;
@group(0) @binding(3) var<storage, read_write> cache: array<CacheSample>;

fn surf_component(i: u32) -> Component { return components[i]; }

fn cache_column(point: vec2<f32>, body: u32, previous: bool) -> SurfColumn {
    let row = cache_row(point, body, previous);
    let record = map.bodies[body];
    let stage = select(record.level.y, record.level.z - record.level.x, previous);
    return surf_column(point, body, row, stage, cache_settings.heights, cache_settings.components);
}

fn cache_row(point: vec2<f32>, body: u32, previous: bool) -> MapRow {
    let record = map.bodies[body];
    let stage = select(record.level.y, record.level.z - record.level.x, previous);
    var row = MapRow(vec3(record.level.x + stage, 0.0, 0.0), record.gains);
    if record.layers.x == 2u {
        let answer = map_sample(point);
        if answer.body == body { row = answer.row; }
        row.water.x += stage - record.level.y;
    }
    return row;
}

struct CachePair {
    now: SurfPoint,
    before: SurfPoint,
}

fn cache_evaluate(column: SurfColumn, point: vec2<f32>, row: MapRow,
    dt: f32, current: bool, both: bool, footprint: u32) -> CachePair {
    if !column.valid { return CachePair(column.empty, column.empty); }
    // A separated previous stage still uses the same sum, at its own time.
    let ordinary = cache_moments(column, point, row, dt, current, both, true, footprint);
    var swash: CacheMoments;
    if column.inner > 0.0 {
        swash = cache_moments(column, point, row, dt, current, both, false, select(footprint, CACHE_CONTOUR, footprint == CACHE_REST));
    }
    let chop = cache_settings.chop_dt.x;
    var result: CachePair;
    result.now = surf_column_result(column, point, row, chop, ordinary.now, swash.now);
    if both {
        result.before = surf_column_result(column, point, row, chop, ordinary.before, swash.before);
    }
    return result;
}

fn cache_sample(point: vec2<f32>, body: u32, previous: bool, footprint: u32) -> CachePair {
    let record = map.bodies[body];
    let dt = cache_settings.chop_dt.y;
    let same_stage = record.level.x + record.level.y == record.level.z;
    let row = cache_row(point, body, false);
    var pair = cache_evaluate(cache_column(point, body, false), point, row, dt, true, previous && same_stage, footprint);
    if previous && !same_stage {
        let old_row = cache_row(point, body, true);
        pair.before = cache_evaluate(cache_column(point, body, true), point, old_row, dt, false, false, CACHE_PREVIOUS_STAGE).now;
    }
    return pair;
}

fn unresolved_throw(sample: SurfPoint, spacing: f32) -> vec2<f32> {
    // When one cell spans much of the pitching phase, its triangle can
    // bridge the fold as a blade. Retain the steep height profile and its
    // whitewater while fading the horizontal throw that the mesh cannot resolve.
    return sample.section_throw * smoothstep(0.25, 0.65, spacing * sample.phase_step);
}

fn visible_shore_face(point: vec2<f32>, halo: vec2<f32>) -> bool {
    let bounds = cache_settings.vertical_bounds;
    if bounds.z == 0.0 { return true; }
    // The face is read only by visible water fragments. A bilinear
    // footprint can reach one node beyond the fragment's world column.
    let lo = vec3(point.x - halo.x, bounds.x, point.y - halo.y);
    let hi = vec3(point.x + halo.x, bounds.y, point.y + halo.y);
    for (var i = 0u; i < 4u; i++) {
        let plane = cache_settings.camera_planes[i];
        let extreme = max(lo * plane.xyz, hi * plane.xyz);
        let scale = dot(max(abs(lo), abs(hi)), abs(plane.xyz)) + abs(plane.w) + 1.0;
        if dot(extreme, vec3(1.0)) + plane.w < -0.000002 * scale { return false; }
    }
    return true;
}

fn cache_shore_face(i: u32) {
    let size = cache_settings.shore_size;
    let count = size.x * size.y;
    let node = i - cache_settings.count;
    if node > count || count == 0u { return; }
    var result: CacheSample;
    if node == count {
        result.unresolved_throw = size;
    } else {
        let uv = vec2<f32>(vec2(node % size.x, node / size.x)) / vec2<f32>(size - 1u);
        let span = vec2<f32>(map.size_count_sea.xy - 1u) * map.origin_spacing_margin.z;
        let point = map.origin_spacing_margin.xy + uv * span;
        let distance = abs(point - cache_settings.anchor);
        let halo = span / vec2<f32>(size - 1u);
        if max(distance.x, distance.y) > 76.0 && visible_shore_face(point, halo) {
            let answer = map_sample(point);
            if answer.body != NO_BODY && answer.has_bed {
                cache_observe(CACHE_LATTICE_NODE, 1u);
                let sample = cache_sample(point, answer.body, false, CACHE_LATTICE).now;
                let strength = breaking_strength(sample);
                if strength > 0.0 {
                    let displaced_bed = map_cell(point + sample.displacement.xz);
                    if displaced_bed.inside && answer.row.water.x + sample.displacement.y > displaced_bed.bed {
                        result.current = strength * vec4(cos(sample.phase), sin(sample.phase), 1.0, 0.0);
                    }
                }
            }
        }
    }
    cache[i] = result;
}

@compute @workgroup_size(64)
fn fill_surf_cache(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= cache_settings.count {
        cache_shore_face(i);
        return;
    }
    let vertex = mesh[i];
    let sea = vertex.w == 0.0;
    let body = select(u32(vertex.w) - 1u, map.size_count_sea.w, sea);
    let anchor = select(vec2(0.0), floor(cache_settings.anchor), sea);
    let point = anchor + vertex.xy;
    var result: CacheSample;
    let local = vertex.xy / vertex.z;
    let spacing = select(0.0, vertex.z, sea);
    let inner = sea_inner_radius(vertex.z);
    if sea && max(abs(local.x), abs(local.y)) < inner - 1.0 {
        cache[i] = result;
        return;
    }
    // A drawn vertex is within one grid step of this normal tap. Every
    // vertex of its triangles is within one more. Test their enclosing box
    // with the sheet's complete displacement bound before touching the bed.
    let halo = 2.0 * vertex.z + cache_settings.vertical_bounds.w;
    if visible_shore_face(point, vec2(halo)) && map_cell(point).inside {
        cache_observe(CACHE_MESH_NODE, 1u);
        var pair: CachePair;
        var face: f32;
        var throw_now: vec2<f32>;
        var throw_before: vec2<f32>;
        if sea && max(abs(local.x), abs(local.y)) >= f32(sea_grid_radius(vertex.z)) {
            let coarse = local * 0.5;
            let coarse_spacing = 2.0 * vertex.z;
            let a = anchor + floor(coarse) * coarse_spacing;
            let fraction = fract(coarse);
            let blend = max(fraction.x, fraction.y);
            pair = cache_sample(a, body, true, CACHE_RING_EDGE);
            face = breaking_face(pair.now);
            throw_now = unresolved_throw(pair.now, coarse_spacing);
            throw_before = unresolved_throw(pair.before, coarse_spacing);
            if blend != 0.0 {
                let b = anchor + ceil(coarse) * coarse_spacing;
                let after = cache_sample(b, body, true, CACHE_RING_EDGE);
                pair.now.displacement = mix(pair.now.displacement, after.now.displacement, blend);
                pair.now.variance = mix(pair.now.variance, after.now.variance, blend);
                face = mix(face, breaking_face(after.now), blend);
                throw_now = mix(throw_now, unresolved_throw(after.now, coarse_spacing), blend);
                pair.now.short_weight = mix(pair.now.short_weight, after.now.short_weight, blend);
                pair.before.displacement = mix(pair.before.displacement, after.before.displacement, blend);
                pair.before.short_weight = mix(pair.before.short_weight, after.before.short_weight, blend);
                throw_before = mix(throw_before, unresolved_throw(after.before, coarse_spacing), blend);
            }
        } else {
            pair = cache_sample(point, body, true, CACHE_REST);
            face = breaking_face(pair.now);
            throw_now = unresolved_throw(pair.now, spacing);
            throw_before = unresolved_throw(pair.before, spacing);
        }
        result = CacheSample(vec4(pair.now.displacement, 1.0 + face), vec4(pair.before.displacement, pair.now.variance),
            vec2(pair.now.short_weight, pair.before.short_weight),
            vec2(pack2x16float(throw_now), pack2x16float(throw_before)));
    }
    cache[i] = result;
}
