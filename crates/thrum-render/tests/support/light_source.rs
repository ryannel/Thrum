// Reverse only REVIEW-RENDER M1's WGSL edits before checking SCENE-VIEWS'
// historical source hashes. The fragments are fixed so later edits fail.
pub(super) fn before_light_split(source: &str) -> String {
    const RECORDS: &str = "struct DirectionalLight {
    direction_to_light: vec4<f32>,
    irradiance: vec4<f32>,
    half_vector_fresnel: vec4<f32>,
    ambient_irradiance: vec4<f32>,
}

struct EnvironmentLight {
    horizontal_irradiance: vec4<f32>,
    sky_diffuse: array<vec4<f32>,9>,
}

";
    const LEGACY_RECORD: &str = "struct DirectionalSource {
    direction_to_light: vec4<f32>,
    irradiance: vec4<f32>,
    half_vector_fresnel: vec4<f32>,
    ambient_irradiance: vec4<f32>,
    sky_diffuse: array<vec4<f32>,9>,
}
";
    const VIEW: &str = "@group(0) @binding(0) var<uniform> view: ViewRecord;";
    const AMBIENT_SOURCE: &str = "fn ambient_source(n: vec3<f32>) -> vec3<f32> {
    if environment_light.horizontal_irradiance.w==0.0 { return light.ambient_irradiance.rgb; }";
    const LEGACY_AMBIENT: &str = "fn ambient_irradiance(n: vec3<f32>) -> vec3<f32> {
    if light.ambient_irradiance.w==0.0 { return light.ambient_irradiance.rgb; }";
    const SURFACE_WRAPPER: &str = "
fn ambient_irradiance(n: vec3<f32>, world_position: vec3<f32>, pixel_position: vec4<f32>) -> vec3<f32> {
    return ambient_source(n);
}

";
    let mut historical = source.to_owned();
    for (split, original) in [
        (RECORDS, ""),
        (VIEW, &format!("{LEGACY_RECORD}{VIEW}")),
        (
            "@group(1) @binding(0) var<uniform> light: DirectionalLight;\n@group(1) @binding(18) var<uniform> environment_light: EnvironmentLight;",
            "@group(1) @binding(0) var<uniform> light: DirectionalSource;",
        ),
        (AMBIENT_SOURCE, LEGACY_AMBIENT),
        (
            "evaluate_sky_diffuse(environment_light.sky_diffuse,n)",
            "evaluate_sky_diffuse(light.sky_diffuse,n)",
        ),
        (SURFACE_WRAPPER, "\n"),
        (
            "indirect_radiance(surface_base,metalness,roughness,n,v,orm.r,input.world_position,input.position)",
            "indirect_radiance(surface_base,metalness,roughness,n,v,orm.r)",
        ),
        (
            "ao: f32,\n                     world_position: vec3<f32>, pixel_position: vec4<f32>) -> vec3<f32>",
            "ao: f32) -> vec3<f32>",
        ),
        (
            "ambient_irradiance(n,world_position,pixel_position)",
            "ambient_irradiance(n)",
        ),
    ] {
        assert_eq!(
            historical.matches(split).count(),
            1,
            "split WGSL fragment {split:?}"
        );
        historical = historical.replacen(split, original, 1);
    }
    historical
}
