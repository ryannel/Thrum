@group(0) @binding(6) var receiver_depth: texture_depth_2d;
@group(0) @binding(7) var receiver_resolved: texture_2d<f32>;

fn last_cascade_visibility(input: VertexOutput, receiver: ShadowReceiver) -> f32 {
    let at = vec2<i32>(input.position.xy);
    let z = textureLoad(receiver_depth, at, 0);
    if z > 0.0 && z == input.position.z {
        let mask = textureLoad(receiver_resolved, at, 0).x;
        if mask >= 0.0 { return mask; }
    }
    return cascade_visibility(1u, receiver);
}
