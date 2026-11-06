@group(1) @binding(8) var sky_reflection: texture_cube<f32>;
@group(1) @binding(9) var sky_dfg: texture_2d<f32>;
@group(1) @binding(10) var sky_reflection_sampler: sampler;

fn indirect_radiance(base: vec3<f32>, metalness: f32, roughness: f32,
                     n: vec3<f32>, v: vec3<f32>, ao: f32) -> vec3<f32> {
    let nov=dot(n,v);
    // Match the direct BRDF's outward hemisphere. Negative view cosines are
    // not a grazing endpoint, including two-sided raster fallback fragments.
    if nov<=0.0 { return vec3(0.0); }
    let r=clamp(roughness,GGX_MIN_ROUGHNESS,1.0);
    let roughness_unit=(r-GGX_MIN_ROUGHNESS)/(1.0-GGX_MIN_ROUGHNESS);
    // The DFG includes both NoV and roughness endpoints at texel centres.
    let size=vec2<f32>(textureDimensions(sky_dfg));
    let uv=(vec2(min(nov,1.0),roughness_unit)*(size-1.0)+vec2(0.5))/size;
    let dfg=textureSampleLevel(sky_dfg,sky_reflection_sampler,uv,0.0).rg;
    let f0=mix(vec3(0.04),base,metalness);
    let ess=f0*dfg.x+vec3(dfg.y);
    // Cube storage is per-unit, disk-free Sky radiance. Use the unattenuated
    // source here, not the directly transmitted directional-light irradiance.
    let environment=textureSampleLevel(sky_reflection,sky_reflection_sampler,
        reflect(-v,n),f32(textureNumLevels(sky_reflection)-1u)*roughness_unit).rgb/1024.0
        *aerial_frame.sun_lux.w*aerial_frame.colour_pre_exposure.rgb;
    let diffuse=ao*(1.0-metalness)*base*(vec3(1.0)-ess)*ambient_irradiance(n)/3.14159265359;
    // Single-scatter GGX energy loss is retained; aerial and pre-exposure are
    // applied once by the caller after direct, indirect and emissive combine.
    return diffuse+ao*environment*ess;
}
