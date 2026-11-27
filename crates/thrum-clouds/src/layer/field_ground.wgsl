// The ground fields share the ground band's world-space rectangle.
struct DiagnosticBandMapping {
    region: vec4<f32>,
    top_m: f32,
    border_m: f32,
    padding: vec2<f32>,
}
@group(3) @binding(8) var diagnostic_band: texture_3d<f32>;
@group(3) @binding(9) var<uniform> diagnostic_window: DiagnosticBandMapping;

fn ground_field_value(field: u32, uv: vec2<f32>) -> f32 {
    if field == 10u {
        let size = textureDimensions(diagnostic_band);
        let pixel = min(vec2<u32>(uv * vec2<f32>(size.xy)), size.xy - 1u);
        let tau = textureLoad(diagnostic_band, vec3<i32>(vec2<i32>(pixel), 0), 0).r;
        return 1.0 - exp(-tau);
    }
    let region = diagnostic_window.region;
    let world = mix(region.xy, region.zw, uv);
    let surface = ground_surface(world);
    if field == 9u {
        let limits = ground_frame.ground_heights;
        return (surface.y - limits.x) / max(1.0, limits.z - limits.x);
    }
    var tau = 0.0;
    for (var index = 0u; index < arrayLength(&ground_rows); index++) {
        tau += ground_row(vec3(world.x, surface.x, world.y), surface, ground_rows[index], true).y;
    }
    return 1.0 - exp(-tau);
}
