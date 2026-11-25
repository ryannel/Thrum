struct CacheSettings {
    anchor: vec2<f32>,
    count: u32,
    components: u32,
    heights: vec4<f32>,
    chop_dt: vec2<f32>,
    shore_size: vec2<u32>,
    camera_planes: array<vec4<f32>, 4>,
    vertical_bounds: vec4<f32>,
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

fn cache_sample(point: vec2<f32>, body: u32, dt: f32, previous: bool) -> SurfPoint {
    let record = map.bodies[body];
    let stage = select(record.level.y, record.level.z - record.level.x, previous);
    var row = MapRow(vec3(record.level.x + stage, 0.0, 0.0), record.gains);
    if record.layers.x == 2u {
        let answer = map_sample(point);
        if answer.body == body { row = answer.row; }
        row.water.x += stage - record.level.y;
    }
    return surf_forward(point, body, row, stage, cache_settings.anchor, dt,
        cache_settings.chop_dt.x, cache_settings.heights, cache_settings.components);
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
                let sample = cache_sample(point, answer.body, 0.0, false);
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
    // Disjoint deep-water and shore paths preserve the ambient cache well past
    // the bed. On the bed the cache stores the forward sheet at the mesh's nodes.
    if map_cell(point).inside {
        let dt = cache_settings.chop_dt.y;
        let local = vertex.xy / vertex.z;
        var now = cache_sample(point, body, 0.0, false);
        var before = cache_sample(point, body, -dt, true);
        var face = breaking_face(now);
        let spacing = select(0.0, vertex.z, sea);
        var throw_now = unresolved_throw(now, spacing);
        var throw_before = unresolved_throw(before, spacing);
        if sea && max(abs(local.x), abs(local.y)) >= f32(sea_grid_radius(vertex.z)) {
            let transition = 2.0;
            let coarse = local / transition;
            let coarse_spacing = transition * vertex.z;
            let a = anchor + floor(coarse) * coarse_spacing;
            let b = anchor + ceil(coarse) * coarse_spacing;
            let fraction = fract(coarse);
            let blend = max(fraction.x, fraction.y);
            let now_a = cache_sample(a, body, 0.0, false);
            let now_b = cache_sample(b, body, 0.0, false);
            now.displacement = mix(now_a.displacement, now_b.displacement, blend);
            now.variance = mix(now_a.variance, now_b.variance, blend);
            face = mix(breaking_face(now_a), breaking_face(now_b), blend);
            throw_now = mix(unresolved_throw(now_a, coarse_spacing),
                unresolved_throw(now_b, coarse_spacing), blend);
            now.short_weight = mix(now_a.short_weight, now_b.short_weight, blend);
            let before_a = cache_sample(a, body, -dt, true);
            let before_b = cache_sample(b, body, -dt, true);
            before.displacement = mix(before_a.displacement, before_b.displacement, blend);
            before.short_weight = mix(before_a.short_weight, before_b.short_weight, blend);
            throw_before = mix(unresolved_throw(before_a, coarse_spacing),
                unresolved_throw(before_b, coarse_spacing), blend);
        }
        result = CacheSample(vec4(now.displacement, 1.0 + face), vec4(before.displacement, now.variance),
            vec2(now.short_weight, before.short_weight),
            vec2(pack2x16float(throw_now), pack2x16float(throw_before)));
    }
    cache[i] = result;
}
