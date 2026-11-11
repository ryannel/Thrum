struct CloudField {
    region: vec4<f32>, // min XZ, max XZ in the advected field coordinates
    plane_drift: vec4<f32>, // plane Y, displacement XZ, lowest valid Sun sine
    validity: vec4<f32>,
}
@group(1) @binding(13) var cloud_optical_depth: texture_2d<f32>;
@group(1) @binding(14) var<uniform> cloud_field: CloudField;

// One factor for every direct-Sun path. Field values already describe optical
// depth along the true Sun ray, so no second obliquity correction is applied.
fn cloud_factor(world_point: vec3<f32>) -> f32 {
    let sun=aerial_frame.sun_lux.xyz;
    if cloud_field.validity.x==0.0 || world_point.y>=cloud_field.plane_drift.x
        || sun.y<cloud_field.plane_drift.w { return 1.0; }
    let distance=(cloud_field.plane_drift.x-world_point.y)/sun.y;
    let point=world_point.xz+sun.xz*distance-cloud_field.plane_drift.yz;
    if any(point<cloud_field.region.xy) || any(point>cloud_field.region.zw) { return 1.0; }
    let uv=(point-cloud_field.region.xy)/(cloud_field.region.zw-cloud_field.region.xy);
    let tau=max(0.0,textureSampleLevel(cloud_optical_depth,aerial_sampler,uv,0.0).r);
    return exp(-tau);
}
