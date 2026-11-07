const HORIZON_STEPS: u32 = 96u;

// Terrain-scale Sun visibility beyond the cascades: march the highest caster
// surface toward the Sun's azimuth and compare the steepest horizon with the
// solar disk. Steps grow geometrically from two texels to the map edge.
fn horizon_visibility(receiver: ShadowReceiver) -> f32 {
    let horizon = shadows.horizon;
    let l = light.direction_to_light.xyz;
    let flat = length(l.xz);
    if horizon.x <= 0.0 || flat < 1e-4 { return 1.0; }
    let azimuth = l.xz/flat;
    let sun_elevation = atan2(l.y,flat);
    let solar_radius = atan(SUN_TAN_RADIUS);
    // Horizons below the lower limb leave the disk whole; above the upper limb
    // they hide it. Both bounds end the march early without changing the result.
    let lower_limb = tan(sun_elevation-solar_radius);
    let upper_limb = tan(min(sun_elevation+solar_radius,1.5));
    let rise = shadows.height_support.x-receiver.position.y;
    let texel = horizon.w;
    let start = 2.0*texel;
    if lower_limb >= 0.0 && rise <= lower_limb*start { return 1.0; }
    let reach = 1.0/max(horizon.z,1e-20);
    let ratio = pow(reach/start,1.0/f32(HORIZON_STEPS-1u));
    // Nearest-texel heights can stand half a texel's rise above the true
    // surface on a slope; allow that much before a sample counts as horizon.
    let extent = vec2<f32>(textureDimensions(shadow_maps));
    var steepest = -1e9;
    var distance = start;
    for (var i = 0u; i < HORIZON_STEPS; i++) {
        // A nonnegative slope threshold rises with distance. A negative one falls,
        // so even lower terrain can still rise above it farther along the ray.
        let threshold = max(lower_limb,steepest);
        if steepest >= upper_limb || (threshold >= 0.0 && rise <= threshold*distance) { break; }
        let q = receiver.position.xz+azimuth*distance;
        let clip = shadows.height_view_proj*vec4(q.x,0.0,q.y,1.0);
        let uv = vec2(clip.x*0.5+0.5,0.5-clip.y*0.5);
        if any(uv<vec2(0.0)) || any(uv>vec2(1.0)) { break; }
        let texel_index = clamp(vec2<i32>(uv*extent),vec2(0),vec2<i32>(extent)-1);
        let depth = textureLoad(shadow_maps,texel_index,2,0);
        if depth > 0.0 {
            let height = (depth-horizon.y)/horizon.x;
            steepest = max(steepest,(height-receiver.position.y-0.5*texel*0.25)/distance);
        }
        distance *= ratio;
    }
    if steepest <= -1e8 { return 1.0; }
    return disk_above((sun_elevation-atan(steepest))/solar_radius);
}
