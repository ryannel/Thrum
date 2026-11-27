
fn river_ownership_point(world: vec2<f32>, rest: vec2<f32>, transformed: bool) -> vec2<f32> {
    return select(world, rest, transformed);
}

fn river_sheet_height(
    point: vec2<f32>, spacing: f32, row: SurfaceRow, raster: f32, datum: f32,
    transformed: bool, weight: f32,
) -> f32 {
    if weight == 0.0 { return raster; }
    var physical = raster + (row.level - datum);
    if !transformed {
        physical = river_column_level(point, spacing, row);
    }
    return mix(raster, physical, weight);
}

@fragment fn river_fragment(input: VertexOut) -> RiverOutput {
    // Derivatives describe the rasterized triangle's map back to rest space.
    let x = dpdx(input.world);
    let y = dpdy(input.world);
    let rest_x = dpdx(input.rest);
    let rest_y = dpdy(input.rest);
    let transformed = input.surf_weight > 0.0;
    let mesh_level = input.river.x;
    let retained = input.world.y - mesh_level;
    let retained_gradient = vec2(dpdx(retained), dpdy(retained));
    let mesh_gradient = vec2(x.y, y.y);
    let mesh_det = cross2(x.xz, y.xz);
    var surface = input;
    var depth = input.clip.z;
    let spacing = input.river.y;
    let weight = river_refinement(input.world.xz, surface_body(input.body), spacing);
    // A displaced receiving sheet can overlap a river even outside the
    // narrow join correction. Plain sheets take ownership at the world hit.
    var row = surface_pixel(river_ownership_point(input.world.xz, input.rest, transformed), input.body);
    let ndc = fma(input.clip.xy * view_aux.viewport_inv_viewport.zw, vec2(2.0, -2.0), vec2(-1.0, 1.0));
    let m = view.view_proj;
    let a = fma(-ndc, vec2(m[0].w), m[0].xy);
    let b = fma(-ndc, vec2(m[2].w), m[2].xy);
    let det = cross2(a, b);
    if weight > 0.0 && row.owned && abs(det) > 0.001 && abs(mesh_det) > 1e-12 {
        // At a fixed height, the pixel's clip x/w and y/w are two linear
        // constraints on world x/z. Work in offsets from the mesh hit.
        let clip = river_clip(input.world);
        let error = fma(-ndc, vec2(clip.w), clip.xy);
        let origin = solve_horizontal(a, b, -error, det);
        let vertical = fma(-ndc, vec2(m[1].w), m[1].xy);
        let direction = solve_horizontal(a, b, -vertical, det);
        var height = 0.0;
        var previous_height = 0.0;
        var previous_residual = 0.0;
        var best_error = 1e30;
        var bracketed = false;
        var opposite_height = 0.0;
        var opposite_residual = 0.0;
