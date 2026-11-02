// Sun-air-only profile. No finite-segment ratios, view source or sky composite.
struct AirSunProfile { geometry:vec4f,rayleigh:vec4f,aerosol:vec4f }
@group(0) @binding(20) var airSunTable:texture_2d<f32>;
@group(0) @binding(21) var airSunFiltered:sampler;
@group(0) @binding(22) var<uniform> airSunProfile:AirSunProfile;
const AIR_SUN_UNIT_TOLERANCE:f32=.00001;
const AIR_SUN_COS_ALLOWANCE:f32=.000002;
const AIR_SUN_COORD_ALLOWANCE:f32=.000002;
const AIR_SUN_HORIZON_AMBIGUITY:f32=.000001;
struct AirSunGeometry { height:f32,mu:f32,valid:bool }
fn airSunFinite(value:f32)->bool {return value==value&&abs(value)<3.402823e38;}
fn airSunInvalid()->vec4f {return vec4f(-1.,-1.,-1.,0.);}
// -1 unsupported,0 opaque planet,1 visible medium path. Planet visibility
// precedes sampling: the final angular table column is only a medium LIMIT.
fn airSunClassifyHeightMu(height:f32,mu:f32)->i32 {
  let R=airSunProfile.geometry.x;let A=airSunProfile.geometry.y;
  if(!airSunFinite(height)||!airSunFinite(mu)||height<0.||height>A||mu< -1.||mu>1.){return -1;}
  if(height==0.&&mu<=0.){return 0;}
  if(mu<0.){
    let rho=sqrt(height*(2.*R+height));let horizon= -rho/(R+height);
    if(abs(mu-horizon)<=AIR_SUN_HORIZON_AMBIGUITY){return -1;}
    if(mu<=horizon){return 0;}
  }
  return 1;
}
// Preconditions: admitted visible height/mu. Top/outward is explicit.
fn airSunCoordinates(height:f32,mu:f32)->vec2f {
  let R=airSunProfile.geometry.x;let A=airSunProfile.geometry.y;
  if(height==A&&mu>=0.){return vec2f(0.,1.);}
  let r=R+height;let rho=sqrt(height*(2.*R+height));let H=sqrt(A*(2.*R+A));
  let delta=A-height;let C=delta*(2.*R+A+height);let b=r*mu;let root=sqrt(b*b+C);
  var difference=root-b-delta;
  if(mu>=0.){difference=2.*r*delta*(1.-mu)/(root+b+delta);}
  return vec2f(difference/(rho+H-delta),rho/H);
}
// Inspector also uses this internal sampler at medium-only table knots.
// It does NOT imply planet visibility for a tangent or admit a ratio API.
fn airSunSampleCoordinates(coordinates:vec2f)->vec4f {
  if(!airSunFinite(coordinates.x)||!airSunFinite(coordinates.y)||any(coordinates<vec2f(-AIR_SUN_COORD_ALLOWANCE))||any(coordinates>vec2f(1.+AIR_SUN_COORD_ALLOWANCE))){return airSunInvalid();}
  let x=clamp(coordinates,vec2f(0.),vec2f(1.));
  let uv=(vec2f(.5)+x*vec2f(255.,63.))/vec2f(256.,64.);
  let rgb=textureSampleLevel(airSunTable,airSunFiltered,uv,0.).rgb;
  if(!airSunFinite(rgb.x)||!airSunFinite(rgb.y)||!airSunFinite(rgb.z)||any(rgb<vec3f(0.))||any(rgb>vec3f(1.))){return airSunInvalid();}
  return vec4f(rgb,1.);
}
fn airSunTransmittanceAtHeightMu(height:f32,mu:f32)->vec4f {
  let classification=airSunClassifyHeightMu(height,mu);
  if(classification<0){return airSunInvalid();}
  if(classification==0){return vec4f(0.,0.,0.,1.);}
  if(height==airSunProfile.geometry.y&&mu>=0.){return vec4f(1.);}
  return airSunSampleCoordinates(airSunCoordinates(height,mu));
}
fn airSunWorldGeometry(p:vec3f,sun:vec3f)->AirSunGeometry {
  // Local tangent profile only. Nonnegative y makes the rationalized height
  // numerator a sum of nonnegative terms; no sqrt(radius²)-R subtraction.
  if(!airSunFinite(p.x)||!airSunFinite(p.y)||!airSunFinite(p.z)||abs(p.x)>100000.||abs(p.z)>100000.||p.y<0.||p.y>20000.){return AirSunGeometry(0.,0.,false);}
  if(!airSunFinite(sun.x)||!airSunFinite(sun.y)||!airSunFinite(sun.z)){return AirSunGeometry(0.,0.,false);}
  let sunLength=length(sun);
  if(!airSunFinite(sunLength)||abs(sunLength-1.)>AIR_SUN_UNIT_TOLERANCE){return AirSunGeometry(0.,0.,false);}
  let direction=sun/sunLength;let R=airSunProfile.geometry.x;
  let numerator=p.x*p.x+p.z*p.z+p.y*(2.*R+p.y);
  let radial=vec3f(p.x,R+p.y,p.z);let radius=length(radial);
  let height=numerator/(radius+R);let cosine=dot(radial,direction)/radius;
  if(!airSunFinite(height)||!airSunFinite(cosine)||cosine< -1.-AIR_SUN_COS_ALLOWANCE||cosine>1.+AIR_SUN_COS_ALLOWANCE){return AirSunGeometry(0.,0.,false);}
  // Only wrapper dot roundoff after the explicit unit-direction check.
  return AirSunGeometry(height,clamp(cosine,-1.,1.),true);
}
fn airSunTransmittance(p:vec3f,sun:vec3f)->vec4f {
  let geometry=airSunWorldGeometry(p,sun);
  if(!geometry.valid){return airSunInvalid();}
  return airSunTransmittanceAtHeightMu(geometry.height,geometry.mu);
}
