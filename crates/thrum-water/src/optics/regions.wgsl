struct OpticsBody {
    low: vec4<f32>,
    high: vec4<f32>,
}
struct OpticsInputs {
    extinction_sight: vec4<f32>,
    downwelling_mirror: vec4<f32>,
    reach_slope_scene: vec4<f32>,
    dispatch: vec4<u32>,
    mask: array<vec4<u32>, 144>,
    bodies: array<OpticsBody, 16>,
}
@group(3) @binding(14) var<uniform> optics: OpticsInputs;

const GATHER_SOURCE: u32 = 0x80000000u;
const GATHER_SHALLOW: u32 = 0x40000000u;
const GATHER_BODY_MASK: u32 = 0xffffu;

fn optics_extent() -> vec2<u32> {
    return vec2<u32>(view_aux.viewport_inv_viewport.xy);
}

fn optics_mask(pixel: vec2<u32>) -> u32 {
    let extent = optics_extent();
    let cell = pixel * min(extent, vec2(32u, 18u)) / extent;
    return optics.mask[cell.y * 8u + cell.x / 4u][cell.x % 4u];
}

fn optics_uv(pixel: vec2<u32>) -> vec2<f32> {
    return (vec2<f32>(pixel) + 0.5) * view_aux.viewport_inv_viewport.zw;
}

fn optics_ndc(uv: vec2<f32>, depth: f32) -> vec4<f32> {
    return vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), depth, 1.0);
}

fn optics_project(point: vec3<f32>) -> vec3<f32> {
    let p = view.view_proj * vec4(point, 1.0);
    if p.w <= 0.0 { return vec3(-1.0); }
    return vec3(p.xy / p.w * vec2(0.5, -0.5) + 0.5, p.z / p.w);
}
