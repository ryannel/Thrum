var<workgroup> cloud_flags: array<u32, 64>;
var<workgroup> active_overlay: u32;

fn classify_overlay(index: u32) -> bool {
    cloud_flags[index] = u32(overlay_face_has_cloud(index));
    workgroupBarrier();
    if index == 0u {
        active_overlay = 0u;
        for (var i = 0u; i < 64u; i++) { active_overlay |= cloud_flags[i]; }
        overlay_chart.cloudy = active_overlay;
        overlay_chart.classifications += 1u;
    }
    return workgroupUniformLoad(&active_overlay) != 0u;
}

fn overlay_face_has_cloud(index: u32) -> bool {
    let extent = textureDimensions(overlay_faces);
    let face_size = extent.x * extent.y;
    for (var i = index; i < face_size * 6u; i += 64u) {
        let pixel = i % face_size;
        let texel = textureLoad(overlay_faces, vec2<i32>(i32(pixel % extent.x), i32(pixel / extent.x)),
            i32(i / face_size), 0);
        if any(texel != vec4(0.0, 0.0, 0.0, 1.0)) { return true; }
    }
    return false;
}
