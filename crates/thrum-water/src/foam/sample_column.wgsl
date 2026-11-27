fn column_component(index: u32) -> Component { return foam_settings.components[index]; }

fn foam_column(point: vec2<f32>, gains: vec4<f32>, dt: f32, current: vec2<f32>, track: vec4<f32>) -> Wave {
    var field = long_wave(point - foam_settings.camera.xy, dt, gains.x,
        foam_settings.camera.w, foam_settings.banks.w);
    let slope = world_slope(field);
    field.velocity.y -= dot(slope, field.velocity.xz);
    field.slope = slope;
    let motion = cascade_motion(point, current, track, water.carriage.w, water.carriage.y);
    for (var band = 0u; band < 3u; band++) {
        let gain = gains[band + 1u];
        if gain * water.heights[band + 1u] <= 0.0 { continue; }
        let carried = carried_column(motion, water.lengths[band], band, 0.0, CarriageGradient(COLUMN_IDENTITY, COLUMN_IDENTITY, vec2(0.0)));
        let sample = carried.wave;
        let band_slope = cascade_world_slope(sample.slope, carried.differential, gain);
        let velocity = gain * (sample.velocity.y + carried.advection_rate);
        field.displacement.y += gain * sample.displacement.y + dt * velocity;
        field.slope += band_slope;
        field.velocity.y += velocity;
    }
    return field;
}
