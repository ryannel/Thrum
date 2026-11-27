struct PlungeSource { place_axis: vec4<f32>, footprint_rate: vec4<f32> }
struct PlungeSources { count: vec4<u32>, sources: array<PlungeSource> }
@group(0) @binding(PLUNGE_BINDING) var<storage, read> plunge_sources: PlungeSources;

fn plunge_births(point: vec2<f32>, spacing: f32) -> vec4<f32> {
    var rates = vec2(0.0);
    for (var i = 0u; i < plunge_sources.count.x; i++) {
        let source = plunge_sources.sources[i];
        let delta = point - source.place_axis.xy;
        if dot(delta, delta) > pow(2.0 * spacing + max(source.footprint_rate.x, source.footprint_rate.y), 2.0) {
            continue;
        }
        let across = source.place_axis.zw;
        let offset = vec2(dot(delta, across), dot(delta, vec2(-across.y, across.x)));
        let extent = max(source.footprint_rate.xy, vec2(spacing)) * 0.5;
        let standard = offset / extent;
        let spread = exp(-1.15 * dot(standard, standard));
        let dilution = source.footprint_rate.x * source.footprint_rate.y / (4.0 * extent.x * extent.y);
        rates += spread * dilution * source.footprint_rate.zw;
    }
    return vec4(rates, 0.0, 0.0);
}
