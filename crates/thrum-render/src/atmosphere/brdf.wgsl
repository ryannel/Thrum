// Independently derived isotropic GGX visible-normal sampling (Heitz 2018).
// Shared perceptual-roughness convention with direct and indirect materials.
const GGX_PI: f32 = 3.14159265359;
const GGX_MIN_ROUGHNESS: f32 = 0.045;
fn ggx_alpha(roughness: f32) -> f32 {
    let r=max(GGX_MIN_ROUGHNESS,roughness);
    return r*r;
}
fn ggx_sample(index: u32, count: u32) -> vec2<f32> {
    return vec2((f32(index)+0.5)/f32(count),f32(reverseBits(index))*2.3283064365386963e-10);
}
// Local +Z is the macro normal. Stretch the view onto the unit hemisphere,
// sample its projected disk, then undo the ellipsoid stretch on the normal.
fn ggx_visible_normal(view: vec3<f32>, alpha: f32, xi: vec2<f32>) -> vec3<f32> {
    let stretched=normalize(vec3(alpha*view.xy,view.z));
    let radial=dot(stretched.xy,stretched.xy);
    var tangent=vec3(1.0,0.0,0.0);
    if radial>0.0 { tangent=vec3(-stretched.y,stretched.x,0.0)/sqrt(radial); }
    let bitangent=cross(stretched,tangent);
    let radius=sqrt(xi.x);
    let phi=2.0*GGX_PI*xi.y;
    let x=radius*cos(phi);
    let blend=0.5*(1.0+stretched.z);
    let y=(1.0-blend)*sqrt(max(0.0,1.0-x*x))+blend*radius*sin(phi);
    let normal=x*tangent+y*bitangent+sqrt(max(0.0,1.0-x*x-y*y))*stretched;
    return normalize(vec3(alpha*normal.xy,max(0.0,normal.z)));
}
