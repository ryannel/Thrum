import { floatToHalf, halfToFloat } from './reference.js';

// Independent scalar reference for the six-point, unmixed-front contract.
// Neighbours are ordered +x,-x,+y,-y,+z,-z.
export function applyFrontMotionCpu(center, neighbours, pitch, dt, kappa, growth, erosion) {
  const total = (v) => Math.max(0, v[0] + v[1]);
  const original = total(center);
  let outward = 0, inward = 0;
  for (let axis = 0; axis < 3; axis++) {
    const above = total(neighbours[axis * 2]);
    const below = total(neighbours[axis * 2 + 1]);
    outward += (Math.max(above - original, below - original, 0) / pitch[axis]) ** 2;
    inward += (Math.max(original - above, original - below, 0) / pitch[axis]) ** 2;
  }
  const growthDelta = dt * Math.max(growth, 0) * Math.sqrt(outward);
  const erosionDelta = dt * Math.max(erosion, 0) * Math.sqrt(inward);
  const q = [center[0], center[1]];
  let moment = original * center[2], birth = center[3];
  for (let axis = 0; axis < 3; axis++) {
    const weight = dt * kappa / (pitch[axis] ** 2);
    for (const neighbour of neighbours.slice(axis * 2, axis * 2 + 2)) {
      q[0] += weight * (neighbour[0] - center[0]);
      q[1] += weight * (neighbour[1] - center[1]);
      moment += weight * (total(neighbour) * neighbour[2] - original * center[2]);
      birth += weight * (neighbour[3] - center[3]);
    }
  }
  q[0] = Math.max(q[0], 0); q[1] = Math.max(q[1], 0);
  const mixedTotal = q[0] + q[1];
  if (mixedTotal > 0) {
    q[0] -= Math.min(q[0], erosionDelta * q[0] / mixedTotal);
    q[1] -= Math.min(q[1], .08 * erosionDelta * q[1] / mixedTotal);
  }
  const survivingTotal = q[0] + q[1];
  const survival = mixedTotal > 1e-12 ? survivingTotal / mixedTotal : 0;
  const addition = Math.min(growthDelta, Math.max(1 - q[0], 0));
  q[0] += addition;
  return [q[0], q[1], q[0] + q[1] > 1e-12 ? Math.max(moment, 0) * survival / (q[0] + q[1]) : 0,
    Math.max(birth, 0) * survival + addition];
}

const shaderTail = `
struct NormalParams { pitchDt:vec4f, rates:vec4f };
@group(0) @binding(0) var<uniform> p:NormalParams;
@group(0) @binding(1) var source:texture_3d<f32>;
@group(0) @binding(2) var result:texture_storage_3d<rgba16float,write>;
@compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u) {
  let dims=textureDimensions(result);if(any(id>=dims)){return;}
  let i=vec3i(id);let hi=vec3i(dims)-1;
  var n:array<vec4f,6>;
  n[0]=textureLoad(source,clamp(i+vec3i(1,0,0),vec3i(0),hi),0);
  n[1]=textureLoad(source,clamp(i-vec3i(1,0,0),vec3i(0),hi),0);
  n[2]=textureLoad(source,clamp(i+vec3i(0,1,0),vec3i(0),hi),0);
  n[3]=textureLoad(source,clamp(i-vec3i(0,1,0),vec3i(0),hi),0);
  n[4]=textureLoad(source,clamp(i+vec3i(0,0,1),vec3i(0),hi),0);
  n[5]=textureLoad(source,clamp(i-vec3i(0,0,1),vec3i(0),hi),0);
  textureStore(result,id,applyFrontMotion(textureLoad(source,i,0),n,p.pitchDt.xyz,p.pitchDt.w,p.rates.x,p.rates.y,p.rates.z));
}`;

function at(values, n, x, y, z) {
  const lo = Math.max(0, Math.min(n - 1, x)), mid = Math.max(0, Math.min(n - 1, y)), hi = Math.max(0, Math.min(n - 1, z));
  return values.slice(((hi * n + mid) * n + lo) * 4, ((hi * n + mid) * n + lo) * 4 + 4);
}

