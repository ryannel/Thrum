@group(0) @binding(0) var dfg_output: texture_storage_2d<rgba16float,write>;
@group(0) @binding(1) var<storage,read_write> prefilter_output: array<vec4<f32>>;

// Prepare each mip's BRDF-only samples once per device.
@compute @workgroup_size(64)
fn prefilter_samples_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let mip=id.y;
    let span=PREFILTER_SPANS[mip];
    let sample=id.x;
    if sample>=span.y { return; }
    let roughness=GGX_MIN_ROUGHNESS+(1.0-GGX_MIN_ROUGHNESS)*f32(mip)/f32(REFLECTION_MIPS-1u);
    let half_vector=ggx_visible_normal(vec3(0.0,0.0,1.0),ggx_alpha(roughness),ggx_sample(sample,span.y));
    let light=reflect(vec3(0.0,0.0,-1.0),half_vector);
    prefilter_output[span.x+sample]=vec4(light,max(0.0,light.z));
}

@compute @workgroup_size(8,8)
fn dfg_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size=textureDimensions(dfg_output);
    if any(id.xy>=size) { return; }
    let nov=f32(id.x)/f32(size.x-1u);
    let roughness=GGX_MIN_ROUGHNESS+(1.0-GGX_MIN_ROUGHNESS)*f32(id.y)/f32(size.y-1u);
    let alpha=ggx_alpha(roughness);
    let a2=alpha*alpha;
    let view=vec3(sqrt(max(0.0,1.0-nov*nov)),0.0,nov);
    let sv=sqrt(a2+(1.0-a2)*nov*nov);
    var result=vec2(0.0);
    for(var sample=0u;sample<DFG_SAMPLES;sample++) {
        let half_vector=ggx_visible_normal(view,alpha,ggx_sample(sample,DFG_SAMPLES));
        let voh=clamp(dot(view,half_vector),0.0,1.0);
        let light=reflect(-view,half_vector);
        let nol=light.z;
        if nol<=0.0 || voh<=0.0 { continue; }
        let sl=sqrt(a2+(1.0-a2)*nol*nol);
        // VNDF cancels D and the outgoing cosine. This correlated-Smith
        // G2/G1 ratio is bounded by one and remains finite at NoV=0.
        let weight=nol*(nov+sv)/(nol*sv+nov*sl);
        let fc=pow(1.0-voh,5.0);
        result+=weight*vec2(1.0-fc,fc);
    }
    // Rejected directions contribute zero; never renormalize by acceptance.
    textureStore(dfg_output,vec2<i32>(id.xy),vec4(result/f32(DFG_SAMPLES),0.0,1.0));
}
