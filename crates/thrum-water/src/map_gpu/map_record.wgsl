struct MapRecord {
    origin_spacing_margin: vec4<f32>,
    size_count_sea: vec4<u32>,
    swash: vec4<f32>,
    bodies: array<BodyRecord, BODY_LIMIT>,
}
