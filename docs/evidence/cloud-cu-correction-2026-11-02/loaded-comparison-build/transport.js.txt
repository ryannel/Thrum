// Shared finite Beer segment, with an independent float64 oracle in optical_reference.py.
export const BEER_WGSL = `
fn beerFactor(tau: f32) -> f32 {
  if (tau < .05) {
    return 1. + tau*(-.5 + tau*(1./6. + tau*(-1./24. + tau*(1./120. - tau/720.))));
  }
  return (1.-exp(-tau))/tau;
}
fn homogeneousScalar(input: vec4<f32>) -> vec4<f32> {
  let tau=input.x*input.y;
  let T=exp(-tau);
  let S=input.z*input.y*beerFactor(tau);
  return vec4<f32>(tau,T,S,S+input.w*T);
}
`;
