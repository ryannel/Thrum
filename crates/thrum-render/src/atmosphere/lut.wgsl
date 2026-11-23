@group(0) @binding(1) var transmittance: texture_2d<f32>;
@group(0) @binding(2) var multiple_scattering: texture_2d<f32>;
@group(0) @binding(3) var output_lut: texture_storage_2d<rgba16float,write>;
@group(0) @binding(4) var lut_sampler: sampler;
@group(0) @binding(5) var ground_irradiance: texture_2d<f32>;
@compute @workgroup_size(LUT_WORKGROUP_EDGE, LUT_WORKGROUP_EDGE)
fn transmittance_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = plane_size(textureDimensions(output_lut),TRANS_PLANES);
    if any(id.xy>=size) { return; }
    let params = trans_params((vec2<f32>(id.xy)+0.5)/vec2<f32>(size));
    let h = params.x;
    let mu = params.y;
    let direction = vec3(sqrt(max(0.0,1.0-mu*mu)),mu,0.0);
    let step = top_distance(h,mu)/64.0;
    var columns = vec3(0.0);
    for(var i=0u;i<64u;i++) {
        columns += medium_densities(altitude(vec3(0.0,h,0.0)+direction*(f32(i)+0.5)*step))*step;
    }
    let lo=exp(-extinction(columns,0u));
    let hi=exp(-extinction(columns,1u));
    textureStore(output_lut,vec2<i32>(id.xy),vec4(project_spectrum(lo,hi),1.0));
    textureStore(output_lut,vec2<i32>(id.xy+vec2(0u,size.y)),lo);
    textureStore(output_lut,vec2<i32>(id.xy+vec2(0u,2u*size.y)),hi);
}
