// Prefixes of the scene camera and its auxiliary record. Only the current
// transform, world eye and viewport are used by this static surface.
struct PoolView {
    view_proj: mat4x4<f32>,
    previous_view_proj: mat4x4<f32>,
    camera_xyz_time: vec4<f32>,
}
struct PoolViewAux {
    inverse_view_proj: mat4x4<f32>,
    viewport_inv_viewport: vec4<f32>,
}
struct PoolPlane { bounds: vec4<f32>, level: vec4<f32> }
@group(0) @binding(0) var<uniform> pool_view: PoolView;
@group(0) @binding(1) var<uniform> pool_aux: PoolViewAux;
@group(3) @binding(0) var<uniform> pool: PoolPlane;
@group(3) @binding(1) var<uniform> reflection: SecondaryViewRecord;
@group(3) @binding(2) var reflection_colour: texture_2d<f32>;
@group(3) @binding(3) var reflection_sampler: sampler;

struct PoolVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
}
@vertex fn pool_vertex(@builtin(vertex_index) vertex: u32) -> PoolVertex {
    let corner = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 0.0),
        vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 1.0))[vertex];
    let xz = mix(pool.bounds.xy, pool.bounds.zw, corner);
    let world = vec3(xz.x, pool.level.x, xz.y);
    return PoolVertex(pool_view.view_proj * vec4(world, 1.0), world);
}
struct PoolOutput {
    @location(0) colour: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @location(2) reactive: f32,
}
@fragment fn pool_fragment(input: PoolVertex) -> PoolOutput {
    let clip = reflection.view_proj * vec4(input.world, 1.0);
    let uv = clip.xy / clip.w * vec2(0.5, -0.5) + vec2(0.5);
    let texel = clamp(uv * reflection.extent.xy, vec2(0.5), reflection.extent.xy - vec2(0.5));
    let sample = textureSampleLevel(reflection_colour, reflection_sampler, texel / reflection.extent.zw, 0.0);
    let ray = reflect(normalize(input.world - pool_view.camera_xyz_time.xyz), vec3(0.0, 1.0, 0.0));
    let scale = environment_storage_scale();
    // Coverage premultiplies the geometry. Fill only the uncovered part with sky.
    let reflected = sample.rgb + (1.0 - sample.a) * environment_sky_radiance(ray) * scale;
    let air = environment_aerial(input.world, input.position.xy * pool_aux.viewport_inv_viewport.zw);
    let colour = air.scatter * scale + air.transmission * reflected;
    return PoolOutput(vec4(min(colour, vec3(65504.0)), 1.0), vec2(65504.0), 1.0);
}
