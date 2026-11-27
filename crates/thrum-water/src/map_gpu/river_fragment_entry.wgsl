@fragment fn water_fragment(input:VertexOut)->Output {
    // Sampled charts and receiving charts use the refined draw. An ordinary
    // chart here keeps its material-space row. Coarser sea rings can still
    // overlap a displaced river bank, so test those at their world hit.
    let chart = map.bodies[surface_body(input.body)].layers.x == 2u;
    let point = river_ownership_point(input.world.xz, input.rest, chart || input.surf_weight > 0.0);
    return shade_water(input, surface_pixel(point,input.body));
}
fn shade_water(input:VertexOut, source:SurfaceRow)->Output {
