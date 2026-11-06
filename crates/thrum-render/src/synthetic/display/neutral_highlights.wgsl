// SPDX-License-Identifier: Apache-2.0
// Copyright 2024 The Khronos Group, Inc.
//
// Adapted from PBR_Neutral/pbrNeutral.glsl in KhronosGroup/ToneMapping:
// https://github.com/KhronosGroup/ToneMapping/blob/180b1a7bddec33f73fe41712a2963cc3ad8e5547/PBR_Neutral/pbrNeutral.glsl
// Source SHA-256: 087a1542c8d12d2d0fb89f570057e4e8365cc04eee5bff76c6e365cfa69f3734
// Per-file licence: .reuse/dep5 at the same revision, retrieved 2026-11-06.
// The Apache-2.0 licence is retained in LICENSE-APACHE-2.0.txt beside this file.
//
// Changes: retain only highlight compression/desaturation; remove the material
// shadow/Fresnel offset; translate to WGSL and reciprocal-peak arithmetic to
// avoid exposed RGB overflow. This is not the standard PBR Neutral operator.
// Input: nonnegative finite pre-exposed Rec.709 RGB, A=p*inverse_camera_gain.
// Output: display-linear Rec.709 RGB relative to SDR display white, in [0,1].
fn neutral_highlights(stored: vec3<f32>, storage_inverse_gain: f32) -> vec3<f32> {
    let maximum = max(stored.r, max(stored.g, stored.b));
    if maximum == 0.0 {
        return vec3(0.0);
    }
    // Branch before dividing A by a possibly tiny maximum. On this branch
    // stored/A is bounded by the knee; the shoulder's q is then bounded too.
    if storage_inverse_gain >= maximum / 0.76 {
        return stored / storage_inverse_gain;
    }
    let chromaticity = stored / maximum;
    let q = storage_inverse_gain / maximum;
    let peak = 1.0 - (0.24 * 0.24) * q / (1.0 - 0.52 * q);
    let retained_colour = q / (0.15 * (1.0 - peak * q) + q);
    return mix(vec3(peak), chromaticity * peak, retained_colour);
}
