// Replay the final opaque depth without shading. Explicit coverage preserves
// the forward raster's Metal coverage and interpolation behaviour.
struct AmbientReceiverOutput {
    @location(0) position: vec4<f32>,
    @builtin(sample_mask) coverage: u32,
}

@fragment fn ambient_receiver(input: VertexOutput) -> AmbientReceiverOutput {
    return AmbientReceiverOutput(vec4(input.world_position, 1.0), 0xffffffffu);
}
