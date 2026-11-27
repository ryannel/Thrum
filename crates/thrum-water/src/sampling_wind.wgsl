fn sample_scaled_field(t:texture_2d_array<f32>,s:sampler,uv:vec2<f32>,layer:i32,dx:vec2<f32>,dy:vec2<f32>,footprint_scale:f32)->vec4<f32> {
    let wind = textureNumLayers(t) == RAIN_SHARED_LAYERS && layer < i32(RAIN_FIRST_LAYER);
