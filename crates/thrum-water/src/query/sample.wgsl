
fn plain_query(point: QueryPoint, gain: f32) -> QueryWave {
    return QueryWave(long_wave(point.position - settings.origin, point.dt, gain,
        settings.lengths_chop.w, settings.components), 0u, 0u);
}

fn write_answer(index: u32, point: QueryPoint, gains: vec4<f32>, result: QueryWave) {
    let row = point.level_current_bed;
    var out: Answer;
    out.body_flags = vec4(point.body, 0u, point.flags & 1u, 0u);
    out.current_bed = vec4(row.yzw, 0.0);
    // A stage change can make a dry or sheltered point gain wave support
    // before its use tick. Keep the actual frame samples for that case.
    let carriage = query_carriage(index, point);
    var bands: array<CarriedColumn, 3>;
    for (var band = 0u; band < 3u; band++) {
        if settings.heights[band + 1u] > 0.0 {
            let carried = cascade_wave(point, carriage.motion, band, carriage.gradient);
            let wave = carried.wave;
            bands[band] = carried;
            out.advection_rate[band] = carried.advection_rate;
            out.cascades[band] = CascadeSample(
                wave.displacement.y,
                array(wave.slope.x, wave.slope.y),
                array(carried.differential.x, carried.differential.y, carried.differential.z, carried.differential.w),
                array(wave.velocity.x, wave.velocity.y, wave.velocity.z),
            );
        }
    }
