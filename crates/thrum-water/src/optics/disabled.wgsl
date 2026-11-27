const OPTICS_ENABLED: bool = false;
struct OpticalPaths {
    reflected: vec3<f32>,
    behind: vec3<f32>,
    transmission: vec3<f32>,
    edge_depth: f32,
}
fn water_optical_paths(
    row: SurfaceRow, world: vec3<f32>, pixel: vec2<u32>, n: vec3<f32>, v: vec3<f32>,
    variance: f32, sky: vec3<f32>, fresnel: f32, hollow_surface: bool,
) -> OpticalPaths {
    return OpticalPaths(sky, vec3(0.0), vec3(0.0), -1.0);
}
