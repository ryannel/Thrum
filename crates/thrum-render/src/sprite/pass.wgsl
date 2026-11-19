struct SpriteView {
    clip_per_world: vec2<f32>,
    padding: vec2<f32>,
}
struct SpriteRow {
    pivot: vec2<f32>,
    right: vec2<f32>,
    down: vec2<f32>,
    frame: u32,
    padding: u32,
    tint: vec4<f32>,
}
struct SpriteFrameRecord {
    origin: vec2<u32>,
    size: vec2<u32>,
}
struct Vertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) texel: vec2<f32>,
    @location(1) @interpolate(flat) frame: u32,
    @location(2) @interpolate(flat) tint: vec4<f32>,
}
@group(0) @binding(0) var<uniform> view: SpriteView;
@group(0) @binding(1) var<storage, read> rows: array<SpriteRow>;
@group(0) @binding(2) var<storage, read> frames: array<SpriteFrameRecord>;
@group(0) @binding(3) var sheet: texture_2d_array<f32>;

@vertex fn vertex(@builtin(vertex_index) index: u32, @builtin(instance_index) instance: u32) -> Vertex {
    let corners = array(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
                        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
    let corner = corners[index];
    let row = rows[instance];
    let frame = frames[row.frame];
    let size = vec2<f32>(frame.size);
    let world = row.pivot + row.right * (corner.x * size.x) + row.down * (corner.y * size.y);
    return Vertex(vec4(world * view.clip_per_world, 0.0, 1.0), corner * size, row.frame, row.tint);
}

@fragment fn fragment(input: Vertex) -> @location(0) vec4<f32> {
    let frame = frames[input.frame];
    let local = clamp(vec2<i32>(input.texel), vec2(0), vec2<i32>(frame.size) - vec2(1));
    let texel = textureLoad(sheet, vec2<i32>(frame.origin) + local, 0, 0);
    return texel * vec4(input.tint.rgb * input.tint.a, input.tint.a);
}
