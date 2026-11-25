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

fn cloud_bank_weight(deck: f32) -> f32 {
    return smoothstep(0.12, 0.45, deck) * (1.0 - smoothstep(0.73, 0.98, deck));
}

fn cloud_bank_billow(shape_noise: texture_3d<f32>, noise_sampler: sampler,
                     shape_uv: vec3<f32>, footprint: f32) -> f32 {
    return textureSampleLevel(shape_noise, noise_sampler, shape_uv * vec3(2.0, 3.0, 2.0),
                              erosion_mip(2.0 * footprint, 50.0)).r;
}

// Broad shaping of the ceiling, underside and gaps inside the weather
// column. Both view and Sun rays use the same advected shape-noise octaves.
fn cloud_bank_shape(shape_noise: texture_3d<f32>, noise_sampler: sampler,
                    point: vec3<f32>, shape_uv: vec3<f32>, bank_noise: vec4<f32>,
                    footprint: f32, height: f32, weight: f32) -> f32 {
    if weight == 0.0 { return 1.0; }
    let air = point.xz / bank_noise.z + bank_noise.xy;
    let swell = textureSampleLevel(shape_noise, noise_sampler, vec3(air.x, 0.31, air.y),
                                  erosion_mip(footprint, 175.0)).r;
    let bank_height = smoothstep(0.32, 0.68, swell);
    let billow = cloud_bank_billow(shape_noise, noise_sampler, shape_uv, footprint);
    let bottom = 0.05 + 0.59 * bank_height + 0.08 * billow;
    let top = 0.55 + 0.45 * bank_height;
    let interval = smoothstep(0.78 * bottom, bottom + 0.12, height)
        * (1.0 - smoothstep(top - 0.18, top, height));
    let separation = smoothstep(0.20, 0.48, swell + 0.16 * billow);
    return mix(1.0, interval * mix(0.28, 1.0, separation), weight);
}

// Authored slabs already contain their billows. Use the shared noise to
// break up the soft boundary without cutting a second, thin shell into it.
fn placed_erode(sigma: f32, extinction: f32, strength: f32, billow: f32, detail: f32) -> f32 {
    let signal = min(1.0, sigma / extinction);
    let threshold = 0.45 * strength * (1.0 - billow);
    let edge = smoothstep(threshold, threshold + 0.28, signal);
    return sigma * edge * (1.0 - 0.20 * strength * detail * (1.0 - edge));
}
