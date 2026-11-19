enable wgpu_ray_query;

// Prefix of the forward view's auxiliary record.

@group(0) @binding(1) var<uniform> view_aux: ViewAuxRecord;
@group(1) @binding(0) var structure: acceleration_structure;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let triangle = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4(triangle[index], 0.0, 1.0);
}

fn instance_tint(identity: u32) -> vec3<f32> {
    // Golden-ratio hue spacing keeps successive scene identities distinct.
    let hue = f32((identity * 0x9e3779b9u) >> 8u) / 16777216.0;
    let phases = fract(hue + vec3(0.0, 2.0 / 3.0, 1.0 / 3.0));
    let rgb = clamp(abs(phases * 6.0 - 3.0) - 1.0, vec3(0.0), vec3(1.0));
    return mix(vec3(0.5), vec3(0.96), rgb);
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
    rayQueryInitialize(&query, structure, RayDesc(RAY_FLAG_FORCE_OPAQUE, 0xffu, 0.0, length(segment), origin, direction));
    while rayQueryProceed(&query) {}
    let hit = rayQueryGetCommittedIntersection(&query);
    let present = hit.kind != RAY_QUERY_INTERSECTION_NONE;
    let point = origin + direction * select(length(segment), hit.t, present);
    let cross_product = cross(dpdx(point), dpdy(point));
    let normal = cross_product * inverseSqrt(max(dot(cross_product, cross_product), 1e-20));
    let light = 0.42 + 0.46 * abs(dot(normal, normalize(vec3(-0.45, -1.0, -0.25))));
    let fade = 0.7 + 0.3 * exp(-select(0.0, hit.t, present) / 260.0);
    let color = instance_tint(hit.instance_custom_data) * light * fade;
    return vec4(select(vec3(0.09, 0.12, 0.16), color, present), 1.0);
}
