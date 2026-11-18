fn srgb_encode(linear: vec3<f32>) -> vec3<f32> {
    return select(1.055*pow(linear,vec3(1.0/2.4))-vec3(0.055),12.92*linear,linear<=vec3(0.0031308));
}
