@group(0) @binding(1) var reflection_sky: texture_2d<f32>;
@group(0) @binding(2) var reflection_sampler: sampler;
@group(0) @binding(3) var reflection_output: texture_storage_2d_array<rgba16float,write>;
@group(0) @binding(4) var<storage,read> prefilter_samples: array<vec4<f32>>;
fn cube_direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let q=2.0*uv-1.0;
    var direction=vec3(0.0);
    switch face {
        case 0u: { direction=vec3(1.0,-q.y,-q.x); }
        case 1u: { direction=vec3(-1.0,-q.y,q.x); }
        case 2u: { direction=vec3(q.x,1.0,q.y); }
        case 3u: { direction=vec3(q.x,-1.0,-q.y); }
        case 4u: { direction=vec3(q.x,-q.y,1.0); }
        default: { direction=vec3(-q.x,-q.y,-1.0); }
    }
    return normalize(direction);
}
@compute @workgroup_size(8,8)
fn reflection_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size=textureDimensions(reflection_output);
    if any(id.xy>=size) || id.z>=6u { return; }
    let normal=cube_direction(id.z,(vec2<f32>(id.xy)+0.5)/vec2<f32>(size));
    var up=vec3(0.0,0.0,1.0);
    if abs(normal.z)>0.999 { up=vec3(1.0,0.0,0.0); }
    let tangent=normalize(cross(up,normal));
    let bitangent=cross(normal,tangent);
    // Output views select one power-of-two mip. Derive its parameter from
    // actual view dimensions rather than mutable per-pass uniform uploads.
    let mip=countLeadingZeros(size.x)-countLeadingZeros(REFLECTION_SIZE);
    var radiance=vec4(0.0);
    var weight=0.0;
    for(var sample=0u;sample<PREFILTER_SAMPLES;sample++) {
        let prepared=prefilter_samples[mip*PREFILTER_SAMPLES+sample];
        let local=prepared.xyz;
        if prepared.w<=0.0 { continue; }
        let direction=tangent*local.x+bitangent*local.y+normal*local.z;
        let uv=sky_uv(direction,atmosphere.sun_lux.xyz);
        // Sky has disjoint curved-horizon halves and excludes the Sun disk.
        // Preserve its gain-scaled unit-Sun radiance through this linear filter.
        radiance+=prepared.w*textureSampleLevel(reflection_sky,reflection_sampler,uv,0.0);
        weight+=prepared.w;
    }
    textureStore(reflection_output,vec2<i32>(id.xy),i32(id.z),radiance/weight);
}
