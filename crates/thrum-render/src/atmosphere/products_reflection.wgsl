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
