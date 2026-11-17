fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    let y = at.world.y / 0.51;
    if distance(at.world.xz, vec2(0.0, -16.1)) < 0.91 {
        let band = smoothstep(0.45, 0.55, abs(fract(y / 4.0) - 0.5) * 2.0);
        let plaster = mix(vec3(0.72, 0.69, 0.60), vec3(0.40, 0.045, 0.025), band);
        out.base_colour = vec4(plaster, 1.0);
        out.roughness = 0.66;
        if y > 12.0 { out.base_colour = vec4(0.08, 0.065, 0.045, 1.0); }
        if y > 12.25 && y < 13.9 { out.emissive = vec3(34.0, 18.0, 5.0); }
    }
    if distance(at.world.xz, vec2(-3.3, -16.7)) < 0.35 && at.world.y > 1.85 && at.world.y < 2.3 {
        out.emissive = vec3(90.0, 45.0, 15.0);
    }
    return out;
}
