struct ExposureSample {
    // Storage p, inverse camera gain, resolved camera EV100, actual incident lux.
    values: vec4<f32>,
    // Flags, current submitted-candidate serial lo/hi, display selectors.
    metadata: vec4<u32>,
}
