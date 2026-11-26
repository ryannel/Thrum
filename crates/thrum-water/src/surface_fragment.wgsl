    let row=surface_vertex(rest,body);
    var f=field_at(rest,vertex.z,row);
    var surf_weight = 0.0;
    var surf_motion = vec3(0.0);
    var surf_normal = vec3(0.0);
    var surf_variance = 0.0;
    let transformed = surf_current(index);
    let local=vertex.xy/vertex.z;
    if sea && max(abs(local.x),abs(local.y))>=f32(sea_grid_radius(vertex.z)) {
        let transition = 2.0;
        let coarse = local / transition;
        let spacing = vertex.z * transition;
        let p = anchor + floor(coarse) * spacing;
        let q = anchor + ceil(coarse) * spacing;
        let fraction = fract(coarse);
        let blend = select(0.5, max(fraction.x, fraction.y), SURF_CACHE_ENABLED);
        let ap=surface_vertex(p,body);
        let a=field_at(p,spacing,ap);
        let bp=surface_vertex(q,body);
        let b=field_at(q,spacing,bp);
        f=add_field(add_field(Field(vec4(0.0),vec4(0.0),vec4(0.0)),a,1.0-blend),b,blend);
        if SURF_CACHE_ENABLED && transformed.w > 0.0 {
            let long_p = long_at(p, spacing);
            let long_q = long_at(q, spacing);
            let long = add_field(add_field(Field(vec4(0.0),vec4(0.0),vec4(0.0)),
                long_p,(1.0-blend)*surface_vertex(p,body).gains.x),
                long_q,blend*surface_vertex(q,body).gains.x);
            f = add_field(f,long,-1.0);
        }
    } else if SURF_CACHE_ENABLED && transformed.w > 0.0 {
        f = add_field(f,long_at(rest,vertex.z),-row.gains.x);
    }
    if SURF_CACHE_ENABLED && transformed.w > 0.0 {
        surf_weight = 1.0;
        let short = surf_short_weights(index);
        let previous_short = f.displacement.xyz - f.velocity.xyz * water.level_dt_reset.y;
        f.displacement = short.x * f.displacement + vec4(transformed.xyz, 0.0);
        f.velocity *= short.x;
        let previous_sample = surf_previous(index);
        surf_motion = transformed.xyz - previous_sample.xyz + (short.x - short.y) * previous_short;
        surf_variance = previous_sample.w;
        if sea { surf_normal = sea_surf_normal(index, local, vertex.z); }
    }
    let wake=wake_at(rest,0.0);
    f.displacement.y+=wake.x;
    f.velocity.y+=wake.w;
    var world=vec3(rest.x+f.displacement.x,row.level+f.displacement.y,rest.y+f.displacement.z);
    // Infinite final edge: clamp reverse-Z to the far plane, preserving the horizon.
    if sea && max(abs(vertex.x),abs(vertex.y))>=524288.0 {world=vec3(anchor.x+vertex.x*100.0,row.level,anchor.y+vertex.y*100.0);}
    var clip=view.view_proj*vec4(world,1.0);clip.z=max(clip.z,0.0000001);
    let current=view.unjittered_view_proj*vec4(world,1.0);
    let previous=view.previous_view_proj*vec4(world-f.velocity.xyz*water.level_dt_reset.y-surf_motion-vec3(0.0,row.level-row.previous_level,0.0),1.0);
    // A coarsely drawn crest still carries its breaking-face cue over the
    // foam-window blend. Fade it once the mesh also loses the crest itself.
    let resolved_face = 1.0 - smoothstep(2.0, 16.0, vertex.z);
    return VertexOut(clip,world,rest,f.velocity.xyz,current,previous,body,surf_weight,
        vec4(surf_normal, surf_variance), resolved_face * max(0.0, transformed.w - 1.0));
}
struct Output { @location(0) colour:vec4<f32>,@location(1) motion:vec2<f32>,@location(2) reactive:f32, }
struct FilteredLayer { slope:vec2<f32>, second:vec3<f32>, differential:vec3<f32>, }
fn filtered_layer(t:texture_2d_array<f32>,s:sampler,uv:vec2<f32>,dx:vec2<f32>,dy:vec2<f32>,layer:i32)->FilteredLayer {
    let derivative=sample_field(t,s,uv,layer+1,dx,dy);
    let moments=sample_field(t,s,uv,layer+3,dx,dy);
    return FilteredLayer(derivative.xy,moments.xyz,vec3(derivative.zw,moments.w));
}
fn reflected_sky(slope:vec2<f32>,covariance:vec3<f32>,v:vec3<f32>)->vec4<f32> {
    // Integrate visible unresolved facets, clamping each reflected direction
    // before the lent sky lookup. A prefiltered cube at the horizon would
    // already include its ground hemisphere even if its centre were clamped.
    let nodes=array<f32,4>(-2.334414218,-0.741963784,0.741963784,2.334414218);
    let weights=array<f32,4>(0.045875855,0.454124145,0.454124145,0.045875855);
    let a=sqrt(max(0.0,covariance.x));
    let limit=sqrt(max(0.0,covariance.x*covariance.z));
    let b=clamp(covariance.y,-limit,limit)/max(a,0.000001);
    let c=sqrt(max(0.0,covariance.z-b*b));
    var result=vec4(0.0);var visible=0.0;
    for(var i=0u;i<4u;i++){for(var j=0u;j<4u;j++) {
        let s=slope+vec2(a*nodes[i],b*nodes[i]+c*nodes[j]);
        let n=normalize(vec3(-s.x,1.0,-s.y));
        let ray=reflect(-v,n);
        let f=water_fresnel(dot(n,v));
        let weight=weights[i]*weights[j]*max(0.0,v.y-dot(v.xz,s));
        let sky=environment_sky_radiance(normalize(vec3(ray.x,max(0.001,ray.y),ray.z)));
        result+=weight*vec4(sky*f,f);visible+=weight;
    }}
    return result/max(visible,0.0000001);
}
@fragment fn water_fragment(input:VertexOut)->Output {
    let dx=dpdx(input.rest);let dy=dpdy(input.rest);
    let wake_slope=wake_at(input.rest,0.0).yz;
    let wake_dx=dpdx(wake_slope);let wake_dy=dpdy(wake_slope);
    var row=surface_pixel(input.rest,input.body);
