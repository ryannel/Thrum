@group(1) @binding(8) var sky_reflection: texture_cube<f32>;
@group(1) @binding(9) var sky_dfg: texture_2d<f32>;
@group(1) @binding(10) var sky_reflection_sampler: sampler;

fn indirect_radiance(base: vec3<f32>, metalness: f32, roughness: f32,
                     n: vec3<f32>, v: vec3<f32>, ao: f32,
                     world_position: vec3<f32>, pixel_position: vec4<f32>) -> vec3<f32> {
