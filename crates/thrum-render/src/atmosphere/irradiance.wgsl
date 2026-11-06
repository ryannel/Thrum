// Degree-two irradiance polynomial, in physical RGB lux. The basis uses
// Qzz=-Qxx-Qyy; cross-term factors are absorbed by the producer.
fn sky_diffuse_basis(n: vec3<f32>) -> array<f32,9> {
    return array<f32,9>(1.0,n.x,n.y,n.z,n.x*n.x-n.z*n.z,
        n.y*n.y-n.z*n.z,n.x*n.y,n.x*n.z,n.y*n.z);
}
fn evaluate_sky_diffuse(coefficients: array<vec4<f32>,9>, n: vec3<f32>) -> vec3<f32> {
    let basis=sky_diffuse_basis(n);
    var result=vec3(0.0);
    for(var i=0u;i<9u;i++) { result+=coefficients[i].rgb*basis[i]; }
    return result;
}
