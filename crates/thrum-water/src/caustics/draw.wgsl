struct CausticVertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) displacement_in_texels: vec2<f32>,
}

@vertex fn caustic_vertex(
    @builtin(vertex_index) vertex: u32,
    @builtin(instance_index) rung: u32,
) -> CausticVertex {
    let side = select(select(128u, 256u, rung < 8u), 512u, rung < 4u);
    let node = vec2(vertex % (side + 1u), vertex / (side + 1u));
    let extent = caustic_draw.window.z;
    let depth = caustic_rung_depth(rung);
    let guard = min(depth * 0.2, 2.0);
    let spacing = (extent + 2.0 * guard) / f32(side);
    let rest = vec2<f32>(node) * spacing - guard;
    let point = caustic_draw.window.xy + rest;
    let row = surface_pixel(point, caustic_slot(point));
    let field = field_at(point, spacing, row);
    let wake = wake_at(point, 0.0);
    let x = vec2(1.0 + field.slope.z, field.slope.w);
    let z = vec2(field.slope.w, 1.0 + field.velocity.w);
    let slope = field.slope.xy + wake.yz;
    let du = vec3(x.x, slope.x, x.y);
    let dv = vec3(z.x, slope.y, z.y);
    let normal = normalize(cross(dv, du));
    let sun = caustic_draw.sun.xyz;
    let mean = refract(-sun, vec3(0.0, 1.0, 0.0), 1.0 / 1.333);
    let beam = refract(-sun, normal, 1.0 / 1.333);
    let offset = beam.xz / max(-beam.y, 0.05);
    let mean_offset = mean.xz / max(-mean.y, 0.05);
    let shift = field.displacement.xz + (offset - mean_offset) * depth
        + offset * (field.displacement.y + wake.x);
    let displacement = select(vec2(0.0), shift, caustic_draw.window.w > 0.0 && row.owned);
    let projected = rest + displacement;
    let clip = projected * (2.0 / extent) - 1.0;
    return CausticVertex(vec4(clip.x, -clip.y, 0.0, 1.0), displacement * (512.0 / extent));
}

@fragment fn caustic_fragment(input: CausticVertex) -> @location(0) f32 {
    let x = vec2(1.0, 0.0) - dpdxFine(input.displacement_in_texels);
    let y = vec2(0.0, 1.0) - dpdyFine(input.displacement_in_texels);
    // Each landing triangle carries the irradiance of its original area.
    // Taking the absolute determinant keeps crossings additive after a fold.
    return min(abs(x.x * y.y - x.y * y.x), 128.0);
}
