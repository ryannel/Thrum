struct SpriteView {
    clip_per_world: vec2<f32>,
    centre_high: vec2<f32>,
    centre_low: vec2<f32>,
    alpha: f32,
    pixels_per_unit: f32,
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
    page: u32,
    kind: u32,
    tile: u32,
    padding: u32,
}
struct Vertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) texel: vec2<f32>,
    @location(1) @interpolate(flat) row_index: u32,
    @location(2) @interpolate(flat) footprint: vec2<f32>,
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

@group(0) @binding(12) var filtering: sampler;

fn relative(position: vec4<f32>) -> vec2<f32> {
    return (position.xy - view.centre_high) + (position.zw - view.centre_low);
}

@vertex fn vertex(@builtin(vertex_index) index: u32, @builtin(instance_index) instance: u32) -> Vertex {
    let corners = array(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 1.0));
    let corner = corners[index];
    let row_index = order[instance];
    let row = rows[row_index];
    let frame = frames[row.frame];
    let size = vec2<f32>(frame.size);
    let pivot = mix(relative(row.previous), relative(row.current), view.alpha);
    let angle = row.rotation + row.rotation_delta * view.alpha;
    let scale = mix(row.scales.xy, row.scales.zw, view.alpha);
    let footprint = 1.0 / max(vec2(0.000001), abs(scale * view.pixels_per_unit));
    // A standalone sprite can reach across its outline. Tiles must retain
    // their original vertices so the rasterizer assigns their shared edge once.
    let margin = select(clamp(max(footprint.x, footprint.y), 1.0, 4.0), 0.0, frame.tile != 0u);
    let texel = corner * (size + 2.0 * margin) - margin;
    let local = texel * scale * vec2(1.0, -1.0);
    let sin_angle = sin(angle);
    let cos_angle = cos(angle);
    let world = pivot + mat2x2(vec2(cos_angle, sin_angle), vec2(-sin_angle, cos_angle)) * local;
    return Vertex(vec4(world * view.clip_per_world, 0.0, 1.0), texel, row_index, footprint);
}

fn nearest(sheet: u32, texel: vec2<i32>, page: i32) -> vec4<f32> {
    switch sheet {
        case 0u: { return textureLoad(sheet0, texel, page, 0); }
        case 1u: { return textureLoad(sheet1, texel, page, 0); }
        case 2u: { return textureLoad(sheet2, texel, page, 0); }
        case 3u: { return textureLoad(sheet3, texel, page, 0); }
        case 4u: { return textureLoad(sheet4, texel, page, 0); }
        case 5u: { return textureLoad(sheet5, texel, page, 0); }
        case 6u: { return textureLoad(sheet6, texel, page, 0); }
        default: { return textureLoad(sheet7, texel, page, 0); }
    }
}

fn filtered(sheet: u32, texel: vec2<f32>, page: i32, lod: f32) -> vec4<f32> {
    switch sheet {
        case 0u: { return textureSampleLevel(sheet0, filtering, texel / vec2<f32>(textureDimensions(sheet0)), page, lod); }
        case 1u: { return textureSampleLevel(sheet1, filtering, texel / vec2<f32>(textureDimensions(sheet1)), page, lod); }
        case 2u: { return textureSampleLevel(sheet2, filtering, texel / vec2<f32>(textureDimensions(sheet2)), page, lod); }
        case 3u: { return textureSampleLevel(sheet3, filtering, texel / vec2<f32>(textureDimensions(sheet3)), page, lod); }
        case 4u: { return textureSampleLevel(sheet4, filtering, texel / vec2<f32>(textureDimensions(sheet4)), page, lod); }
        case 5u: { return textureSampleLevel(sheet5, filtering, texel / vec2<f32>(textureDimensions(sheet5)), page, lod); }
        case 6u: { return textureSampleLevel(sheet6, filtering, texel / vec2<f32>(textureDimensions(sheet6)), page, lod); }
        default: { return textureSampleLevel(sheet7, filtering, texel / vec2<f32>(textureDimensions(sheet7)), page, lod); }
    }
}

fn coverage_coordinate(texel: vec2<f32>, footprint: vec2<f32>) -> vec2<f32> {
    // Convert the overlap of a one-pixel-wide box with the two adjacent
    // source texels to the weights of a linear sample.
    let edge = round(texel);
    return edge + clamp((texel - edge) / footprint, vec2(-0.5), vec2(0.5));
}

fn source(sheet: u32, frame: SpriteFrameRecord, texel: vec2<f32>, footprint: vec2<f32>) -> vec4<f32> {
    let page = i32(frame.page);
    let origin = vec2<f32>(frame.origin);
    let size = vec2<f32>(frame.size);
    if frame.kind == 2u {
        let cell = vec2<i32>(floor(texel));
        if frame.tile == 0u && (any(cell < vec2(0)) || any(cell >= vec2<i32>(frame.size))) {
            return vec4(0.0);
        }
        return nearest(sheet, vec2<i32>(frame.origin) + clamp(cell, vec2(0), vec2<i32>(frame.size) - 1), page);
    }

    let minification = max(footprint.x, footprint.y);
    var lod = 0.0;
    var point = texel;
    if frame.kind != 3u {
        lod = clamp(log2(minification), 0.0, 2.0);
        if frame.kind == 0u && minification <= 1.00001 {
            point = coverage_coordinate(texel, footprint);
        }
    }
    // At the coarsest level this is the centre of the outer gutter texel.
    // Clamping also handles sprites smaller than one quarter of native size.
    point = clamp(point, vec2(-2.0), ceil(size * 0.25) * 4.0 + 2.0);
    return filtered(sheet, origin + point, page, lod);
}

@fragment fn fragment(input: Vertex) -> @location(0) vec4<f32> {
    let row = rows[input.row_index];
    let frame = frames[row.frame];
    let texel = source(row.sheet, frame, input.texel, input.footprint);
    let alpha = texel.a * row.tint.a;
    let colour = mix(texel.rgb * row.tint.rgb * row.tint.a, row.flash.rgb * alpha, row.flash.a);
    return vec4(colour, alpha * (1.0 - row.additive));
}
