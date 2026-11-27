// Keep each 2×2 block's edge pixels on the same texel in trace and compose,
// including the final partial block of an odd render extent.
fn cloud_stop_offset(slot: u32) -> vec2<u32> {
    return vec2(slot % 2u, slot / 2u);
}

fn cloud_stop_slot(pixel: vec2<u32>) -> i32 {
    return i32(pixel.x % 2u + 2u * (pixel.y % 2u));
}

fn cloud_texel_uv(texel: vec2<u32>) -> vec2<f32> {
    return (vec2<f32>(texel) + 0.5) / vec2<f32>(cloud.active_extent);
}

fn cloud_ground_texel_uv(texel: vec2<u32>) -> vec2<f32> {
    let extent = vec2<f32>(cloud.render_extent);
    return min(vec2<f32>(texel * 2u + 1u), extent - 0.5) / extent;
}

// Invert the ground sample grid, including the centre of an odd edge block.
fn cloud_ground_grid(uv: vec2<f32>) -> vec2<f32> {
    let extent = vec2<f32>(cloud.render_extent);
    let grid = (uv * extent - 1.0) * 0.5;
    let last = vec2<f32>(cloud.active_extent) - 1.0;
    let odd_shift = vec2<f32>(cloud.render_extent % 2u) * 0.25;
    let previous = last - 1.0;
    return grid + clamp((grid - previous) / (1.0 - odd_shift), vec2(0.0), vec2(1.0)) * odd_shift;
}

// Only a whole-ray stop can contribute to the sky's spatial or history filter.
fn cloud_sky_tap(field: texture_2d_array<f32>, metadata: texture_2d_array<f32>,
                 texel: vec2<i32>, weight: f32, total_weight: ptr<function, f32>) -> vec4<f32> {
    let at = clamp(texel, vec2(0), vec2<i32>(cloud.active_extent) - 1);
    for (var slot = 0; slot < 4; slot++) {
        if textureLoad(metadata, at, slot, 0).w == 0.0 {
            *total_weight += weight;
            return textureLoad(field, at, slot, 0) * weight;
        }
    }
    return vec4(0.0);
}

fn cloud_ground_sky(field: texture_2d_array<f32>, metadata: texture_2d_array<f32>,
                    uv: vec2<f32>) -> vec4<f32> {
    let grid = cloud_ground_grid(uv);
    let base = vec2<i32>(floor(grid));
    let f = fract(grid);
    var weight = 0.0;
    let a = cloud_sky_tap(field, metadata, base, (1.0 - f.x) * (1.0 - f.y), &weight);
    let b = cloud_sky_tap(field, metadata, base + vec2(1, 0), f.x * (1.0 - f.y), &weight);
    let c = cloud_sky_tap(field, metadata, base + vec2(0, 1), (1.0 - f.x) * f.y, &weight);
    let d = cloud_sky_tap(field, metadata, base + vec2(1), f.x * f.y, &weight);
    // The caller uses its fresh opacity if no history tap was eligible.
    return select(vec4(0.0, 0.0, 0.0, -1.0), (a + b + c + d) / max(weight, 1e-8), weight > 0.0);
}
