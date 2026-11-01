// Readbacks exercise the same closures consumed by the actual graph.
struct Query {origin:vec4<f32>,ray:vec4<f32>,options:vec4<f32>}
@group(1) @binding(0) var<storage,read> queries:array<Query>;
@group(1) @binding(1) var<storage,read_write> answers:array<vec4<f32>>;
@compute @workgroup_size(1) fn probe(@builtin(global_invocation_id) id:vec3<u32>){
 let q=queries[id.x];let p=q.origin.xyz;let base=id.x*8u;let tau=sunDepth(p);
 answers[base+3u]=vec4<f32>(incident(p,tau.x),geometryVisibility(p,scene.sun.xyz));
 answers[base+4u]=vec4<f32>(tau,sigmaCloud(p,0.));answers[base+5u]=vec4<f32>(cloudAmbient(p),fogTau(p,scene.sun.xyz,FAR));answers[base+7u]=vec4<f32>(airTau(p,scene.sun.xyz,FAR),0.);
 if(q.options.z<.5){return;}
 let ray=normalize(q.ray.xyz);let hit=sceneHit(p,ray,q.options.w>.5);let distance=min(hit.t,q.options.x);let v=transport(p,ray,distance,q.options.y>.5);var terminalValue=terminal(p,ray,hit);var valid=v.valid;
 if(hit.kind==3u&&distance==hit.t){let rd=reflectedDirection(ray);let o=p+ray*hit.t+rd*.001;let rh=sceneHit(o,rd,false);let rv=transport(o,rd,rh.t,true);terminalValue=rv.S+rv.T*terminal(o,rd,rh);valid=valid&&rv.valid;}
 answers[base]=vec4<f32>(v.S,distance);answers[base+1u]=vec4<f32>(v.T,f32(hit.kind));answers[base+2u]=vec4<f32>(terminalValue,select(0.,1.,valid));answers[base+6u]=vec4<f32>(v.S+v.T*terminalValue,0.);
}
