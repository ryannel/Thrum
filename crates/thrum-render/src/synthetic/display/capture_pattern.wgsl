// Mac window correspondence fixture. Exact RGBA8 code values, with independent
// horizontal, vertical and mixed channels. The normal SDR target and presenter
// consume this output. No dither, sampling or additional encoding is applied.
@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corners = array<vec2<f32>, 3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(corners[index],0.0,1.0);
}
@fragment fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let x = u32(position.x);
    let y = u32(position.y);
    let code = vec3<u32>(x % 251u, y % 241u, (x / 251u + y / 241u * 17u) % 239u);
    return vec4(vec3<f32>(code) / 255.0, 1.0);
}
