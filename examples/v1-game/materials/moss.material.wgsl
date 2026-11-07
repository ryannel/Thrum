// A material hook for the cooked valley's meshes: moss where they face the sky. Serve the page
// with ?assets=<dev server>&material=moss.material.wgsl, edit this file in the web root and
// choose Reload material. A valid hook draws from the next frame once it compiles; a mistake is
// reported with its line here and the previous material keeps drawing.
//
// `surface` holds the factors the placement sets; return the factors to shade with.
//   surface.base_colour  linear RGBA, multiplies the base-colour texture
//   surface.roughness    0 (mirror) to 1 (matte)
//   surface.metalness    0 (stone) to 1 (metal)
//   surface.emissive     added radiance, linear RGB
// `at` says where: at.uv, at.world (metres, +Y up) and at.normal (unit, world space).
fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    let up = smoothstep(0.55, 0.85, at.normal.y);
    out.base_colour = vec4(mix(surface.base_colour.rgb, vec3(0.10, 0.22, 0.05), up), surface.base_colour.a);
    out.roughness = mix(surface.roughness, 1.0, up);
    return out;
}
