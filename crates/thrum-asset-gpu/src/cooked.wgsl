// Cooked-asset entries appended to the prepared scene's own forward shader. Meshes use the scene's
// fragment entry, so they are lit exactly as scene geometry; no lighting is defined here.

struct CookedVertex {
    @location(0) position: vec3<f32>,
    @location(1) normal_oct: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) model0: vec4<f32>,
    @location(4) model1: vec4<f32>,
    @location(5) model2: vec4<f32>,
    @location(6) model3: vec4<f32>,
    // 1 once this placement's product was drawn in the previous submitted frame and the view
    // history continues; 0 for a new or replaced product, a device change or a camera cut.
    @location(7) history: u32,
}

fn cooked_octahedral(e: vec2<f32>) -> vec3<f32> {
    var n = vec3(e.x, e.y, 1.0 - abs(e.x) - abs(e.y));
    let t = max(-n.z, 0.0);
    n.x += select(t, -t, n.x >= 0.0);
    n.y += select(t, -t, n.y >= 0.0);
    return normalize(n);
}

@vertex fn cooked_vertex(input: CookedVertex) -> VertexOutput {
    let model = mat4x4(input.model0, input.model1, input.model2, input.model3);
    let world = model * vec4(input.position, 1.0);
    var output: VertexOutput;
    output.position = view.view_proj * world;
    output.current_clip = view.unjittered_view_proj * world;
    output.world_position = world.xyz;
    // Placements are rigid with uniform scale, so the model matrix transforms normals.
    output.normal = normalize((model * vec4(cooked_octahedral(input.normal_oct), 0.0)).xyz);
    output.uv = input.uv;
    output.material = 0u;
    // Placements are static: last frame's clip position comes from last frame's camera, when this
    // geometry existed last frame.
    let valid = input.history != 0u;
    output.history_valid = select(0u, 1u, valid);
    output.clip_delta = select(vec4(0.0),
        (view.previous_view_proj - view.unjittered_view_proj) * world, valid);
    return output;
}

struct CookedVolume {
    model: mat4x4<f32>,
    inverse_model: mat4x4<f32>,
    // x: extinction per unit density per unit object length, y: march steps, z: light steps.
    params: vec4<f32>,
}
@group(2) @binding(8) var cooked_density: texture_3d<f32>;
@group(2) @binding(9) var cooked_density_sampler: sampler;
@group(2) @binding(10) var<uniform> cooked_volume: CookedVolume;

struct CookedVolumeOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
}

@vertex fn cooked_volume_vertex(@builtin(vertex_index) index: u32) -> CookedVolumeOutput {
    // Unit cube [0,1]^3; corner c has x = bit 0, y = bit 1, z = bit 2. Faces wind
    // counter-clockwise seen from outside; the pipeline draws back faces so the march also
    // works with the camera inside the box.
    let corners = array<u32, 36>(1u, 3u, 7u, 1u, 7u, 5u, 0u, 4u, 6u, 0u, 6u, 2u,
                                 2u, 6u, 7u, 2u, 7u, 3u, 0u, 1u, 5u, 0u, 5u, 4u,
                                 4u, 5u, 7u, 4u, 7u, 6u, 0u, 2u, 3u, 0u, 3u, 1u);
    let c = corners[index % 36u];
    let p = vec3(f32(c & 1u), f32((c >> 1u) & 1u), f32((c >> 2u) & 1u));
    let world = cooked_volume.model * vec4(p, 1.0);
    var output: CookedVolumeOutput;
    output.position = view.view_proj * world;
    output.world = world.xyz;
    return output;
}

fn cooked_density_at(p: vec3<f32>) -> f32 {
    return textureSampleLevel(cooked_density, cooked_density_sampler, p, 0.0).r;
}

fn cooked_box_exit(origin: vec3<f32>, direction: vec3<f32>) -> vec2<f32> {
    let inverse = 1.0 / direction;
    let a = (vec3(0.0) - origin) * inverse;
    let b = (vec3(1.0) - origin) * inverse;
    let near = min(a, b);
    let far = max(a, b);
    return vec2(max(max(near.x, near.y), max(near.z, 0.0)), min(min(far.x, far.y), far.z));
}

@fragment fn cooked_volume_fragment(input: CookedVolumeOutput) -> FragmentOutput {
    let camera = view.camera_xyz_time.xyz;
    let origin = (cooked_volume.inverse_model * vec4(camera, 1.0)).xyz;
    let direction = normalize((cooked_volume.inverse_model * vec4(input.world - camera, 0.0)).xyz);
    let span = cooked_box_exit(origin, direction);
    let steps = max(u32(cooked_volume.params.y), 1u);
    let light_steps = max(u32(cooked_volume.params.z), 1u);
    let dt = max(span.y - span.x, 0.0) / f32(steps);
    let extinction = cooked_volume.params.x;
    // Sun direction in object space; light-march length is one box diagonal.
    let to_light = normalize((cooked_volume.inverse_model * vec4(light.direction_to_light.xyz, 0.0)).xyz);
    let light_dt = 1.7320508 / f32(light_steps);
    // Sky light approximated as uniform radiance E_up / pi over the upper hemisphere only; the
    // isotropic phase integrates that half sphere to E_up / (2 pi) of in-scattered radiance. The
    // sky term is not shadowed by the volume itself (diagnostic consumer).
    let ambient = ambient_irradiance(vec3(0.0, 1.0, 0.0)) / (2.0 * 3.14159265359);
    let isotropic = 1.0 / (4.0 * 3.14159265359);
    var transmittance = 1.0;
    var radiance = vec3(0.0);
    for (var i = 0u; i < steps; i++) {
        let p = origin + direction * (span.x + (f32(i) + 0.5) * dt);
        let density = cooked_density_at(p);
        if density <= 0.0 { continue; }
        var optical = 0.0;
        for (var j = 0u; j < light_steps; j++) {
            let q = p + to_light * (f32(j) + 0.5) * light_dt;
            if any(q < vec3(0.0)) || any(q > vec3(1.0)) { break; }
            optical += cooked_density_at(q) * extinction * light_dt;
        }
        let sigma = density * extinction;
        // In-scattered radiance per unit extinction: Sun irradiance through the light march times
        // the isotropic phase, plus sky radiance.
        let source = light.irradiance.rgb * exp(-optical) * isotropic + ambient;
        let step_transmittance = exp(-sigma * dt);
        // Single scattering, albedo one, no multiple scattering: a diagnostic content consumer
        // that reads darker than a real cloud, not a sky or cloud renderer.
        radiance += transmittance * (1.0 - step_transmittance) * source;
        transmittance *= step_transmittance;
        if transmittance < 0.003 { break; }
    }
    let alpha = 1.0 - transmittance;
    // Empty rays contribute neither colour nor correspondence. Occupied rays
    // have no retained volume sample and explicitly invalidate their motion.
    if alpha <= 0.0 { discard; }
    var colour = vec3(0.0);
    if alpha > 0.0001 {
        let entry = (cooked_volume.model * vec4(origin + direction * span.x, 1.0)).xyz;
        colour = aerial_colour(radiance / alpha, entry, input.position) * alpha;
    }
    let exposed = clamp(colour * scene_storage_scale(), vec3(0.0), vec3(65504.0));
    return FragmentOutput(vec4(exposed, alpha), vec2(65504.0));
}
