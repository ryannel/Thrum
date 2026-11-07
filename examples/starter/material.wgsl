// The rock's material hook. Edit it and press L in the window: a valid hook draws from the next
// frame; a mistake is reported with its line here and the previous material keeps drawing.
//
// `surface` holds the factors the rock's placement sets; return the factors to shade with.
//   surface.base_colour  linear RGBA, multiplies the base-colour texture
//   surface.roughness    0 (mirror) to 1 (matte)
//   surface.metalness    0 (stone) to 1 (metal)
//   surface.emissive     added radiance, linear RGB
// `at` says where: at.uv, at.world (metres, +Y up) and at.normal (unit, world space).
//
// Try: moss where the rock faces the sky.
//   let up = smoothstep(0.55, 0.85, at.normal.y);
//   out.base_colour = vec4(mix(surface.base_colour.rgb, vec3(0.10, 0.22, 0.05), up), 1.0);
//   out.roughness = mix(surface.roughness, 1.0, up);
fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    return out;
}
