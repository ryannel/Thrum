enable wgpu_ray_query;

// Prefix of the forward view's auxiliary record.
struct ViewAux {
    inverse_view_proj: mat4x4<f32>,
    viewport_inv_viewport: vec4<f32>,
}

@group(0) @binding(1) var<uniform> view_aux: ViewAux;
@group(1) @binding(0) var structure: acceleration_structure;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let triangle = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4(triangle[index], 0.0, 1.0);
}

@fragment
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let ndc = pixel.xy * view_aux.viewport_inv_viewport.zw * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    let near_h = view_aux.inverse_view_proj * vec4(ndc, 1.0, 1.0);
    let far_h = view_aux.inverse_view_proj * vec4(ndc, 0.0, 1.0);
    let origin = near_h.xyz / near_h.w;
    let segment = far_h.xyz / far_h.w - origin;
    let direction = normalize(segment);
    var query: ray_query;
    rayQueryInitialize(&query, structure, RayDesc(0u, 0xffu, 0.0, length(segment), origin, direction));
    rayQueryProceed(&query);
    let hit = rayQueryGetCommittedIntersection(&query);
    let present = hit.kind != RAY_QUERY_INTERSECTION_NONE;
    let point = origin + direction * select(length(segment), hit.t, present);
    let cross_product = cross(dpdx(point), dpdy(point));
    let normal = cross_product * inverseSqrt(max(dot(cross_product, cross_product), 1e-20));
    let light = 0.42 + 0.46 * abs(dot(normal, normalize(vec3(-0.45, -1.0, -0.25))));
    let fade = 0.7 + 0.3 * exp(-select(0.0, hit.t, present) / 260.0);
    let color = vec3(0.76, 0.88, 0.96) * light * fade;
    return vec4(select(vec3(0.09, 0.12, 0.16), color, present), 1.0);
}
