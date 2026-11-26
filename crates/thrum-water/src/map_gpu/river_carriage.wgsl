fn mix_cascade_motion(a: CascadeMotion, b: CascadeMotion, weight: f32) -> CascadeMotion {
    return CascadeMotion(
        mix(a.departure, b.departure, weight),
        mix(a.second, b.second, weight),
        mix(a.drift, b.drift, weight),
        mix(a.blend, b.blend, weight),
        mix(a.blend_rate, b.blend_rate, weight),
        mix(a.second_drift, b.second_drift, weight),
    );
}
