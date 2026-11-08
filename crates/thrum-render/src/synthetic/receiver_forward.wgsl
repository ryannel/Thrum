@group(0) @binding(6) var receiver_depth: texture_depth_2d;
@group(0) @binding(7) var receiver_resolved: texture_2d<f32>;

// D32 stores can round one representable step away from the forward fragment Z.
// Both passes still use the same opaque draws, coverage and strict depth order.
fn receiver_depth_matches(stored: f32, fragment: f32) -> bool {
    if !(stored > 0.0 && stored <= 1.0 && fragment > 0.0 && fragment <= 1.0) {
        return false;
    }
    let a = bitcast<u32>(stored);
    let b = bitcast<u32>(fragment);
    return max(a, b) - min(a, b) <= 1u;
}

fn last_cascade_visibility(input: VertexOutput, receiver: ShadowReceiver) -> f32 {
    let at = vec2<i32>(input.position.xy);
    let z = textureLoad(receiver_depth, at, 0);
    if receiver_depth_matches(z, input.position.z) {
        let mask = textureLoad(receiver_resolved, at, 0).x;
        if mask >= 0.0 { return mask; }
    }
    return cascade_visibility(1u, receiver);
}
