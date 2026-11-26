@fragment fn water_fragment(input:VertexOut)->Output {
    return shade_water(input, surface_pixel(input.rest,input.body));
}
fn shade_water(input:VertexOut, source:SurfaceRow)->Output {
