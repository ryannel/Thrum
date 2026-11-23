fn erosion_mip(footprint: f32, metres_per_texel: f32) -> f32 {
    return max(0.0, log2(max(1.0, footprint / metres_per_texel)));
}

fn cloud_erode(sigma: f32, extinction: f32, strength: f32, billow: f32, detail: f32) -> f32 {
    let signal = min(1.0, sigma / extinction);
    let threshold = strength * (1.0 - billow);
    // Shape erosion sweeps through the profile. Detail acts on the
    // surviving edge, with the core and the uneroded bound intact.
    let edge = max(0.0001, min(0.12, 1.0 - threshold));
    let shape = clamp((signal - threshold) / edge, 0.0, 1.0);
    return sigma * max(0.0, shape - strength * detail * (1.0 - shape));
}

// Authored slabs already contain their billows. Use the shared noise to
// break up the soft boundary without cutting a second, thin shell into it.
fn placed_erode(sigma: f32, extinction: f32, strength: f32, billow: f32, detail: f32) -> f32 {
    let signal = min(1.0, sigma / extinction);
    let threshold = 0.45 * strength * (1.0 - billow);
    let edge = smoothstep(threshold, threshold + 0.28, signal);
    return sigma * edge * (1.0 - 0.20 * strength * detail * (1.0 - edge));
}
