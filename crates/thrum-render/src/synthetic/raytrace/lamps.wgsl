struct LampShadowQuality {
    ray_limit: u32,
}

@group(1) @binding(16) var<uniform> lamp_shadow_quality: LampShadowQuality;
@group(1) @binding(17) var<storage, read_write> unshadowed_lamp_references: atomic<u32>;

fn lamp_blocked(point: vec3<f32>, normal: vec3<f32>, lamp: vec3<f32>) -> bool {
    // Winding flips under mirrored transforms. The transformed authored
    // normal still points out of the face on the side lit by this lamp.
    let to_lamp = lamp - point;
    let lit_normal = select(-normal, normal, dot(normal, to_lamp) >= 0.0);
    let origin = shadow_ray_origin(point, lit_normal);
    let offset = lamp - origin;
    let distance = length(offset);
    if distance <= 1e-6 || dot(offset, to_lamp) <= 0.0 { return false; }
    let end = bitcast<f32>(bitcast<u32>(distance) - 1u);
    return shadow_ray_blocked(origin, offset / distance, end);
}
