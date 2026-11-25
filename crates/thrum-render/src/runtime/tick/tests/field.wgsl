// One dual-purpose shader. Each pipeline's layout includes only its entry's resources.
struct FrameParams {
    steps: u32,
    reset: u32,
    pad: vec2<u32>,
}

@group(0) @binding(0) var<uniform> frame: FrameParams;
@group(0) @binding(1) var<storage, read> previous: array<u32, 4>;
@group(0) @binding(2) var<storage, read_write> next: array<u32, 4>;

@compute @workgroup_size(4)
fn step(@builtin(local_invocation_index) point: u32) {
    if frame.reset != 0u {
        next[point] = 17u + point * 3u;
    } else {
        next[point] = previous[point] + frame.steps * (point + 1u);
    }
}

@group(0) @binding(3) var<uniform> points: vec4<u32>;
@group(0) @binding(4) var<storage, read_write> answers: array<u32, 4>;

@compute @workgroup_size(4)
fn query(@builtin(local_invocation_index) id: u32) {
    if points[id] < 4u {
        answers[id] = previous[points[id]];
    } else {
        answers[id] = 0u;
    }
}
