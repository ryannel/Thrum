@group(1) @binding(0) var outPrefix:texture_storage_3d<rgba16float,write>;
@group(1) @binding(1) var outAmbient:texture_storage_3d<rgba16float,write>;
@compute @workgroup_size(8,8) fn buildPrefix(@builtin(global_invocation_id) id:vec3u) {
  if(any(id.xy>=vec2u(66))) {return;}
  let xy=(vec2f(id.xy)-.5)/64.-.5;
  let centre=scene.lo.xyz+scene.extent.xyz*.5;
  let start=centre+scene.U.xyz*(xy.x*2.*scene.U.w)+scene.V.xyz*(xy.y*2.*scene.V.w)-scene.W.xyz*scene.W.w;
  let h=2.*scene.W.w/64.;var tau=0.;
  textureStore(outPrefix,vec3i(vec2i(id.xy),0),vec4f(0.));
  textureStore(outPrefix,vec3i(vec2i(id.xy),1),vec4f(0.));
  for(var z=0u;z<64u;z++) {
    tau+=sigmaAt(start+scene.W.xyz*((f32(z)+.5)*h))*h;
    textureStore(outPrefix,vec3i(vec3u(id.xy,z+2u)),vec4f(tau,0.,0.,0.));
  }
}
@compute @workgroup_size(4,4,4) fn buildAmbient(@builtin(global_invocation_id) id:vec3u) {
  if(any(id>=vec3u(8))) {return;}
  let p=scene.lo.xyz+(vec3f(id)+.5)/8.*scene.extent.xyz;
  let directions=array<vec3f,5>(vec3f(1.,0.,0.),vec3f(-1.,0.,0.),vec3f(0.,0.,1.),vec3f(0.,0.,-1.),vec3f(0.,1.,0.));
  var T=0.;for(var i=0u;i<5u;i++){T+=exp(-tauDirect(p,directions[i],64u));}
  textureStore(outAmbient,vec3i(id),vec4f(scene.sky.rgb*T/6.,0.));
}
