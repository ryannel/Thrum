// Read-only environment operations. Inputs and outputs use physical scene units.
@group(0) @binding(1) var surface_transmittance: texture_2d<f32>;
@group(0) @binding(2) var aerial_sampler: sampler;
struct EnvironmentDirectional {
    direction_to_light: vec4<f32>,
    irradiance: vec4<f32>,
    half_vector_fresnel: vec4<f32>,
    ambient_irradiance: vec4<f32>,
    sky_diffuse: array<vec4<f32>,9>,
}
@group(0) @binding(3) var<uniform> environment_light: EnvironmentDirectional;
@group(0) @binding(4) var environment_sky: texture_2d<f32>;
@group(0) @binding(5) var aerial_scatter: texture_2d_array<f32>;
@group(0) @binding(6) var aerial_transmittance: texture_2d_array<f32>;
@group(0) @binding(7) var sky_reflection: texture_cube<f32>;
@group(0) @binding(8) var sky_dfg: texture_2d<f32>;
@group(0) @binding(9) var sky_reflection_sampler: sampler;
@group(0) @binding(10) var<uniform> environment_exposure: vec4<f32>;

// This scale is applied once by the caller after lighting and aerial composition.
fn environment_storage_scale() -> f32 { return environment_exposure.x; }

fn environment_sky_radiance(direction: vec3<f32>) -> vec3<f32> {
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
// uv is the point's normalized position in the active view, top-left origin.
fn environment_aerial(world_point: vec3<f32>, uv: vec2<f32>) -> EnvironmentAerial {
    let sample=aerial_sample(length(world_point-atmosphere.camera_altitude.xyz),clamp(uv,vec2(0.0),vec2(1.0)));
    return EnvironmentAerial(sample.transmission,aerial_radiance(sample.scatter.rgb)+vec3(sample.night));
}

struct EnvironmentReflection {
    radiance: vec3<f32>,
    split_sum: vec2<f32>,
}
fn environment_reflection(direction: vec3<f32>, roughness: f32, view_cosine: f32) -> EnvironmentReflection {
    let r=clamp(roughness,0.045,1.0);
    let unit=(r-0.045)/(1.0-0.045);
    let size=vec2<f32>(textureDimensions(sky_dfg));
    let uv=(vec2(clamp(view_cosine,0.0,1.0),unit)*(size-1.0)+vec2(0.5))/size;
    let pair=textureSampleLevel(sky_dfg,sky_reflection_sampler,uv,0.0).rg;
    let encoded=textureSampleLevel(sky_reflection,sky_reflection_sampler,normalize(direction),f32(textureNumLevels(sky_reflection)-1u)*unit);
    let radiance=decode_sky_light(encoded,atmosphere.sun_lux.w*atmosphere.colour_pre_exposure.rgb);
    return EnvironmentReflection(radiance,pair);
}
