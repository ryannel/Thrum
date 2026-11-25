@group(1) @binding(24) var<uniform> ambient_frame: AmbientFrame;
@group(1) @binding(25) var ambient_history: texture_2d<f32>;

fn ambient_contact(input: VertexOutput) -> f32 {
    if ambient_frame.valid == 0u || input.history_valid == 0u { return 1.0; }
    let previous = input.current_clip + input.clip_delta;
    if previous.w <= 0.0 { return 1.0; }
    let screen = input.position.xy + motion_vector(input) + ambient_frame.jitter_delta;
    let stride = f32(ambient_frame.stride);
    let offset = f32(ambient_frame.stride / 2u) + 0.5;
    let position = (screen - offset) / stride;
    let first = vec2<i32>(floor(position));
    let alpha = fract(position);
    let z = previous.z / previous.w;
    let extent = ambient_extent(ambient_frame);
    var sum = 0.0;
    var weight_sum = 0.0;
    for (var y = 0; y <= 1; y++) {
        for (var x = 0; x <= 1; x++) {
            let at = first + vec2(x, y);
            if !ambient_in_bounds(at, extent) { continue; }
            let sample = textureLoad(ambient_history, at, 0).xy;
            let spatial = mix(1.0 - alpha, alpha, vec2<f32>(f32(x), f32(y)));
            let weight = spatial.x * spatial.y * ambient_depth_weight(sample.y, z);
            sum += weight * sample.x;
            weight_sum += weight;
        }
    }
    return select(1.0, clamp(sum / weight_sum, 0.0, 1.0), weight_sum > 1e-4);
}
