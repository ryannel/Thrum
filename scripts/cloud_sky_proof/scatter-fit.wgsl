// Lookup-only section is exported as ScatterFit.lookupWGSL for finite GPU probes.
// Preconditions: verified finite profile, T>=0, 0<=mu0<=1, one product generation.
struct ScatterProfile { counts:vec4u, limits:vec4f }
@group(0) @binding(8) var scatterTable:texture_2d<f32>;
@group(0) @binding(9) var<storage,read> scatterAxes:array<f32>;
@group(0) @binding(10) var<uniform> scatterProfile:ScatterProfile;

fn scatterLower(value:f32,offset:u32,count:u32)->u32 {
  var lo=0u;var hi=count-1u;
  loop {
    if(hi-lo<=1u){break;}
    let middle=lo+(hi-lo)/2u;
    if(scatterAxes[offset+middle]<=value){lo=middle;}else{hi=middle;}
  }
  return lo;
}
fn scatterFitA(thickness:f32,mu0:f32)->f32 {
  if(thickness<scatterProfile.limits.x){return 0.;}
  let nt=scatterProfile.counts.x;let nm=scatterProfile.counts.y;
  let t=min(thickness,scatterProfile.limits.y);
  // Admitted mu0 is a cosine; clamp only protects roundoff at its endpoints.
  let mu=clamp(mu0,0.,1.);
  let ti=scatterLower(t,0u,nt);let mi=scatterLower(mu,nt,nm);
  let ft=(t-scatterAxes[ti])/(scatterAxes[ti+1u]-scatterAxes[ti]);
  let fm=(mu-scatterAxes[nt+mi])/(scatterAxes[nt+mi+1u]-scatterAxes[nt+mi]);
  let a00=textureLoad(scatterTable,vec2i(i32(ti),i32(mi)),0).r;
  let a10=textureLoad(scatterTable,vec2i(i32(ti+1u),i32(mi)),0).r;
  let a01=textureLoad(scatterTable,vec2i(i32(ti),i32(mi+1u)),0).r;
  let a11=textureLoad(scatterTable,vec2i(i32(ti+1u),i32(mi+1u)),0).r;
  return mix(mix(a00,a10,ft),mix(a01,a11,ft),fm);
}
// END_SCATTER_TABLE_LOOKUP

// Minimal duplicate of the bounded preview's 192-byte Scene ABI, no preview code.
// This builder is for its world-Y-aligned owner field; rotated/sheared producers
// must publish the same defined world-column descriptor through their own adapter.
struct ScatterScene {
  lo:vec4f,extent:vec4f,camera:vec4f,lookAt:vec4f,
  sun:vec4f,U:vec4f,V:vec4f,W:vec4f,
  E:vec4f,sky:vec4f,grid:vec4u,switches:vec4u
}
@group(0) @binding(0) var scatterDensity:texture_3d<f32>;
@group(0) @binding(4) var<uniform> scatterScene:ScatterScene;
@group(1) @binding(2) var scatterBodyOut:texture_storage_2d<r32float,write>;
@group(1) @binding(3) var scatterBankOut:texture_storage_2d<r32float,write>;

// One invocation reads one packed RGBA texel per Y, producing four X/Z columns.
// The zero half-voxel boundary collars of trilinear reconstruction make each
// logical cell's integrated hat weight exactly h, including the first/last Y.
@compute @workgroup_size(8,8) fn buildScatterColumns(@builtin(global_invocation_id) id:vec3u) {
  let outputSize=textureDimensions(scatterBodyOut);
  if(id.x>=outputSize.x||id.y>=(outputSize.y+3u)/4u){return;}
  let ny=textureDimensions(scatterDensity).y-2u;
  let h=scatterScene.extent.y/f32(ny);
  var total=vec4f(0.);
  for(var y=0u;y<ny;y++) {
    total+=textureLoad(scatterDensity,vec3i(i32(id.x+1u),i32(y+1u),i32(id.y+1u)),0);
  }
  total*=h;
  let mu0=abs(scatterScene.sun.y);
  for(var channel=0u;channel<4u;channel++) {
    let z=id.y*4u+channel;
    if(z<outputSize.y) {
      let pixel=vec2i(i32(id.x),i32(z));let body=total[channel];
      textureStore(scatterBodyOut,pixel,vec4f(body,0.,0.,0.));
      textureStore(scatterBankOut,pixel,vec4f(scatterFitA(body,mu0),0.,0.,0.));
    }
  }
}
