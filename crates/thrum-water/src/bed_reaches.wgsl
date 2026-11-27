fn bed_reaches(h: vec4<f32>, bed: f32, at: vec2<f32>, k: u32, level: f32, wet: bool) -> bool {
    let corner = vec2<f32>(f32(k & 1u), f32(k >> 1u));
    let uv = at - corner;
    let cross = h.w - h.z - h.y + h.x;
    let a = h[k];
    let b = uv.x * (h.y - h.x + cross * corner.y);
    let c = uv.y * (h.z - h.x + cross * corner.x);
    let d = cross * uv.x * uv.y;
    let descending = b + d < 0.0 && c + d < 0.0;
    let saddle = descending && b > 0.0 && c > 0.0;
    if wet {
        return a < level && bed < level && !(saddle && a - b*c/d >= level);
    }
    let end = a + b + c + d;
    if max(end, level) < max(a, level) || (a > level && b < 0.0 && c < 0.0) {
        return false;
    }
    return !(descending && end > level) && !(saddle && a - b*c/d > level);
}
