@group(0) @binding(0) var scene: texture_2d<f32>;

@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4(positions[index], 0.0, 1.0);
}

@fragment fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let linear = textureLoad(scene, vec2<i32>(position.xy), 0).rgb;
    return vec4(srgb_encode(clamp(linear, vec3(0.0), vec3(1.0))), 1.0);
}

// Current display headroom and the greatest stored half-float at or below it.
@group(1) @binding(0) var<uniform> edr_headroom: vec2<f32>;

@fragment fn float_fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let linear = textureLoad(scene, vec2<i32>(position.xy), 0).rgb;
    return vec4(min(linear, vec3(edr_headroom.y)), 1.0);
}
