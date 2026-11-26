@fragment fn water_fragment(input:VertexOut)->Output {
    // Coarser draws can still overlap a displaced river bank. Keep the
    // transformed surf sheet's material coverage, and test other draws
    // at the world hit just like the refined river and receiver draws.
    let point = river_ownership_point(input.world.xz, input.rest, input.surf_weight > 0.0);
    return shade_water(input, surface_pixel(point,input.body));
}
fn shade_water(input:VertexOut, source:SurfaceRow)->Output {
