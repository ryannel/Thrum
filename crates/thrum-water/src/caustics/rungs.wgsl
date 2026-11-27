fn caustic_rung_depth(rung: u32) -> f32 {
    return 0.25 * exp2(f32(rung) * 0.5);
}
