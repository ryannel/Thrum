// Conservative work mask for the dense oracle. This is not sparse storage.
struct Params { camera:vec4f,focus:vec4f,sun:vec4f,bmin:vec4f,bmax:vec4f,weather:vec4f,control:vec4f,front:vec4f,image:vec4f,domain:vec4f };
struct Emitter { location:vec4f,timing:vec4f };
@group(0) @binding(0) var<uniform> p:Params;
@group(0) @binding(1) var q:texture_3d<f32>;
@group(0) @binding(2) var<storage,read> emitters:array<Emitter,32>;
@group(0) @binding(3) var<storage,read_write> support:array<u32>;
var<workgroup> occupied:atomic<u32>;
@compute @workgroup_size(4,4,4) fn main(@builtin(workgroup_id) brick:vec3u,@builtin(local_invocation_id) local:vec3u) {
  let n=textureDimensions(q);let bricks=(n+7u)/8u;
  if(all(local==vec3u(0))){atomicStore(&occupied,0u);}
  workgroupBarrier();
  for(var k=0u;k<8u;k++) {
    let cell=brick*8u+local+vec3u(k&1u,(k>>1u)&1u,(k>>2u)&1u)*4u;
    if(all(cell<n)){let v=textureLoad(q,vec3i(cell),0);if(v.x+v.y>0.){atomicStore(&occupied,1u);}}
  }
  workgroupBarrier();
  if(any(local!=vec3u(0))){return;}
  var enabled=atomicLoad(&occupied);
  let frontal=u32(p.control.w)==1u&&p.front.z>3.5;
  // Legacy non-Cu profiles retain dense work; profile4 has exact finite base support.
  if(u32(p.control.w)!=0u&&!frontal){enabled=1u;}
  if(p.control.x>0.&&p.control.z>0.) {
    let lo=mix(p.bmin.xyz,p.bmax.xyz,vec3f(brick*8u)/vec3f(n));
    let hi=mix(p.bmin.xyz,p.bmax.xyz,vec3f(min((brick+1u)*8u,n))/vec3f(n));
    // Even32 overlapping maximum-rate pulses beyond900m contribute <1e-9 per tick,
    // below half the minimum RGBA16F subnormal. XZ bounds enclose the compact kernel.
    if((!frontal&&lo.y<=2120.&&hi.y>=320.)||(frontal&&lo.y<=2440.&&hi.y>=1050.)) {
      for(var i=0u;i<32u;i++) {
        let e=emitters[i];let radius=e.location.z*1.25*select(1.,2.,frontal);
        let center=e.location.xy*select(1.,2.,frontal)+select(vec2f(0),vec2f(p.weather.w,p.weather.x*1.1),frontal);
        let age=p.weather.x-e.timing.x;
        if(e.location.w>.5&&(!frontal||(age>0.&&age<22.))&&all(hi.xz>=center-vec2f(radius))&&all(lo.xz<=center+vec2f(radius))){enabled=1u;}
      }
    }
  }
  support[(brick.z*bricks.y+brick.y)*bricks.x+brick.x]=enabled;
}
