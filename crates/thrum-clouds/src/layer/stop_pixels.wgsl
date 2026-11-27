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

// The sky filter reads the same depth tap the single-stop trace used.
// At odd extents the tap can land on either side of a 2×2 block.
fn cloud_sky_stop_slot(texel: vec2<u32>) -> i32 {
    let pixel = vec2<u32>(cloud_texel_uv(texel) * vec2<f32>(cloud.render_extent));
    return cloud_stop_slot(pixel);
}
