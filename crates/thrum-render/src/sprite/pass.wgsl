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

fn sprite_pivot(row: SpriteRow) -> vec2<f32> {
    return mix(relative(row.previous), relative(row.current), view.alpha);
}

@vertex fn vertex(@builtin(vertex_index) index: u32, @builtin(instance_index) instance: u32) -> Vertex {
    let corners = array(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 1.0));
    let corner = corners[index];
    let row_index = order[instance];
    let row = rows[row_index];
    let frame = frames[row.frame];
    let size = vec2<f32>(frame.size);
    let pivot = sprite_pivot(row);
    let angle = row.rotation + row.rotation_delta * view.alpha;
    let scale = mix(row.scales.xy, row.scales.zw, view.alpha);
    let footprint = 1.0 / max(vec2(0.000001), abs(scale * view.pixels_per_unit));
    // A standalone sprite can reach across its outline. Tiles must retain
    // their original vertices so the rasterizer assigns their shared edge once.
    let filter_margin = select(1.0, SPRITE_PIXEL_FILTER_RADIUS, frame.kind == SPRITE_PIXEL);
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
    let radius = min(SPRITE_PIXEL_FILTER_RADIUS, (0.5 + strength * 20.0 *
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

fn perceptual_weight(root_luma: f32, neighbours: vec4<f32>, masses: vec4<f32>) -> f32 {
    return 1.0 + dot(masses, (neighbours - root_luma) / (neighbours + root_luma));
}

fn projected_texel(position: vec2<f32>, row: SpriteRow, footprint: vec2<f32>) -> vec2<f32> {
    // Invert the sprite projection at the pixel centre. The rasterizer may
    // snap quad vertices to a grid coarser than the narrow colour shoulder.
    let extent = round(2.0 * (view.pixels_per_unit / view.clip_per_world));
    let screen_to_world_axes = vec2(1.0, -1.0);
    let relative_pixels = (position - 0.5 * extent) * screen_to_world_axes;
    let delta = relative_pixels - sprite_pivot(row) * view.pixels_per_unit;
    let angle = row.rotation + row.rotation_delta * view.alpha;
    let y = sin(angle);
    let x = cos(angle);
    let local_pixels = mat2x2(vec2(x, -y), vec2(y, x)) * delta;
    let scale = mix(row.scales.xy, row.scales.zw, view.alpha);
    return local_pixels * footprint * sign(scale) * screen_to_world_axes;
}

fn reconstruct_two_x(sheet: u32, frame: SpriteFrameRecord, texel: vec2<f32>, footprint: vec2<f32>,
    coverage: vec4<f32>) -> vec4<f32> {
    let page = i32(frame.page);
    let origin = vec2<i32>(frame.origin);
    let cell = vec2<i32>(floor(texel - vec2(0.5)));
    let phase = texel - (vec2<f32>(cell) + vec2(1.0));
    // Centre the narrow colour blend on the projected edge everywhere
    // along its length. Coverage alpha still uses the full footprint.
    let shoulder = footprint / 384.0;
    let blend = smoothstep(-shoulder, shoulder, phase);
    // Prefer the lower texel when the pixel centre lands on the edge.
    // A centred blend can soften every mark at once when a scroll lands
    // on that exact phase. Keep the support and its endpoints continuous.
    let blend_squared = blend * blend;
    let blend_fourth = blend_squared * blend_squared;
    let weight = blend_fourth * blend_fourth;
    if all(weight == round(weight)) {
        let sample = nearest(sheet, origin + cell + vec2<i32>(weight), page);
        return vec4(coverage.rgb * (1.0 - sample.a) + coverage.a * sample.rgb, coverage.a);
    }
    let a = nearest(sheet, origin + cell, page);
    let b = nearest(sheet, origin + cell + vec2(1, 0), page);
    let c = nearest(sheet, origin + cell + vec2(0, 1), page);
    let d = nearest(sheet, origin + cell + vec2(1, 1), page);
    // At a corner, correlate the crossings so the checker keeps some of its
    // diagonal contrast. Moving equal mass off the two opposing taps keeps
    // both axis weights unchanged, including at the end of either shoulder.
    let diagonal = 0.75 * (min(weight.x, weight.y) - weight.x * weight.y);
    let weights = vec4((1.0 - weight.x) * (1.0 - weight.y) + diagonal,
        weight.x * (1.0 - weight.y) - diagonal,
        (1.0 - weight.x) * weight.y - diagonal, weight.x * weight.y + diagonal);
    let luminance = vec3(0.2126, 0.7152, 0.0722);
    let opacity = vec4(a.a, b.a, c.a, d.a);
    let masses = weights * opacity;
    let mass = dot(masses, vec4(1.0));
    // Transfer equal weight between each pair, using straight luminance
    // to make a complementary crossing near-linear in perceived light.
    // The signed ratios are bounded, so weights stay nonnegative. Apply
    // them to premultiplied scene-linear colour and retain box alpha.
    let luma = vec4(dot(a.rgb, luminance), dot(b.rgb, luminance),
        dot(c.rgb, luminance), dot(d.rgb, luminance));
    let root_luma = sqrt(luma / max(opacity, vec4(0.000001)) + 0.000001);
    let normalised = masses / max(mass, 0.000001);
    let adjusted = weights * vec4(
        perceptual_weight(root_luma.x, root_luma, normalised),
        perceptual_weight(root_luma.y, root_luma, normalised),
        perceptual_weight(root_luma.z, root_luma, normalised),
        perceptual_weight(root_luma.w, root_luma, normalised));
    let colour = a.rgb * adjusted.x + b.rgb * adjusted.y + c.rgb * adjusted.z + d.rgb * adjusted.w;
    return vec4(coverage.rgb * (1.0 - mass) + coverage.a * colour, coverage.a);
}

fn source(row: SpriteRow, frame: SpriteFrameRecord, input: Vertex) -> vec4<f32> {
    let sheet = row.sheet;
    let texel = input.texel;
    let footprint = input.footprint;
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
            let projected = projected_texel(input.clip.xy, row, footprint);
            pixel = mix(pixel, reconstruct_two_x(sheet, frame, projected, footprint, pixel), two_x);
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
    let texel = source(row, frame, input);
    let alpha = texel.a * row.tint.a;
    let colour = mix(texel.rgb * row.tint.rgb * row.tint.a, row.flash.rgb * alpha, row.flash.a);
    return vec4(colour, alpha * (1.0 - row.additive));
}
