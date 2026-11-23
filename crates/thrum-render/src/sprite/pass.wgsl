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
    let filter_margin = select(1.0, 1.25, frame.kind == SPRITE_PIXEL);
    let margin = select(clamp(max(footprint.x, footprint.y), filter_margin, SPRITE_GUTTER), 0.0, frame.tile != 0u);
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

// Integral of the symmetric tent's CDF. Use symmetry to avoid subtracting
// large nearly equal cubes when the tent tends to a point.
fn tent_integral(x: f32, radius: f32) -> f32 {
    let tail = max(0.0, 1.0 - abs(x) / radius);
    return max(x, 0.0) + radius * tail * tail * tail / 6.0;
}

fn widened_cdf(x: f32, width: f32, radius: f32) -> f32 {
    let half_width = 0.5 * width;
    return clamp((tent_integral(x + half_width, radius) -
        tent_integral(x - half_width, radius)) / width, 0.0, 1.0);
}

struct AxisSamples {
    points: vec2<f32>,
    tail: f32,
}

fn widened_axis(point: f32, width: f32, strength: f32) -> AxisSamples {
    let scale = 1.0 / width;
    let radius = min(1.25, (0.5 + strength * 20.0 *
        pow(clamp((scale - 1.0) * (2.0 - scale), 0.0, 0.25), 2.0)) * width) - 0.5 * width;
    if radius < 0.0001 {
        let linear = coverage_coordinate(vec2(point), vec2(width)).x;
        return AxisSamples(vec2(linear), 0.0);
    }
    // The weight beyond the fourth texel is zero. Keep the two adjacent
    // pairs in hardware linear samples and combine them by their total mass.
    let edge = round(point);
    let left = widened_cdf(edge - point, width, radius);
    let far_left = widened_cdf(edge - 1.0 - point, width, radius);
    let near_right = widened_cdf(edge + 1.0 - point, width, radius) - left;
    return AxisSamples(vec2(
        edge - 0.5 - far_left / max(left, 0.000001),
        edge + 1.5 - near_right / max(1.0 - left, 0.000001)
    ), 1.0 - left);
}

fn widened_sample(sheet: u32, texel: vec2<f32>, footprint: vec2<f32>, page: i32, strength: f32) -> vec4<f32> {
    let x = widened_axis(texel.x, footprint.x, strength);
    let y = widened_axis(texel.y, footprint.y, strength);
    let first = mix(filtered(sheet, vec2(x.points.x, y.points.x), page, 0.0),
        filtered(sheet, vec2(x.points.y, y.points.x), page, 0.0), x.tail);
    let second = mix(filtered(sheet, vec2(x.points.x, y.points.y), page, 0.0),
        filtered(sheet, vec2(x.points.y, y.points.y), page, 0.0), x.tail);
    return mix(first, second, y.tail);
}

fn reconstruct_two_x(sheet: u32, frame: SpriteFrameRecord, texel: vec2<f32>, footprint: vec2<f32>,
    coverage: vec4<f32>) -> vec4<f32> {
    // Keep transparent or partially covered outlines on the coverage kernel.
    if coverage.a < 1.0 {
        return coverage;
    }
    let page = i32(frame.page);
    let origin = vec2<i32>(frame.origin);
    let cell = vec2<i32>(floor(texel - vec2(0.5)));
    let phase = texel - (vec2<f32>(cell) + vec2(1.0));
    // Advance both boundaries of a projected texel by the same subpixel
    // amount. The flat parts keep their width as the sample grid crosses
    // an edge, and the continuous shoulder still covers the crossing.
    let shoulder = 0.012 * footprint;
    let crossing = phase + 0.11 * footprint;
    let weight = smoothstep(-shoulder, shoulder, crossing);
    if all(weight == round(weight)) {
        return nearest(sheet, origin + cell + vec2<i32>(weight), page);
    }
    let a = nearest(sheet, origin + cell, page);
    let b = nearest(sheet, origin + cell + vec2(1, 0), page);
    let c = nearest(sheet, origin + cell + vec2(0, 1), page);
    let d = nearest(sheet, origin + cell + vec2(1, 1), page);
    // Interior fine marks lose the dark part of the signal when two or four
    // opaque texels straddle a sample. Interpolate along the same local axes
    // with a bias towards the darker contributors in the crossing interval.
    // Keep alpha and its premultiplied edge coverage on the ordinary kernel.
    if min(min(a.a, b.a), min(c.a, d.a)) < 1.0 {
        return coverage;
    }
    let luminance = vec3(0.2126, 0.7152, 0.0722);
    let separable = vec4((1.0 - weight.x) * (1.0 - weight.y), weight.x * (1.0 - weight.y),
        (1.0 - weight.x) * weight.y, weight.x * weight.y);
    let luma = vec4(dot(a.rgb, luminance), dot(b.rgb, luminance),
        dot(c.rgb, luminance), dot(d.rgb, luminance));
    let diagonals = abs(0.5 * (luma.x + luma.w - luma.y - luma.z));
    let diagonal_strength = smoothstep(0.05, 0.55, diagonals);
    // On a checker, the equal diagonal colours are a continuous route
    // through a double-axis crossing. Barycentric weights agree with the
    // one-dimensional interpolation on each outer edge of this cell.
    let triangle = vec4(1.0 - max(weight.x, weight.y), max(0.0, weight.x - weight.y),
        max(0.0, weight.y - weight.x), min(weight.x, weight.y));
    let weights = mix(separable, triangle, diagonal_strength);
    let dark_weights = weights / (vec4(mix(0.12, 0.02, diagonal_strength)) + luma);
    return (a * dark_weights.x + b * dark_weights.y + c * dark_weights.z + d * dark_weights.w) /
        dot(dark_weights, vec4(1.0));
}

fn source(sheet: u32, frame: SpriteFrameRecord, texel: vec2<f32>, footprint: vec2<f32>) -> vec4<f32> {
    let page = i32(frame.page);
    let origin = vec2<f32>(frame.origin);
    let size = vec2<f32>(frame.size);
    let minification = max(footprint.x, footprint.y);
    if frame.kind == SPRITE_PIXEL && minification <= 1.00001 {
        let scale = 1.0 / footprint;
        let strength = smoothstep(1.0, 1.25, min(scale.x, scale.y));
        let wide = (scale.x > 1.00001 && scale.x < 1.99999) ||
            (scale.y > 1.00001 && scale.y < 1.99999);
        let two_x = strength * (1.0 - smoothstep(0.0, 0.28, max(abs(scale.x - 2.0), abs(scale.y - 2.0))));
        var pixel = vec4(0.0);
        if wide && strength > 0.0 {
            pixel = widened_sample(sheet, origin + texel, footprint, page, strength);
        } else {
            pixel = filtered(sheet, origin + coverage_coordinate(texel, footprint), page, 0.0);
        }
        if two_x > 0.0 {
            pixel = mix(pixel, reconstruct_two_x(sheet, frame, texel, footprint, pixel), two_x);
        }
        return pixel;
    }
    // At the coarsest level this is the centre of the outer gutter texel.
    // Clamping also handles sprites smaller than one quarter of native size.
    let half_gutter = 0.5 * SPRITE_GUTTER;
    let point = clamp(texel, vec2(-half_gutter), ceil(size / SPRITE_GUTTER) * SPRITE_GUTTER + half_gutter);
    let lod = clamp(log2(minification), 0.0, SPRITE_MAX_MIP);
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
