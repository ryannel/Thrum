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
fn cloud_path_nodes(field:texture_2d_array<f32>,pixel:vec2<i32>,first:i32)->vec4<f32> {
    let size=vec2<i32>(textureDimensions(field));
    let count=i32(textureNumLayers(field))*4;
    var result=vec4(0.0);
    // Four consecutive height nodes occupy at most two packed array layers.
    let lower=clamp(first,0,count-1)/4;
    let upper=clamp(first+3,0,count-1)/4;
    let a=textureLoad(field,clamp(pixel,vec2(0),size-1),lower,0);
    var b=a;
    if upper!=lower {b=textureLoad(field,clamp(pixel,vec2(0),size-1),upper,0);}
    for(var i=0;i<4;i++) {
        let node=clamp(first+i,0,count-1);
        result[i]=select(b[node%4],a[node%4],node/4==lower);
    }
    return result;
}
fn cloud_path_sample(field:texture_2d_array<f32>,uv:vec2<f32>,height:f32)->f32 {
    let count=textureNumLayers(field)*4u;
    let q=clamp((cloud.heights.y-height)/(cloud.heights.y-cloud.heights.x),0.0,1.0)*f32(count-1u);
    let pixel=uv*vec2<f32>(textureDimensions(field))-0.5;
    let lo=vec2<i32>(floor(pixel));let f=fract(pixel);
    let first=i32(floor(q))-1;
    // Positive bilinear weights preserve each column's cumulative ordering.
    // Cubics in X/Z would not preserve that ordering between height nodes.
    let a=cloud_path_nodes(field,lo,first);
    let b=cloud_path_nodes(field,lo+vec2(1,0),first);
    let c=cloud_path_nodes(field,lo+vec2(0,1),first);
    let d=cloud_path_nodes(field,lo+vec2(1),first);
    let vertical=mix(mix(a,b,f.x),mix(c,d,f.x),f.y);
    return cloud_path_cubic(vertical,fract(q));
}
