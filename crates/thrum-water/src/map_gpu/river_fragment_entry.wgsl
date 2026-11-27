@fragment fn water_fragment(input:VertexOut)->Output {
    // The coarse rings of a receiving sea skip the refined draw but can
    // overlap a displaced river bank. Ordinary bodies keep their material row.
    let coarse_receiver = SEA_RECEIVES_RIVER && input.body == 0u;
    let point = river_ownership_point(input.world.xz, input.rest, !coarse_receiver || input.surf_weight > 0.0);
    return shade_water(input, surface_pixel(point,input.body));
}
fn shade_water(input:VertexOut, source:SurfaceRow)->Output {
