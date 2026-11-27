// The column is read beneath the receiver. The ladder is read at the
// corresponding point on the surface along the mean refracted Sun ray.
fn sun_path(point: vec3<f32>, toward_sun: vec3<f32>) -> vec4<f32> {
    let window = sun_path_mapping.window;
    if window.w == 0.0 { return vec4(point, 1.0); }
    let column = caustic_column(point.xz, window, sun_path_columns);
    let depth = column.x - point.y;
    if column.w == 0.0 || depth <= 0.0 { return vec4(point, 1.0); }

    let exit = water_sun_exit(point, column.x, toward_sun);
    let focused = caustic_field_at(
        exit.xz, depth, 0.0, window, sun_path_field, sun_path_sampler, sun_path_mapping.quality.y,
    );
    return vec4(exit, 1.0 + exp(-column.y * depth) * (focused - 1.0));
}
