struct FloatingVertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec3<f32>,
    @location(3) model0: vec4<f32>,
    @location(4) model1: vec4<f32>,
    @location(5) model2: vec4<f32>,
    @location(6) model3: vec4<f32>,
    @location(7) previous0: vec4<f32>,
    @location(8) previous1: vec4<f32>,
    @location(9) previous2: vec4<f32>,
    @location(10) previous3: vec4<f32>,
    @location(11) valid: u32,
}

struct FloatingFragment {
    @builtin(position) position: vec4<f32>,
    @location(0) current_clip: vec4<f32>,
    @location(1) world: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) clip_delta: vec4<f32>,
    @location(4) @interpolate(flat) valid: u32,
    @location(5) colour: vec3<f32>,
}

@vertex fn floating_vertex(input: FloatingVertex) -> FloatingFragment {
    let model = mat4x4(input.model0, input.model1, input.model2, input.model3);
    let previous = mat4x4(input.previous0, input.previous1, input.previous2, input.previous3);
    let local = vec4(input.position, 1.0);
    let world = model * local;
    var out: FloatingFragment;
    out.position = view.view_proj * world;
    out.current_clip = view.unjittered_view_proj * world;
    out.world = world.xyz;
    out.normal = normalize((model * vec4(input.normal, 0.0)).xyz);
    out.clip_delta = view.previous_view_proj * (previous * local) - out.current_clip;
    out.valid = input.valid;
    out.colour = input.colour;
    return out;
}

@fragment fn floating_fragment(input: FloatingFragment) -> OpaqueFragmentOutput {
    let p = input.world;
    let n = normalize(input.normal);
    let l = light.direction_to_light.xyz;
    let v = normalize(view.camera_xyz_time.xyz - p);
    let sun = sun_irradiance(p) * floating_sun_factor(p, n);
    let direct = direct_brdf(input.colour, 0.0, 0.44, n, l, v, sun, normalize(v + l), 0.0);
    let indirect = indirect_radiance(input.colour, 0.0, 0.44, n, v, 1.0, p, input.position);
    let colour = aerial_colour(direct + indirect, p, input.position) * scene_storage_scale();
    var motion: VertexOutput;
    motion.current_clip = input.current_clip;
    motion.clip_delta = input.clip_delta;
    motion.history_valid = input.valid;
    return OpaqueFragmentOutput(vec4(colour, 1.0), motion_vector(motion), select(1.0, 0.0, input.valid != 0u), 0xffffffffu);
}
