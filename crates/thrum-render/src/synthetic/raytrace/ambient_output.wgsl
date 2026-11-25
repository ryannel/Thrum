// The depth-tested raster surface owns this receiver. Other passes can
// continue using ordinary depth for their existing history and filters.
struct AmbientFragmentOutput {
    @location(0) colour: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @location(2) reactive: f32,
    @location(3) receiver: vec4<f32>,
    @builtin(sample_mask) coverage: u32,
}

fn ambient_output(input: VertexOutput, colour: vec4<f32>) -> AmbientFragmentOutput {
    let payload = select(vec4(0.0), vec4(input.world_position, 1.0), ambient_frame.stride != 0u);
    return AmbientFragmentOutput(colour, motion_vector(input),
        select(0.0, 1.0, input.history_valid == 0u), payload, 0xffffffffu);
}

@fragment fn ambient_fragment(input: VertexOutput, @builtin(front_facing) front_facing: bool) -> AmbientFragmentOutput {
    let receiver = ShadowReceiver(input.world_position,vec3(0.0),vec3(0.0),input.normal);
    return ambient_output(input, shade(input, receiver, front_facing, false));
}

@fragment fn ambient_shadowed_fragment(input: VertexOutput, @builtin(front_facing) front_facing: bool) -> AmbientFragmentOutput {
    let receiver = ShadowReceiver(input.world_position,dpdx(input.world_position),
        dpdy(input.world_position),normalize(input.normal));
    return ambient_output(input, shade(input, receiver, front_facing, true));
}
