// The lighthouse material hook: red and white bands up the tower, a glowing lantern room and a
// dark cap. It runs for every cooked mesh, so the rocks (away from the tower's axis) keep the
// factors their placements set. Edit it and press L in the window.
fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    let radius = length(at.world.xz);
    if (radius > 1.8) {
        return out;
    }
    let y = at.world.y;
    if (y < 12.0) {
        // Three red bands on white, softened at their edges.
        let band = smoothstep(0.45, 0.55, abs(fract(y / 4.0) - 0.5) * 2.0);
        out.base_colour = vec4(mix(vec3(0.85, 0.84, 0.80), vec3(0.55, 0.04, 0.03), band), 1.0);
        out.roughness = 0.55;
    } else if (y < 12.3) {
        out.base_colour = vec4(0.05, 0.05, 0.05, 1.0);
        out.metalness = 0.9;
        out.roughness = 0.4;
    } else if (y < 13.9) {
        // The lamp: warm light from the lantern room.
        out.base_colour = vec4(0.9, 0.8, 0.5, 1.0);
        out.emissive = vec3(60.0, 45.0, 20.0);
        out.roughness = 0.1;
    } else {
        out.base_colour = vec4(0.25, 0.04, 0.03, 1.0);
        out.metalness = 0.6;
        out.roughness = 0.35;
    }
    return out;
}
