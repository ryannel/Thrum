struct SpriteView {
    clip_per_world: vec2<f32>,
    centre_high: vec2<f32>,
    centre_low: vec2<f32>,
    alpha: f32,
    padding: f32,
}
struct SpriteRow {
    current: vec4<f32>,
    previous: vec4<f32>,
    scales: vec4<f32>,
    rotation: f32,
    rotation_delta: f32,
    frame: u32,
    sheet: u32,
    tint: vec4<f32>,
    flash: vec4<f32>,
    additive: f32,
    padding0: u32,
    padding1: u32,
    padding2: u32,
}
struct SpriteFrameRecord {
    origin: vec2<u32>,
    size: vec2<u32>,
}
struct Vertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) texel: vec2<f32>,
    @location(1) @interpolate(flat) row_index: u32,
}
@group(0) @binding(0) var<uniform> view: SpriteView;
@group(0) @binding(1) var<storage, read> rows: array<SpriteRow>;
@group(0) @binding(2) var<storage, read> frames: array<SpriteFrameRecord>;
@group(0) @binding(3) var<storage, read> order: array<u32>;
@group(0) @binding(4) var sheet0: texture_2d_array<f32>;
@group(0) @binding(5) var sheet1: texture_2d_array<f32>;
@group(0) @binding(6) var sheet2: texture_2d_array<f32>;
@group(0) @binding(7) var sheet3: texture_2d_array<f32>;
@group(0) @binding(8) var sheet4: texture_2d_array<f32>;
@group(0) @binding(9) var sheet5: texture_2d_array<f32>;
@group(0) @binding(10) var sheet6: texture_2d_array<f32>;
@group(0) @binding(11) var sheet7: texture_2d_array<f32>;

fn relative(position: vec4<f32>) -> vec2<f32> {
    return (position.xy - view.centre_high) + (position.zw - view.centre_low);
}

@vertex fn vertex(@builtin(vertex_index) index: u32, @builtin(instance_index) instance: u32) -> Vertex {
    let corners = array(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 1.0));
    let corner = corners[index];
    let row_index = order[instance];
    let row = rows[row_index];
    let size = vec2<f32>(frames[row.frame].size);
    let pivot = mix(relative(row.previous), relative(row.current), view.alpha);
    let angle = row.rotation + row.rotation_delta * view.alpha;
    let scale = mix(row.scales.xy, row.scales.zw, view.alpha);
    let local = corner * size * scale * vec2(1.0, -1.0);
    let sin_angle = sin(angle);
    let cos_angle = cos(angle);
    let world = pivot + mat2x2(vec2(cos_angle, sin_angle), vec2(-sin_angle, cos_angle)) * local;
    return Vertex(vec4(world * view.clip_per_world, 0.0, 1.0), corner * size, row_index);
}

fn source(sheet: u32, texel: vec2<i32>) -> vec4<f32> {
    switch sheet {
        case 0u: { return textureLoad(sheet0, texel, 0, 0); }
        case 1u: { return textureLoad(sheet1, texel, 0, 0); }
        case 2u: { return textureLoad(sheet2, texel, 0, 0); }
        case 3u: { return textureLoad(sheet3, texel, 0, 0); }
        case 4u: { return textureLoad(sheet4, texel, 0, 0); }
        case 5u: { return textureLoad(sheet5, texel, 0, 0); }
        case 6u: { return textureLoad(sheet6, texel, 0, 0); }
        default: { return textureLoad(sheet7, texel, 0, 0); }
    }
}

@fragment fn fragment(input: Vertex) -> @location(0) vec4<f32> {
    let row = rows[input.row_index];
    let frame = frames[row.frame];
    let local = clamp(vec2<i32>(input.texel), vec2(0), vec2<i32>(frame.size) - vec2(1));
    let texel = source(row.sheet, vec2<i32>(frame.origin) + local);
    let alpha = texel.a * row.tint.a;
    let colour = mix(texel.rgb * row.tint.rgb * row.tint.a, row.flash.rgb * alpha, row.flash.a);
    return vec4(colour, alpha * (1.0 - row.additive));
}
