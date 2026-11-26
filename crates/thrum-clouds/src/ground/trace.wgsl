fn ground_interval(camera: vec3<f32>, ray: vec3<f32>, end: f32) -> vec2<f32> {
    if ground_frame.has_active_rows == 0u { return vec2(end, 0.0); }
    let heights = ground_frame.ground_heights.xy;
    if abs(ray.y) < 1e-7 {
        return vec2(0.0, select(0.0, end, camera.y >= heights.x && camera.y < heights.y));
    }
    let at = (heights - camera.y) / ray.y;
    return clamp(vec2(min(at.x, at.y), max(at.x, at.y)), vec2(0.0), vec2(end));
}
