// The column is read beneath the receiver. The ladder is read at the
// corresponding point on the surface along the mean refracted Sun ray.
fn sun_path(point: vec3<f32>, toward_sun: vec3<f32>) -> SunPathSample {
    let slope = water_mean_sun_slope(toward_sun);
    let ray_plane = point.xz + point.y * slope;
    let footprint = max(length(dpdx(ray_plane)), length(dpdy(ray_plane)));
    let window = sun_path_mapping.window;
    if window.w == 0.0 { return SunPathSample(point, 1.0, false); }
    let outside = sun_path_mapping.outside_column;
    let column = caustic_column(point.xz, sun_path_mapping.column_bounds, outside, sun_path_columns);
    let depth = column.x - point.y;
    if column.w == 0.0 || depth <= 0.0 { return SunPathSample(point, 1.0, false); }

    let exit = water_sun_exit(point, column.x, toward_sun);
    let focused = caustic_field_at(
        exit.xz, depth, footprint, window, sun_path_field, sun_path_sampler, sun_path_mapping.quality.y,
    );
    let border = caustic_border(point.xz, window);
    let beam = water_sun_beam(toward_sun, vec3(0.0, 1.0, 0.0));
    let rate = outside.y / max(-beam.y, 0.05);
    return SunPathSample(exit, 1.0 + border * exp(-rate * depth) * (focused - 1.0), true);
}
