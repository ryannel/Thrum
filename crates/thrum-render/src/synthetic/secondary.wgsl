@group(0) @binding(6) var<uniform> secondary_view: SecondaryViewRecord;

@fragment fn secondary_fragment(input: VertexOutput,
    @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let receiver = ShadowReceiver(input.world_position, vec3(0.0), vec3(0.0), input.normal);
    if secondary_view.options.x > 0.0 && dot(secondary_view.clip_plane, vec4(input.world_position, 1.0)) < 0.0 {
        discard;
    }
    return vec4(shade(input, receiver, front_facing, false).rgb, 1.0);
}

@fragment fn secondary_shadowed_fragment(input: VertexOutput,
    @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let receiver = ShadowReceiver(input.world_position, dpdx(input.world_position),
        dpdy(input.world_position), normalize(input.normal));
    if secondary_view.options.x > 0.0 && dot(secondary_view.clip_plane, vec4(input.world_position, 1.0)) < 0.0 {
        discard;
    }
    return vec4(shade(input, receiver, front_facing, true).rgb, 1.0);
}
