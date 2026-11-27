
@group(0) @binding(26) var wetness_field: texture_2d<f32>;
@group(0) @binding(0) var<storage, read_write> result: array<vec4<f32>>;

@compute @workgroup_size(1)
fn probe_wet_ground() {
    let points = array(
        vec3(145.0, 18.118, 65.0),
        vec3(145.25, 18.120, 65.25),
        vec3(152.15, 18.133, 68.025),
        vec3(145.546875, 18.118, 65.25),
        vec3(145.5, 18.120, 65.25),
    );
    for (var i = 0u; i < 5u; i++) {
        let wet = ground_wetness(points[i], vec3(0.0, 1.0, 0.0));
        result[i] = vec4(wet.pores, wet.surface, wet.cover, f32(wet.class_index));
    }
}
