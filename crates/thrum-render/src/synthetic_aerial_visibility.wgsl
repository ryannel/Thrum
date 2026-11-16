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

