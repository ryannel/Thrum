// Terrain-shadowed aerial perspective. The Sun shadow-height map holds, per
// column, the height below which caster terrain hides the Sun's centre (R) and
// the distance to that caster (G). Only the Sun's single-scattered part of each
// ray interval is attenuated; multiple scattering stays (sky light still reaches
// shadowed air).
@group(0) @binding(4) var air_shadow: texture_2d<f32>;

const AIR_INTERVALS: u32 = 8u;

fn air_sun_visibility(point: vec3<f32>) -> f32 {
    let l = light.direction_to_light.xyz;
    let flat = length(l.xz);
    if shadows.horizon.x <= 0.0 || flat < 1e-4 { return 1.0; }
    let clip = shadows.height_view_proj * vec4(point.x, 0.0, point.z, 1.0);
    let uv = vec2(clip.x * 0.5 + 0.5, 0.5 - clip.y * 0.5);
    if any(uv < vec2(0.0)) || any(uv > vec2(1.0)) { return 1.0; }
    let extent = vec2<f32>(textureDimensions(air_shadow));
    let texel = clamp(vec2<i32>(uv * extent), vec2(0), vec2<i32>(extent) - 1);
    let column = textureLoad(air_shadow, texel, 0).rg;
    if column.x <= -1e29 { return 1.0; }
    // Angular margin of the Sun's centre over the caster, in solar radii. For
    // small angles e − θ ≈ (y − H)·cos²e / D, and cos²e is the light's flat² term.
    let distance = max(column.y, shadows.horizon.w);
    let margin = (point.y - column.x) * flat * flat / (distance * SUN_TAN_RADIUS);
    return disk_above(margin);
}

fn aerial_colour(colour: vec3<f32>, world_position: vec3<f32>, screen_position: vec4<f32>) -> vec3<f32> {
    let camera = aerial_frame.camera_altitude.xyz;
    let offset = world_position - camera;
    let distance = min(32000.0, length(offset));
    let direction = offset / max(length(offset), 1e-6);
    let uv = aerial_uv(screen_position);
    let end = aerial_sample(distance, uv);
    // Integrate the interval contributions in sqrt-distance, matching the
    // volume's layer spacing, and attenuate each one's single-scattered part.
    var scatter = vec3(0.0);
    var previous = vec4(0.0);
    var previous_distance = 0.0;
    for (var i = 1u; i <= AIR_INTERVALS; i++) {
        let fraction = f32(i) / f32(AIR_INTERVALS);
        let interval_end = distance * fraction * fraction;
        let current = select(aerial_sample(interval_end, uv).scatter, end.scatter, i == AIR_INTERVALS);
        // Unclamped, so fully lit intervals telescope to the unshadowed result.
        let delta = current - previous;
        let luminance = dot(delta.rgb, vec3(0.2126, 0.7152, 0.0722));
        let single = clamp(delta.a / max(luminance, 1e-20), 0.0, 1.0);
        let middle = camera + direction * (0.5 * (previous_distance + interval_end));
        let visibility = air_sun_visibility(middle);
        scatter += delta.rgb * (1.0 - single * (1.0 - visibility));
        previous = current;
        previous_distance = interval_end;
    }
    return composite_aerial(end, colour, scatter);
}
