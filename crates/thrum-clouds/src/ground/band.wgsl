@group(0) @binding(3) var band_out: texture_storage_3d<rgba16float, write>;
@group(0) @binding(4) var<uniform> band_window: GroundBandWindow;

@compute @workgroup_size(8, 8, 1)
fn fill_ground_band(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(band_out);
    if any(id >= size) { return; }
    let xz = mix(band_window.region.xy, band_window.region.zw, vec2<f32>(id.xy) / vec2<f32>(size.xy - 1u));
    let terrain = ground_surface(xz).x;
    let point = vec3(xz.x, terrain + band_window.top_border.x * f32(id.z) / f32(size.z - 1u), xz.y);
    let direction = mapping.path_border.xyz;

    // Condensate above the ground band already has cumulative Sun depth.
    // Sample that column once. Ground extinction is the only new integral.
    let cloud = cloud_window_depths(point, point.y, ground_frame.upper_heights).x;
    var tau = cloud;
    var visibility = 1.0;
    let fog_end = max(0.0, (ground_frame.ground_heights.y - point.y) / direction.y);
    var distance = 0.0;
    // Use short steps close to the receiver and larger footprints as the ray
    // recedes. Surface occlusion issues no density read beyond the fog.
    let max_distance = max(fog_end, max(0.0, (ground_frame.ground_heights.z - point.y) / direction.y));
    for (var i = 0u; i < 96u && distance < max_distance; i++) {
        let end = min(max_distance, distance + max(5.0, 0.1 * distance));
        let fog_step = min(end, fog_end) - distance;
        let at = distance + 0.5 * select(end - distance, fog_step, fog_step > 0.0);
        let sample = point + direction * at;
        let surface = ground_surface(sample.xz);
        if sample.y < surface.x { visibility = 0.0; }
        if fog_step > 0.0 && ground_frame.has_active_rows != 0u {
            tau += fog_step * ground_extinction(sample, surface, true);
        }
        if visibility == 0.0 && end >= fog_end { break; }
        distance = end;
    }
    textureStore(band_out, vec3<i32>(id), vec4(min(65000.0, tau), terrain, visibility, 0.0));
}
