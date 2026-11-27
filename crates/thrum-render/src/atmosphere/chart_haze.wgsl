@group(0) @binding(1) var sky: texture_2d<f32>;
@group(0) @binding(2) var aerial_sampler: sampler;
@group(0) @binding(4) var<uniform> live_atmosphere: Atmosphere;
var<workgroup> horizontal_lux: array<vec4<f32>, 64>;

@compute @workgroup_size(64)
fn haze_main(@builtin(local_invocation_index) index: u32) {
    if !classify_overlay(index) {
        if index == 0u { overlay_chart.haze = 1.0; }
        return;
    }
    let solar = live_atmosphere.sun_lux.w * live_atmosphere.colour_pre_exposure.rgb;
    var shaded = vec3(0.0);
    var open = 0.0;
    for (var sample = 0u; sample < 4u; sample++) {
        let direction = overlay_meter_direction(index + sample * 64u);
        let uv = sky_uv(direction, atmosphere.sun_lux.xyz);
        let encoded = textureSampleLevel(sky, aerial_sampler, uv, 0.0);
        let clear = decode_sky_light(encoded, solar);
        let layer = textureSampleLevel(sky_overlay, aerial_sampler, direction, 0.0);
        let weight = direction.y * 2.0 * PI / 256.0;
        shaded += (clear * layer.a + layer.rgb) * weight;
        open += dot(clear, REC709_LUMA) * weight;
    }
    horizontal_lux[index] = vec4(shaded, open);
    workgroupBarrier();
    for (var stride = 32u; stride > 0u; stride /= 2u) {
        if index < stride { horizontal_lux[index] += horizontal_lux[index + stride]; }
        workgroupBarrier();
    }
    if index == 0u {
        let direct = overlay_open_sun();
        let clear = horizontal_lux[0].w + direct;
        let shadowed = dot(horizontal_lux[0].rgb, REC709_LUMA) + direct * overlay_sun_factor();
        var scale = 1.0;
        if clear > 1e-8 && abs(shadowed - clear) > clear * 1e-6 {
            scale = clamp(shadowed / clear, 0.0, 1.0);
        }
        overlay_chart.haze = scale;
    }
}
