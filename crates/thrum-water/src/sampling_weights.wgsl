// Positive cubic B-spline reconstruction of the linear LEAN moments.
// Bilinear reconstruction of s and s*s gives f*(1-f)*delta(s)^2 even
// for an affine field: its variance vanishes on the texel lattice. The
// cubic kernel has constant second central moment (1/3 texel squared),
// keeps the unresolved energy, and makes magnified shading continuous.
fn cubic_weights(f:f32)->vec4<f32> {
    let f2=f*f;let f3=f2*f;let g=1.0-f;
    return vec4(g*g*g,3.0*f3-6.0*f2+4.0,-3.0*f3+3.0*f2+3.0*f+1.0,f3)/6.0;
}
