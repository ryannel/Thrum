fn srgb_encode(linear: vec3<f32>) -> vec3<f32> {
    return select(SRGB_SCALE*pow(linear,vec3(SRGB_ENCODE_EXPONENT))-vec3(SRGB_OFFSET),SRGB_SLOPE*linear,linear<=vec3(0.0031308));
}
