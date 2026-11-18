@group(2) @binding(0) var ground: texture_2d<f32>;
@group(2) @binding(1) var cloud: texture_3d<f32>;
@group(2) @binding(2) var product_sampler: sampler;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) product: u32,
}

@vertex fn vertex(@builtin(vertex_index) index: u32, @builtin(instance_index) product: u32) -> Vertex {
    let uv = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 0.0),
        vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 1.0)
    )[index];
    let position = vec2(-0.95 + 0.24 * f32(product), -0.56) + uv * vec2(0.21, -0.37);
    return Vertex(vec4(position, 1.0, 1.0), uv, product);
}

struct Fragment {
    @location(0) colour: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @location(2) reactive: f32,
}

@fragment fn fragment(input: Vertex) -> Fragment {
    let albedo = textureSample(ground, product_sampler, input.uv).rgb;
    let density = textureSample(cloud, product_sampler, vec3(input.uv, 0.5)).r;
    let colour = select(albedo, vec3(density), input.product == 1u);
    // These fixed screen-space previews have no geometric correspondence in history.
    return Fragment(vec4(colour, 1.0), vec2(0.0), 1.0);
}
