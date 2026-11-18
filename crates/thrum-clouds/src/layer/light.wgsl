@group(2) @binding(9) var<storage,read> cloud_phase:array<f32>;
@group(2) @binding(10) var<storage,read> cloud_ambient:array<vec4<f32>,2>;
fn cloud_hg_phase(s2:f32,g:f32)->f32 {
    return (1.0-g*g)/(4.0*PI*pow((1.0-g)*(1.0-g)+4.0*g*s2,1.5));
}
fn cloud_phase_row(row:u32,s2:f32)->f32 {
    let entries=u32(cloud.erosion.w);
    let at=sqrt(sqrt(clamp(s2,0.0,1.0)))*f32(entries-1u);
    let lo=min(u32(floor(at)),entries-2u);
    let start=row*entries+lo;
    return mix(cloud_phase[start],cloud_phase[start+1u],at-f32(lo));
}
fn cloud_phases(ray:vec3<f32>,sun:vec3<f32>)->vec2<f32> {
    // |ray-sun|^2/4 preserves the forward peak where 1-dot loses digits.
    let s2=clamp(dot(ray-sun,ray-sun)*0.25,0.0,1.0);
    let row=u32(cloud.erosion.x);
    let first=mix(cloud_phase_row(row,s2),cloud_phase_row(row+1u,s2),cloud.erosion.y);
    // Narrow the middle lobe to avoid a bright ring far outside the Sun's vicinity.
    return vec2(first,cloud_hg_phase(s2,mix(cloud.erosion.z*cloud.scattering.z,cloud.erosion.z,0.65)));
}
fn cloud_source(solar:vec3<f32>,phases:vec2<f32>,sun_depths:vec2<f32>,
                vertical_depths:vec2<f32>,above:f32,ray:vec3<f32>,distance:f32,transmission:f32)->vec3<f32> {
    // The first profile's near/far medium handover ends at 500 m.
    let boundary=cloud_smooth((distance-500.0)/500.0);
    let g=cloud.erosion.z;
    let depth=-log(transmission);
    let direct=cloud_solar_source(phases,sun_depths,above,-ray,depth,boundary,
        mapping.path_border.xyz,cloud.scattering.xyz,g);
    let diffuse_depth=0.75*(1.0-g)*vertical_depths;
    let ambient=cloud_ambient[0].rgb/(1.0+diffuse_depth.x)+cloud_ambient[1].rgb/(1.0+diffuse_depth.y);
    return solar*direct+ambient;
}
