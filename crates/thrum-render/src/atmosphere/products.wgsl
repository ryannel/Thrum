// Read-only environment operations. Inputs and outputs use physical scene units.
@group(0) @binding(1) var surface_transmittance: texture_2d<f32>;
@group(0) @binding(2) var aerial_sampler: sampler;
@group(0) @binding(3) var<uniform> environment_light: EnvironmentLight;
@group(0) @binding(4) var environment_sky: texture_2d<f32>;
@group(0) @binding(5) var aerial_scatter: texture_2d_array<f32>;
@group(0) @binding(6) var aerial_transmittance: texture_2d_array<f32>;
@group(0) @binding(7) var sky_reflection: texture_cube<f32>;
@group(0) @binding(8) var sky_dfg: texture_2d<f32>;
@group(0) @binding(9) var sky_reflection_sampler: sampler;
@group(0) @binding(10) var<uniform> environment_exposure: vec4<f32>;
@group(0) @binding(11) var<uniform> environment_directional: DirectionalLight;

fn environment_sun_direction() -> vec3<f32> {
    return environment_directional.direction_to_light.xyz;
}

// This scale is applied once by the caller after lighting and aerial composition.
fn environment_storage_scale() -> f32 { return environment_exposure.x; }

fn environment_clear_sky_radiance(direction: vec3<f32>) -> vec3<f32> {
    let size=vec2<f32>(textureDimensions(environment_sky));
    let uv=clamp(sky_uv(normalize(direction),atmosphere.sun_lux.xyz),0.5/size,1.0-0.5/size);
    let encoded=textureSampleLevel(environment_sky,aerial_sampler,uv,0.0);
    return decode_sky_light(encoded,atmosphere.sun_lux.w*atmosphere.colour_pre_exposure.rgb);
}

fn environment_diffuse(normal: vec3<f32>) -> vec3<f32> {
    return max(evaluate_sky_diffuse(environment_light.sky_diffuse,normalize(normal)),vec3(0.0));
}

struct EnvironmentAerial {
    transmission: vec3<f32>,
    scatter: vec3<f32>,
}
