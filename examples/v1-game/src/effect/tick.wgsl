struct Posts {
    count: vec4<u32>,
    points: array<vec4<u32>, CAPACITY>,
}

@group(0) @binding(0) var<uniform> posts: Posts;
@group(0) @binding(1) var trail: texture_2d<u32>;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;

@compute @workgroup_size(CAPACITY)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= posts.count.x {
        return;
    }
    let point = posts.points[id.x];
    if point.z != 0u {
        results[id.x] = textureLoad(trail, vec2<i32>(point.xy), 0).x;
    } else {
        results[id.x] = 0u;
    }
}
