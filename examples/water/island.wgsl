fn gravel_hash(point: vec2<f32>) -> f32 {
    let q = fract(vec3(point.xyx) * vec3(0.1031, 0.11369, 0.13787));
    let p = q + dot(q, q.yzx + 19.19);
    return fract((p.x + p.y) * p.z);
}

fn ground_noise(point: vec2<f32>) -> f32 {
    let i = floor(point);
    let f = smoothstep(vec2(0.0), vec2(1.0), fract(point));
    let n = mix(
        mix(gravel_hash(i), gravel_hash(i + vec2(1.0, 0.0)), f.x),
        mix(gravel_hash(i + vec2(0.0, 1.0)), gravel_hash(i + vec2(1.0, 1.0)), f.x),
        f.y,
    );
    // Below the pixel footprint the texture tends to its mean.
    return mix(n, 0.5, smoothstep(0.3, 1.7, max(length(dpdx(point)), length(dpdy(point)))));
}

fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    let xz = at.world.xz;
    let height = at.world.y - ISLAND_DATUM;
    let low = ground_noise(xz * 0.07);
    let cover = ground_noise(xz * 0.31 + vec2(low * 2.0)) * 0.5 + low * 0.5;
    let coarse = ground_noise(xz * vec2(2.7, 3.2));
    let fine = ground_noise(xz * 32.0);
    let ripples = sin(xz.x * 24.0 + 7.0 * low + ground_noise(xz * 1.8));
    let ripple_fade = 1.0 - smoothstep(0.014, 0.12, length(fwidth(xz)));
    // Fragment derivatives above need uniform control flow across the quad.
    if any(surface.base_colour.rgb < vec3(0.99)) { return surface; }
    var out = surface;
    let beach = vec3(0.43, 0.34, 0.20) * (0.75 + 0.24 * cover + 0.2 * coarse + 0.22 * fine + 0.05 * ripples * ripple_fade);
    let grass = mix(vec3(0.063, 0.105, 0.029), vec3(0.21, 0.22, 0.082), smoothstep(0.11, 0.84, cover))
        * (0.79 + 0.34 * coarse + 0.20 * fine);
    let stone = vec3(0.31, 0.25, 0.18) * (0.62 + 0.27 * cover + 0.29 * coarse + 0.23 * fine);
    let lake = 8.2 + 0.12 * length((xz - vec2(183.0, 5.0)) * vec2(1.0, 1.16));
    let pond = 8.0 + 0.24 * length((xz - vec2(104.0, 93.0)) * vec2(1.0, 1.13));
    let gravel = 1.0 - smoothstep(0.35, 1.55 + coarse * 0.7, abs(height - select(10.6, 12.0, lake < pond)));
    let upland = smoothstep(18.0, 33.0, height + 10.0 * cover);
    let terrain = mix(beach, mix(grass, stone, max(gravel * 0.85, upland)),
        smoothstep(1.3, 4.8, height + 1.3 * cover));
    // The sheltered pond's silt floor reflects the narrow light ridges.
    // Blend it into the grassy bank across the surveyed pond's shoreline.
    let pond_floor = (1.0 - smoothstep(10.4, 10.9, height))
        * (1.0 - smoothstep(14.0, 20.0, length((xz - vec2(104.0, 93.0)) * vec2(1.0, 1.13))));
    let silt = mix(beach, vec3(0.64, 0.56, 0.41), 0.65);
    out.base_colour = vec4(mix(terrain, silt, pond_floor), 1.0);
    out.roughness = 0.9;
    return out;
}
