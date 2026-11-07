// The synthetic output stores encoded sRGB values in Rgba8Unorm. A non-sRGB
// surface receives them unchanged; an sRGB attachment needs a decode before
// its automatic encoding. Nearest source loads preserve exact diagnostic bytes.
@group(0) @binding(0) var source: texture_2d<f32>;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex fn vertex(@builtin(vertex_index) index: u32) -> VertexOut {
    let p = array<vec2<f32>, 3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0))[index];
    var out: VertexOut;
    out.position = vec4(p, 0.0, 1.0);
    // Texture row zero is topmost; clip-space Y points upward.
    out.uv = vec2((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
    return out;
}

fn stored(uv: vec2<f32>) -> vec4<f32> {
    let size = textureDimensions(source);
    let xy = clamp(vec2<i32>(uv * vec2<f32>(size)), vec2<i32>(0), vec2<i32>(size) - vec2<i32>(1));
    return textureLoad(source, xy, 0);
}

@fragment fn unorm_surface(in: VertexOut) -> @location(0) vec4<f32> {
    return stored(in.uv);
}

@fragment fn srgb_surface(in: VertexOut) -> @location(0) vec4<f32> {
    let encoded = stored(in.uv);
    let low = encoded.rgb / 12.92;
    let high = pow((encoded.rgb + vec3(0.055)) / 1.055, vec3(2.4));
    let linear = select(high, low, encoded.rgb <= vec3(0.04045));
    return vec4(linear, encoded.a);
}
