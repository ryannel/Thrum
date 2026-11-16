// Dielectric interface and its visible, unresolved slope distribution. This is
// independent of illumination: all radiance still comes from the lent library.
fn water_fresnel(cosine:f32)->f32 {
    let c=clamp(cosine,0.0,1.0);let index=1.333;
    let transmitted=sqrt(1.0-(1.0-c*c)/(index*index));
    let perpendicular=(c-index*transmitted)/(c+index*transmitted);
    let parallel=(index*c-transmitted)/(index*c+transmitted);
    return 0.5*(perpendicular*perpendicular+parallel*parallel);
}
fn water_mean_fresnel(slope:vec2<f32>,covariance:vec3<f32>,v:vec3<f32>)->f32 {
    if covariance.x+covariance.z<0.00000001 {
        return water_fresnel(dot(normalize(vec3(-slope.x,1.0,-slope.y)),v));
    }
    // Four-point Gauss-Hermite nodes/weights for a standard normal variable.
    // The tensor rule integrates both slope axes; projected visible area, not
    // the count of normals, weights the Fresnel split. Directional radiance
    // and its filtering remain the lent environment provider's responsibility.
    let nodes=array<f32,4>(-2.334414218,-0.741963784,0.741963784,2.334414218);
    let weights=array<f32,4>(0.045875855,0.454124145,0.454124145,0.045875855);
    let a=sqrt(max(0.0,covariance.x));
    let cross_limit=sqrt(max(0.0,covariance.x*covariance.z));
    let b=clamp(covariance.y,-cross_limit,cross_limit)/max(a,0.000001);
    let c=sqrt(max(0.0,covariance.z-b*b));
    var reflected=0.0;var visible=0.0;
    for(var i=0u;i<4u;i++){for(var j=0u;j<4u;j++) {
        let s=slope+vec2(a*nodes[i],b*nodes[i]+c*nodes[j]);
        let n=normalize(vec3(-s.x,1.0,-s.y));
        let nv=dot(n,v);
        let weight=weights[i]*weights[j]*max(0.0,v.y-dot(v.xz,s));
        visible+=weight;
        reflected+=weight*water_fresnel(nv);
    }}
    return clamp(reflected/max(visible,0.0000001),0.0,1.0);
}
