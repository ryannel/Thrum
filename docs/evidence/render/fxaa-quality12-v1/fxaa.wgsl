// Diagnostic WGSL port of NVIDIA FXAA 3.11 Quality preset 12.
// Reference: GameTechDev/CMAA2 commit 071c6b0857559f4e36f614362e6d2aab1b61938a,
// Projects/CMAA2/FXAA/Fxaa3_11.h, SHA-256 4834fada78774fc55726adeb717c53ca7324b66820df08ef773515cafb148aef.
// Copyright (c) 2014 NVIDIA CORPORATION. All rights reserved.
// The reference's three-clause redistribution notice and disclaimer must accompany
// any production derivative; see docs/evidence/render/FXAA-SOURCE-SELECTION-2026-11-02.md.

@group(0) @binding(0) var encoded_scene: texture_2d<f32>;
@group(0) @binding(1) var encoded_sampler: sampler;

fn sample_at(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(encoded_scene, encoded_sampler, uv, 0.0);
}

fn luma_at(uv: vec2<f32>) -> f32 {
    return sample_at(uv).a;
}

@vertex fn vs_fxaa(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(corners[index], 0.0, 1.0);
}

@fragment fn fs_fxaa(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(encoded_scene));
    let texel = 1.0 / size;
    let uv = position.xy * texel;
    let center = sample_at(uv);
    let m = center.a;
    var n = luma_at(uv + vec2<f32>(0.0, -texel.y));
    var s = luma_at(uv + vec2<f32>(0.0, texel.y));
    let e = luma_at(uv + vec2<f32>(texel.x, 0.0));
    let w = luma_at(uv + vec2<f32>(-texel.x, 0.0));
    let range_max = max(max(max(n, s), max(e, w)), m);
    let range_min = min(min(min(n, s), min(e, w)), m);
    let range = range_max - range_min;
    if (range < max(0.0625, range_max * 0.125)) {
        return vec4<f32>(center.rgb, 1.0);
    }

    let nw = luma_at(uv + vec2<f32>(-texel.x, -texel.y));
    let ne = luma_at(uv + vec2<f32>(texel.x, -texel.y));
    let sw = luma_at(uv + vec2<f32>(-texel.x, texel.y));
    let se = luma_at(uv + vec2<f32>(texel.x, texel.y));
    let ns = n + s;
    let we = w + e;
    let edge_h = abs(-2.0 * m + ns) * 2.0
        + abs(-2.0 * e + ne + se) + abs(-2.0 * w + nw + sw);
    let edge_v = abs(-2.0 * m + we) * 2.0
        + abs(-2.0 * n + nw + ne) + abs(-2.0 * s + sw + se);
    let horizontal = edge_h >= edge_v;
    let subpix_a = (ns + we) * 2.0 + nw + ne + sw + se;
    let subpix_b = subpix_a / 12.0 - m;
    let subpix_c = clamp(abs(subpix_b) / range, 0.0, 1.0);
    let subpix_f = (3.0 - 2.0 * subpix_c) * subpix_c * subpix_c;
    let subpix_offset = subpix_f * subpix_f * 0.75;

    if (!horizontal) {
        n = w;
        s = e;
    }
    let gradient_n = n - m;
    let gradient_s = s - m;
    let pair_n = abs(gradient_n) >= abs(gradient_s);
    let gradient = max(abs(gradient_n), abs(gradient_s));
    var length_sign = select(texel.x, texel.y, horizontal);
    if (pair_n) { length_sign = -length_sign; }
    var pos_b = uv;
    if (horizontal) { pos_b.y += length_sign * 0.5; }
    else { pos_b.x += length_sign * 0.5; }
    let search_step = select(vec2<f32>(0.0, texel.y), vec2<f32>(texel.x, 0.0), horizontal);
    var pos_n = pos_b - search_step;
    var pos_p = pos_b + search_step;
    let luma_nn = select(s + m, n + m, pair_n);
    let luma_mm = m - luma_nn * 0.5;
    let gradient_scaled = gradient * 0.25;
    var end_n = luma_at(pos_n) - luma_nn * 0.5;
    var end_p = luma_at(pos_p) - luma_nn * 0.5;
    var done_n = abs(end_n) >= gradient_scaled;
    var done_p = abs(end_p) >= gradient_scaled;

    // Quality preset 12: sample P0, then P1/P2/P3 while needed; P4 extends
    // the final span without another sample, matching the reference branch.
    let steps = array<f32, 3>(1.5, 2.0, 4.0);
    for (var i = 0u; i < 3u && (!done_n || !done_p); i += 1u) {
        if (!done_n) { pos_n -= search_step * steps[i]; }
        if (!done_p) { pos_p += search_step * steps[i]; }
        if (!done_n) {
            end_n = luma_at(pos_n) - luma_nn * 0.5;
            done_n = abs(end_n) >= gradient_scaled;
        }
        if (!done_p) {
            end_p = luma_at(pos_p) - luma_nn * 0.5;
            done_p = abs(end_p) >= gradient_scaled;
        }
    }
    if (!done_n) { pos_n -= search_step * 12.0; }
    if (!done_p) { pos_p += search_step * 12.0; }

    let dist_n = select(uv.y - pos_n.y, uv.x - pos_n.x, horizontal);
    let dist_p = select(pos_p.y - uv.y, pos_p.x - uv.x, horizontal);
    let good_n = (end_n < 0.0) != (luma_mm < 0.0);
    let good_p = (end_p < 0.0) != (luma_mm < 0.0);
    let good_span = select(good_p, good_n, dist_n < dist_p);
    let pixel_offset = 0.5 - min(dist_n, dist_p) / (dist_n + dist_p);
    let edge_offset = select(0.0, pixel_offset, good_span);
    let offset = max(edge_offset, subpix_offset);
    var final_uv = uv;
    if (horizontal) { final_uv.y += offset * length_sign; }
    else { final_uv.x += offset * length_sign; }
    return vec4<f32>(sample_at(final_uv).rgb, 1.0);
}
