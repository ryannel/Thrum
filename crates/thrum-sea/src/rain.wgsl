struct BandFrame {
    lengths_reach: vec4<f32>,
    table: vec4<u32>,
    steps: vec4<f32>,
    scales: vec4<f32>,
}
@group(0) @binding(0) var<uniform> frame: BandFrame;
@group(0) @binding(1) var rings: texture_2d_array<f32>;
@group(0) @binding(2) var<storage, read> drops: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> tiles: array<u32>;
@group(0) @binding(4) var output: texture_storage_2d_array<rgba16float, write>;

@group(0) @binding(7) var ring_filter: sampler;

fn ring_sample(klass: u32, radius: f32, age: f32) -> vec3<f32> {
    let p = vec2(radius, age) / frame.steps.xy;
    let uv = (p + 0.5) / vec2<f32>(frame.table.xy);
    return frame.scales.xyz * textureSampleLevel(rings, ring_filter, uv, i32(klass), 0.0).xyz;
}

@compute @workgroup_size(8, 8)
fn rain_bands(@builtin(global_invocation_id) id: vec3<u32>) {
    let side = frame.lengths_reach[id.z];
    let p = (vec2<f32>(id.xy) + 0.5) * (side / f32(RAIN_SIZE));
    let cells = id.xy / (RAIN_SIZE / RAIN_TILES);
    let tile = (id.z * RAIN_TILES + cells.y) * RAIN_TILES + cells.x;
    let reach_squared = frame.lengths_reach.z * frame.lengths_reach.z;
    var sum = vec4(0.0);
    for (var i = tiles[tile]; i < tiles[tile+1u]; i += 1u) {
        let drop = drops[tiles[i]];
        let v = p - drop.xy;
        let squared = dot(v, v);
        if squared < reach_squared {
            let radius = sqrt(squared);
            let ring_class = u32(drop.w);
            let weight = fract(drop.w) * 2.0;
            let s = weight * ring_sample(ring_class, radius, drop.z);
            sum += vec4(s.x, s.y * v / max(radius, 1e-20), s.z);
        }
    }
    let layer = i32(id.z * 2u);
    let slope = sum.yz;
    textureStore(output, vec2<i32>(id.xy), layer, sum);
    textureStore(output, vec2<i32>(id.xy), layer+1,
        vec4(slope.x*slope.x, slope.x*slope.y, slope.y*slope.y, 0.0));
}

@group(0) @binding(5) var previous: texture_2d_array<f32>;
@group(0) @binding(6) var reduced: texture_storage_2d_array<rgba16float, write>;
@compute @workgroup_size(8, 8)
fn rain_mip(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= textureDimensions(reduced)) { return; }
    let p = vec2<i32>(id.xy * 2u);
    let z = i32(id.z);
    textureStore(reduced, vec2<i32>(id.xy), z, 0.25 * (
        textureLoad(previous, p, z, 0) + textureLoad(previous, p+vec2(1, 0), z, 0) +
        textureLoad(previous, p+vec2(0, 1), z, 0) + textureLoad(previous, p+vec2(1, 1), z, 0)));
}
