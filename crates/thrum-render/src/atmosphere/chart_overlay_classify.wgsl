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
