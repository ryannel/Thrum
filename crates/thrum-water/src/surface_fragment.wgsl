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
