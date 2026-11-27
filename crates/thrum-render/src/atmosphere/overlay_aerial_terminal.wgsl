fn environment_far_sky(direction: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let size = vec2<f32>(textureDimensions(environment_sky));
    let sky_coordinate = clamp(sky_uv(normalize(direction), atmosphere.sun_lux.xyz), 0.5/size, 1.0-0.5/size);
    let encoded = textureSampleLevel(environment_sky, aerial_sampler, sky_coordinate, 0.0);
    let night = vec3(encoded.a / NIGHT_LUT_GAIN);
    let solar = atmosphere.sun_lux.w * atmosphere.colour_pre_exposure.rgb;
    return encoded.rgb / SKY_LUT_GAIN * solar * chart_haze_scale() + night;
}
