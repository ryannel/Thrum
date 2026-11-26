@compute @workgroup_size(8, 8, 1)
fn ground_medium(@builtin(global_invocation_id) cell: vec3<u32>) {
    let size = textureDimensions(medium_out);
    if any(cell.xy >= size) { return; }
    let sample = medium_sample_point(cell);
    let near = ground_near_weight(sample.w);
    var value = vec3(0.0);
    if near > 0.0 && (ground_frame.has_active_rows != 0u || cloud_deck_active()) {
        let ray = normalize(sample.xyz - medium_observer.camera_altitude.xyz);
        let sun = medium_observer.sun_lux.xyz;
        let mist = ground_light(sample.xyz, ray, sun, sample.w, 1.0, true);
        var deck = GroundLight(0.0, 0.0, vec3(0.0), 1.0);
        if cloud_deck_active() {
            deck = cloud_deck_light(sample.xyz, ray, sun, sample.w, 1.0);
        }
        let ambient = dot(mist.ambient + deck.ambient, vec3(0.2126, 0.7152, 0.0722));
        value = near * vec3(mist.extinction + deck.extinction, mist.solar_kernel + deck.solar_kernel, ambient);
    }
    textureStore(medium_out, vec2<i32>(cell.xy), i32(cell.z), vec4(value, 0.0));
}
