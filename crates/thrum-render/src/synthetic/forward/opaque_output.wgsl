        + material.emissive_cutoff.rgb*emissive.rgb;
    // Keep scene radiance above one. Repair unsupported presentation arithmetic
    // before pre-exposure and bound only to the finite fp16 storage envelope.
    let finite = select(vec3(0.0), colour, colour >= vec3(0.0));
    let bounded = min(finite, vec3(3.402823e38));
    let aerial = aerial_colour(bounded, input.world_position, input.position);
    return vec4(clamp(aerial*scene_storage_scale(),vec3(0.0),vec3(MAX_FINITE_HALF)),1.0);
}
struct FragmentOutput {
    @location(0) colour: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @location(2) reactive: f32,
}
// Explicit coverage prevents Metal triangle merging from borrowing a neighbour
// primitive's interpolants for derivative/helper lanes. All samples stay covered;
// the ordinary depth test still owns visibility. Volumes keep FragmentOutput.
struct OpaqueFragmentOutput {
    @location(0) colour: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @location(2) reactive: f32,
    @builtin(sample_mask) coverage: u32,
}
fn motion_vector(input: VertexOutput) -> vec2<f32> {
    let current = input.current_clip;
    let delta = input.clip_delta;
    let previous_w = current.w + delta.w;
    if input.history_valid == 0u || current.w <= 0.0 || previous_w <= 0.0 {
        return vec2(MAX_FINITE_HALF);
    }
    let difference = (delta.xy * current.w - current.xy * delta.w)
        / (current.w * previous_w);
    return clamp(difference * vec2(0.5, -0.5) * view_aux.viewport_inv_viewport.xy,
        vec2(-MAX_FINITE_HALF), vec2(MAX_FINITE_HALF));
}
@fragment fn fragment(input:VertexOutput, @builtin(front_facing) front_facing: bool)->OpaqueFragmentOutput {
    let receiver = ShadowReceiver(input.world_position,vec3(0.0),vec3(0.0),input.normal);
    return OpaqueFragmentOutput(shade(input,receiver,front_facing,false), motion_vector(input), select(0.0, 1.0, input.history_valid == 0u), 0xffffffffu);
}
@fragment fn shadowed_fragment(input:VertexOutput, @builtin(front_facing) front_facing: bool)->OpaqueFragmentOutput {
    let receiver = ShadowReceiver(input.world_position,dpdx(input.world_position),
        dpdy(input.world_position),normalize(input.normal));
    return OpaqueFragmentOutput(shade(input,receiver,front_facing,true), motion_vector(input), select(0.0, 1.0, input.history_valid == 0u), 0xffffffffu);
}
