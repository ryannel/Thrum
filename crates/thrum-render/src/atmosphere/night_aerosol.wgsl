// Reweight the prepared first-scatter source for the live atmosphere. The
// fixed chart combines a shell and diffuse incident field in one channel.
// Represent its incoming column by one slant path. The view-ray attenuation
// below uses the full spherical column and the selected wavelength.
fn aerosol_night_source(h: f32, mu: f32, linear: sampler) -> f32 {
    let base = night_source(h, mu, linear);
    if aerosol.surface_extinction.w == MIE_EXTINCTION &&
        aerosol.extinction[1].x == MIE_EXTINCTION { return base; }
    let density = medium_densities(h);
    let rayleigh = RAYLEIGH_BETA[1].x * density.x;
    let phase = scattering_phase(mu * 0.5);
    let old_scatter = dot(vec2(rayleigh, MIE_SCATTERING * density.y), phase);
    let new_scatter = dot(vec2(rayleigh, aerosol.scattering[1].x * density.y), phase);
    let column = 2.0 * MIE_SCALE_HEIGHT * density.y;
    let shift = (aerosol.extinction[1].x - MIE_EXTINCTION) * column;
    return base * (new_scatter / max(old_scatter, 1e-20)) * exp(-shift);
}

// The fixed terminal chart already holds the sum of first scatter and a
// boundary. Reintegrate the scattered part with live aerosol and transport
// the residual boundary by the change in optical depth. A default sample
// returns the original chart value directly.
fn aerosol_night_terminal(h: f32, mu: f32, tangent: f32, ground: bool, linear: sampler) -> f32 {
    let base = night_terminal(h, mu, tangent, ground, linear);
    let difference = aerosol.extinction[1].x - MIE_EXTINCTION;
    if night_control.x == 0u || difference == 0.0 { return base; }
    let b = (R + h) * mu;
    let c = (95000.0 - h) * (2.0 * R + 95000.0 + h);
    let q = sqrt(max(0.0, b * b + c));
    let ground_column = h * (2.0 * R + h);
    let ground_end = ground_column / max(0.001, -b + sqrt(max(0.0, b * b - ground_column)));
    let end = select(select(q - b, c / max(0.001, q + b), mu >= 0.0), ground_end, ground);
    let direction = vec3(sqrt(max(0.0, 1.0 - mu * mu)), mu, 0.0);
    var original_depth = 0.0;
    var change = 0.0;
    var original_scatter = 0.0;
    var current_scatter = 0.0;
    for (var step = 0u; step < 64u; step++) {
        let a = f32(step) / 64.0;
        let b = f32(step + 1u) / 64.0;
        let dt = (b * b - a * a) * end;
        let point = vec3(0.0, h, 0.0) + direction * (0.5 * (a * a + b * b) * end);
        let height = altitude(point);
        let density = medium_densities(height);
        let local_mu = local_sun_mu(point, direction);
        let original = (RAYLEIGH_BETA[1].x * density.x + MIE_EXTINCTION * density.y +
            OZONE_BETA[1].x * density.z) * dt;
        let delta = difference * density.y * dt;
        original_scatter += exp(-original_depth - 0.5 * original) * night_source(height, local_mu, linear) * dt;
        current_scatter += exp(-max(0.0, original_depth + change + 0.5 * (original + delta))) *
            aerosol_night_source(height, local_mu, linear) * dt;
        original_depth += original;
        change += delta;
    }
    return current_scatter + max(0.0, base - original_scatter) * exp(clamp(-change, -80.0, 30.0));
}
