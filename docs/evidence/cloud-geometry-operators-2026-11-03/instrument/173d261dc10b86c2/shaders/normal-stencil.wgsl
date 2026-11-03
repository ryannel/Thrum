// Pure frontal-normal operator. Neighbour order is +x,-x,+y,-y,+z,-z.
fn applyFrontMotion(center:vec4f, neighbours:array<vec4f,6>, pitch:vec3f, dt:f32, kappa:f32, growth:f32, erosion:f32)->vec4f {
  let c=max(center.x+center.y,0.);
  var gp2=0.;var gm2=0.;
  for(var axis=0u;axis<3u;axis++) {
    let plus=max(neighbours[2u*axis].x+neighbours[2u*axis].y,0.);
    let minus=max(neighbours[2u*axis+1u].x+neighbours[2u*axis+1u].y,0.);
    let inv=1./pitch[axis];
    let gp=max(max(plus-c,minus-c),0.)*inv;
    let gm=max(max(c-plus,c-minus),0.)*inv;
    gp2+=gp*gp;gm2+=gm*gm;
  }
  let rawGrowth=max(growth,0.)*dt*sqrt(gp2);
  let rawErosion=max(erosion,0.)*dt*sqrt(gm2);
  let alpha=vec3f(kappa*dt)/(pitch*pitch);
  var mixed=vec4f(center.xy,c*center.z,center.w)*(1.-2.*(alpha.x+alpha.y+alpha.z));
  for(var axis=0u;axis<3u;axis++) {
    for(var side=0u;side<2u;side++) {
      let n=neighbours[2u*axis+side];
      mixed+=alpha[axis]*vec4f(n.xy,max(n.x+n.y,0.)*n.z,n.w);
    }
  }
  let old=max(mixed.xy,vec2f(0.));
  let total=old.x+old.y;
  let liquidLoss=min(old.x,rawErosion*old.x/max(total,1e-12));
  let iceLoss=min(old.y,.08*rawErosion*old.y/max(total,1e-12));
  let survivor=old-vec2f(liquidLoss,iceLoss);
  let survivingTotal=survivor.x+survivor.y;
  let survivingFraction=select(0.,survivingTotal/max(total,1e-12),total>1e-12);
  let addition=min(rawGrowth,max(1.-survivor.x,0.));
  let result=survivor+vec2f(addition,0.);
  let ageMoment=max(mixed.z,0.)*survivingFraction;
  let age=select(0.,ageMoment/max(result.x+result.y,1e-12),result.x+result.y>1e-12);
  let birth=max(mixed.w,0.)*survivingFraction+addition;
  return vec4f(result,age,birth);
}
