// Keep each 2×2 block's edge pixels on the same texel in trace and compose,
// including the final partial block of an odd render extent.
fn cloud_stop_offset(slot: u32) -> vec2<u32> {
    return vec2(slot % 2u, slot / 2u);
}

fn cloud_stop_slot(pixel: vec2<u32>) -> i32 {
    return i32(pixel.x % 2u + 2u * (pixel.y % 2u));
}
