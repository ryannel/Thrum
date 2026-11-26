// Shape-preserving cubic interpolation in height of cumulative Sun depth.
// Harmonic secant slopes keep every interval inside its two endpoint values:
// no negative depth, ringing or invented bright/dark extrema at a cloud edge.
fn cloud_path_slope(a:f32,b:f32)->f32 {
    if a*b<=0.0 {return 0.0;}
    return 2.0*a*b/(a+b);
}
fn cloud_path_cubic(v:vec4<f32>,t:f32)->f32 {
    let d=v.yzw-v.xyz;
    let slopes=vec2(cloud_path_slope(d.x,d.y),cloud_path_slope(d.y,d.z));
    let t2=t*t;let t3=t2*t;
    let result=v.y*(2.0*t3-3.0*t2+1.0)+slopes.x*(t3-2.0*t2+t)
        +v.z*(-2.0*t3+3.0*t2)+slopes.y*(t3-t2);
    return clamp(result,min(v.y,v.z),max(v.y,v.z));
}
fn cloud_path_nodes(field:texture_2d_array<f32>,uv:vec2<f32>,first:i32,count:i32)->vec4<f32> {
    var result=vec4(0.0);
    // Four consecutive height nodes occupy at most two packed array layers.
    let lower=clamp(first,0,count-1)/4;
    let upper=clamp(first+3,0,count-1)/4;
    let a=textureSampleLevel(field,noise_sampler,uv,lower,0.0);
    var b=a;
    if upper!=lower {b=textureSampleLevel(field,noise_sampler,uv,upper,0.0);}
    for(var i=0;i<4;i++) {
        let node=clamp(first+i,0,count-1);
        result[i]=select(b[node%4],a[node%4],node/4==lower);
    }
    return result;
}
fn cloud_path_above(field:texture_2d_array<f32>,uv:vec2<f32>,height:f32,layers:u32,limits:vec2<f32>)->f32 {
    let count=layers*4u;
    let q=clamp((limits.y-height)/(limits.y-limits.x),0.0,1.0)*f32(count-1u);
    let first=i32(floor(q))-1;
    // Hardware bilinear filtering in X/Z preserves cumulative ordering.
    // The cubic still operates on the four interpolated height nodes.
    let nodes=cloud_path_nodes(field,uv,first,i32(count));
    let t=fract(q);
    let grazing=1.0-smoothstep(0.18,0.35,mapping.path_border.y);
    let interior=smoothstep(0.0,1.0,min(q,f32(count-1u)-q));
    // One height interval spans more of the ray at low Sun. Use the same
    // four depths to average over that interval without creating extrema.
    let s=1.0-t;
    let weights=vec4(s*s*s,4.0-t*t*(6.0-3.0*t),4.0-s*s*(6.0-3.0*s),t*t*t);
    let filtered=dot(nodes,weights)/6.0;
    return mix(cloud_path_cubic(nodes,t),filtered,grazing*interior);
}

// Both heights lie on the same Sun ray. Reuse its horizontal lookup and
// total depth when replacing the segment between the two heights.
fn cloud_path_sample(field:texture_2d_array<f32>,uv:vec2<f32>,heights:vec2<f32>,limits:vec2<f32>)->vec3<f32> {
    let layers=textureNumLayers(field);
    // Constrain the repeating noise sampler to the Sun texture's edge centres.
    let edge=0.5/vec2<f32>(textureDimensions(field));
    let coordinate=clamp(uv,edge,1.0-edge);
    let whole=textureSampleLevel(field,noise_sampler,coordinate,i32(layers)-1,0.0).w;
    let above=cloud_path_above(field,coordinate,heights.x,layers,limits);
    var above_start=above;
    if heights.y!=heights.x {
        above_start=cloud_path_above(field,coordinate,heights.y,layers,limits);
    }
    return vec3(above,whole,above_start);
}
