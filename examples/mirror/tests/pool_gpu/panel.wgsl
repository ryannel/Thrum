struct RadiancePanel { place: vec4<f32>, half_size: vec4<f32> }
@group(2) @binding(0) var<uniform> panel: RadiancePanel;
struct PanelVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
}
@vertex fn panel_vertex(@builtin(vertex_index) index: u32) -> PanelVertex {
    let corner = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(-1.0, 1.0), vec2(1.0, -1.0),
        vec2(1.0, -1.0), vec2(-1.0, 1.0), vec2(1.0, 1.0))[index];
    let half = panel.half_size.xyz;
    let offset = select(vec3(corner.x * half.x, corner.y * half.y, half.z),
                        vec3(corner.x * half.x, half.y, corner.y * half.z), half.y < half.z);
    let world = panel.place.xyz + offset;
    return PanelVertex(view.view_proj * vec4(world, 1.0), world);
}
fn panel_colour(input: PanelVertex) -> vec4<f32> {
    let colour = aerial_colour(vec3(panel.place.w), input.world, input.position);
    return vec4(min(colour * scene_storage_scale(), vec3(65504.0)), 1.0);
}
