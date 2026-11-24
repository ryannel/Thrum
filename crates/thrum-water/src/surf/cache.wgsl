struct CacheSettings {
    anchor: vec2<f32>,
    count: u32,
    components: u32,
    heights: vec4<f32>,
    chop_dt: vec4<f32>,
}
struct CacheSample {
    current: vec4<f32>,
    previous: vec4<f32>,
    short_weights: vec4<f32>,
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

@compute @workgroup_size(64)
fn fill_surf_cache(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= cache_settings.count { return; }
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
        if sea && max(abs(local.x), abs(local.y)) >= f32(sea_grid_radius(vertex.z)) {
            let transition = 2.0;
            let coarse = local / transition;
            let spacing = transition * vertex.z;
            let a = anchor + floor(coarse) * spacing;
            let b = anchor + ceil(coarse) * spacing;
            let fraction = fract(coarse);
            let blend = max(fraction.x, fraction.y);
            let now_a = cache_sample(a, body, 0.0, false);
            let now_b = cache_sample(b, body, 0.0, false);
            now.displacement = mix(now_a.displacement, now_b.displacement, blend);
            now.variance = mix(now_a.variance, now_b.variance, blend);
            face = mix(breaking_face(now_a), breaking_face(now_b), blend);
            now.short_weight = mix(now_a.short_weight, now_b.short_weight, blend);
            let before_a = cache_sample(a, body, -dt, true);
            let before_b = cache_sample(b, body, -dt, true);
            before.displacement = mix(before_a.displacement, before_b.displacement, blend);
            before.short_weight = mix(before_a.short_weight, before_b.short_weight, blend);
        }
        result = CacheSample(vec4(now.displacement, 1.0 + face), vec4(before.displacement, now.variance),
            vec4(now.short_weight, before.short_weight, 0.0, 0.0));
    }
    cache[i] = result;
}
