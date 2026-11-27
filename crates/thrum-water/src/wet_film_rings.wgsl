fn film_rings(ground: vec3<f32>, normal: vec3<f32>, footprint: f32) -> vec2<f32> {
    if rain.grid.w == 0u { return vec2(0.0); }
    let exposure = rain_exposure(ground.xz, ground.y);
    if exposure <= 0.0 { return vec2(0.0); }
    var slope = vec2(0.0);
    for (var band = 0u; band < 2u; band += 1u) {
        let side = rain.lengths[band];
        let metres = footprint * rain.footprint_scale[band] / max(normal.y, 0.25);
        let mip = max(0.0, log2(metres / (side / 512.0)));
        let layer = i32(RAIN_FIRST_LAYER + 2u * band);
        slope += textureSampleLevel(cascades, rain_periodic, ground.xz / side, layer, mip).yz;
    }
    return slope * exposure * 0.16;
}
