struct GroundBandMapping {
    region: vec4<f32>,
    top_m: f32,
    border_m: f32,
    padding: vec2<f32>,
}
@group(1) @binding(19) var ground_band: texture_3d<f32>;
@group(1) @binding(20) var<uniform> ground_band_mapping: GroundBandMapping;

// Return the optical-depth factor, the surface visibility, and the band's
// weight. Ground and height endpoints are at texel centres.
fn ground_band_sample(point: vec3<f32>) -> vec3<f32> {
    let region = ground_band_mapping.region;
    let edge = min(point.xz - region.xy, region.zw - point.xz);
    let border = min(edge.x, edge.y);
    if border <= 0.0 { return vec3(1.0, 1.0, 0.0); }
    let size = vec3<f32>(textureDimensions(ground_band));
    let uv = ((point.xz - region.xy) / (region.zw - region.xy) * (size.xy - 1.0) + 0.5) / size.xy;
    let ground = textureSampleLevel(ground_band, aerial_sampler, vec3(uv, 0.5 / size.z), 0.0).g;
    let top = ground_band_mapping.top_m;
    let height = point.y - ground;
    if height >= top { return vec3(1.0, 1.0, 0.0); }
    let level = (clamp(height / top, 0.0, 1.0) * (size.z - 1.0) + 0.5) / size.z;
    let band = textureSampleLevel(ground_band, aerial_sampler, vec3(uv, level), 0.0);
    return vec3(exp(-max(0.0, band.r)), band.b, smoothstep(0.0, ground_band_mapping.border_m, border));
}

fn cloud_factor(point: vec3<f32>) -> f32 {
    let band = ground_band_sample(point);
    if band.z >= 1.0 { return band.x; }
    return mix(cloud_plane_factor(point), band.x, band.z);
}
