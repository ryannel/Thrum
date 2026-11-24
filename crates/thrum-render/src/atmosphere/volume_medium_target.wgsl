@group(0) @binding(0) var<uniform> medium_observer: Atmosphere;
@group(0) @binding(1) var<uniform> medium_camera: VolumeHistory;
@group(0) @binding(2) var medium_out: texture_storage_2d_array<rgba16float, write>;

fn medium_sample_point(cell: vec3<u32>) -> vec4<f32> {
    let size = vec2<f32>(textureDimensions(medium_out));
    let uv = (vec2<f32>(cell.xy) + 0.5) / size;
    let ndc = vec2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let far = medium_observer.inverse_view_projection * vec4(ndc, 0.0, 1.0);
    let eye = medium_observer.camera_altitude.xyz;
    let direction = normalize(far.xyz / far.w - eye);
    let start = volume_distance(cell.z);
    let end = volume_distance(cell.z + 1u);
    let jitter = volume_shadow_jitter(cell.xy, cell.z, medium_camera.current_view_forward_phase.w);
    let distance = mix(start, end, jitter);
    return vec4(eye + direction * distance, distance);
}