function trilinear(values, n, point) {
  const base=point.map(v=>Math.floor(v)), f=point.map((v,i)=>v-base[i]);
  let q=0;
  for(let z=0;z<2;z++)for(let y=0;y<2;y++)for(let x=0;x<2;x++) {
    const w=(x?f[0]:1-f[0])*(y?f[1]:1-f[1])*(z?f[2]:1-f[2]);
    const a=at(values,n,base[0]+x,base[1]+y,base[2]+z);
    q+=w*(a[0]+a[1]);
  }
  return q;
}

function isoRadius(values,n,pitch,direction) {
  const center=(n-1)*.5;
  const sample=(r)=>trilinear(values,n,direction.map(v=>center+v*r/pitch));
  let inside=0,outside=0;
  for(let r=0;r<2500;r+=pitch*.25){if(sample(r)<.5){outside=r;break;}inside=r;}
  if(outside===0)return NaN;
  for(let k=0;k<18;k++){const mid=(inside+outside)*.5;if(sample(mid)>=.5)inside=mid;else outside=mid;}
  return (inside+outside)*.5;
}

// Observer-only GPU fixture. Scene must load the pure shader as codes['normal-stencil'].
export async function runNormalReference(scene) {
  const helper=scene.codes['normal-stencil'];
  if(!helper)throw Error('normal-stencil.wgsl is not loaded');
  const g=scene.gpu,d=g.device;
  const pipeline=await g.compute('isolated frontal-normal reference',helper+shaderTail);
  const uniform=g.buffer('normal oracle uniform',32,GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST);
  const params=new Float32Array(8);
  async function fixture(n,initial,pitch,dt,kappa,growth,erosion,steps=1) {
    const textures=[g.texture('normal oracle ping',[n,n,n]),g.texture('normal oracle pong',[n,n,n])];
    const raw=Uint16Array.from(initial,floatToHalf);
    d.queue.writeTexture({texture:textures[0]},raw,{bytesPerRow:n*8,rowsPerImage:n},[n,n,n]);
    params.set([pitch[0],pitch[1],pitch[2],dt,kappa,growth,erosion,0]);
    d.queue.writeBuffer(uniform,0,params);
    const groups=[0,1].map(i=>g.group(pipeline,[[0,uniform],[1,textures[i].createView()],[2,textures[1-i].createView()]]));
    const e=d.createCommandEncoder();
    for(let step=0;step<steps;step++)scene.compute(e,pipeline,groups[step&1],[n,n,n]);
    const output=textures[steps&1];
    const stride=Math.ceil(n*8/256)*256;
    const read=g.buffer('normal oracle read',stride*n*n,GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST);
    e.copyTextureToBuffer({texture:output},{buffer:read,bytesPerRow:stride,rowsPerImage:n},[n,n,n]);
    g.submit(e);await read.mapAsync(GPUMapMode.READ);
    const bytes=new Uint16Array(read.getMappedRange().slice(0));read.unmap();
    const values=new Float32Array(n*n*n*4),row=stride/2;
    for(let z=0;z<n;z++)for(let y=0;y<n;y++)for(let x=0;x<n;x++)for(let c=0;c<4;c++)
      values[((z*n+y)*n+x)*4+c]=halfToFloat(bytes[(z*n+y)*row+x*4+c]);
    read.destroy();textures.forEach(t=>t.destroy());
    return {values,input:Float32Array.from(raw,halfToFloat)};
  }
  const tiny=8,tinyInput=new Float32Array(tiny**3*4),small=[];
  function fill(fn){for(let z=0;z<tiny;z++)for(let y=0;y<tiny;y++)for(let x=0;x<tiny;x++)tinyInput.set(fn(x,y,z),((z*tiny+y)*tiny+x)*4);}
  for(const [name,fn,pitch,rate] of [
    ['vacuum',()=>[0,0,0,0],[100,100,100],[1000,20,12]],
    ['constant',()=>[.4,.2,18,.1],[100,100,100],[1000,20,12]],
    ['ramp',(x)=>[(x+.5)*.1,.05,10+x,.02*x],[100,100,100],[0,20,0]],
    ['phaseCapacity',(x,y,z)=>x===4&&y===4&&z===4?[.98,.3,100,.2]:[1,1,20,.4],[10,10,10],[0,40,0]],
    ['erosion',(x,y,z)=>x===4&&y===4&&z===4?[.4,.4,100,.3]:[0,0,0,0],[10,10,10],[0,0,20]],
  ]) {
    fill(fn);const {values,input}=await fixture(tiny,tinyInput,pitch,.25,...rate);
    let maxError=0;const channelTolerance=[.002,.002,.15,.002];
    const atCenter=((4*tiny+4)*tiny+4)*4;
    for(let z=1;z<tiny-1;z++)for(let y=1;y<tiny-1;y++)for(let x=1;x<tiny-1;x++){
      const i=((z*tiny+y)*tiny+x)*4,c=Array.from(input.slice(i,i+4));
      const neighbours=[[1,0,0],[-1,0,0],[0,1,0],[0,-1,0],[0,0,1],[0,0,-1]].map(([dx,dy,dz])=>Array.from(at(input,tiny,x+dx,y+dy,z+dz)));
      const expected=applyFrontMotionCpu(c,neighbours,pitch,.25,...rate);
      for(let channel=0;channel<4;channel++)maxError=Math.max(maxError,Math.abs(values[i+channel]-expected[channel])/channelTolerance[channel]);
    }
    small.push({name,maxError,tolerance:1,channelTolerance,center:Array.from(values.slice(atCenter,atCenter+4)),scope:'All interior voxels versus independent CPU point oracle; errors normalized by declared binary16 channel tolerances'});
  }
  const balls=[];
  for(const n of [32,64]){
    const pitch=6000/n,initial=new Float32Array(n**3*4),middle=(n-1)*.5;
    for(let z=0;z<n;z++)for(let y=0;y<n;y++)for(let x=0;x<n;x++){
      const r=Math.hypot(x-middle,y-middle,z-middle)*pitch;
      initial[((z*n+y)*n+x)*4]=Math.max(0,Math.min(1,(1200-r)/400));
    }
    const input=Float32Array.from(Uint16Array.from(initial,floatToHalf),halfToFloat);
    const initialAxes=[isoRadius(input,n,pitch,[1,0,0]),isoRadius(input,n,pitch,[1,1,1].map(v=>v/Math.sqrt(3)))];
    for(const sign of [1,-1]){
      const {values}=await fixture(n,initial,[pitch,pitch,pitch],.25,0,sign>0?20:0,sign<0?20:0,40);
      const finalAxes=[isoRadius(values,n,pitch,[1,0,0]),isoRadius(values,n,pitch,[1,1,1].map(v=>v/Math.sqrt(3)))];
      const displacement=finalAxes.map((r,i)=>r-initialAxes[i]);
      const errors=displacement.map(d=>Math.abs(d-sign*200));
      balls.push({grid:n,pitch,sign,initialRadius:initialAxes,finalRadius:finalAxes,displacement,
        expectedDisplacement:sign*200,maxError:Math.max(...errors),tolerance:pitch,
        scope:'Actual GPU 40x0.25 s ball-front motion; radius at q=.5 along axis and body diagonal, trilinear readback'});
    }
  }
  uniform.destroy();
  return {
    normalStencil:{maxError:Math.max(...small.map(c=>c.maxError)),tolerance:1,cases:small,
      scope:'Standalone GPU producer versus independent CPU stencil; no runtime thermal/source bindings'},
    normalBallFront:{maxError:Math.max(...balls.map(c=>c.maxError/c.tolerance)),tolerance:1,cases:balls,
      scope:'Measured axial/diagonal q=.5 displacement for growth and erosion at 32^3/64^3; each case normalized by one grid pitch'},
  };
}
