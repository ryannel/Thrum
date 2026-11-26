,current,previous,body,surf_weight,
        vec4(surf_normal, surf_variance), resolved_face * max(0.0, transformed.w - 1.0));
}

@vertex fn water_vertex(@location(0) vertex: vec4<f32>, @builtin(vertex_index) index: u32) -> VertexOut {
    return displaced_water_vertex(vertex, index);
}
