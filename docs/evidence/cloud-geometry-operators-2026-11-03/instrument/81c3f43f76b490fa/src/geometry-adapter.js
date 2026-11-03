import { FrozenMaster } from './master-adapter.js';
import { halfToFloat } from './reference.js';
async function verified(record,json=false){
 const response=await fetch('assets/'+record.file);if(!response.ok)throw Error('Geometry input unavailable:'+record.file);
 const bytes=await response.arrayBuffer();if(bytes.byteLength!==record.bytes)throw Error('Geometry byte count mismatch:'+record.file);
 const sha=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),v=>v.toString(16).padStart(2,'0')).join('');
 if(sha!==record.sha256)throw Error('Geometry SHA mismatch:'+record.file);return json?JSON.parse(new TextDecoder().decode(bytes)):bytes;
}
export function lookupGeometry(curves,channel,requested,operator,continuous=false){
 const c=curves.channels[channel];if(!c||![0,1,2,3,4].includes(operator)||!Number.isFinite(requested))throw Error('Invalid geometry control');
 const xs=c.exposure_world_m,target=Math.max(0,Math.min(c.full_extinction_exposure_world_m,requested));let lo=0,hi=xs.length-1;
 while(lo+1<hi){const m=(lo+hi)>>1;if(xs[m]<=target)lo=m;else hi=m;}
 const nearest=Math.abs(xs[lo]-target)<=Math.abs(xs[hi]-target)?lo:hi;
 if(!continuous)lo=hi=nearest;
 const fraction=lo===hi?0:(target-xs[lo])/(xs[hi]-xs[lo]);
 const at=key=>c[key][lo]+fraction*(c[key][hi]-c[key][lo]),actual=continuous?target:xs[lo];
 return {preparedGeometry:true,geometryOperator:operator,geometryExposure:actual,geometryRhoThreshold:at('rho_threshold'),geometryAlpha:at('alpha'),
  geometryWidth:curves.reference_taper_halfwidth_world_m,geometryChannels:channel==='noisy',geometryLookup:{channel,index:continuous?null:nearest,bracket:{lowerIndex:lo,upperIndex:hi,lowerExposureWorldM:xs[lo],upperExposureWorldM:xs[hi],fraction},requestedExposure:requested,actualExposureWorldM:actual,
   effectiveOperatorExposureWorldM:operator===0||operator===4?0:actual,targetNativeIntegral:at('target_integral'),referenceNativeIntegral:curves.reference_integral,
   policy:continuous?'Piecewise linear calibrated LUT interpolation; native integral matching approximate between knots, no ray/projected matching claim':'Nearest calibrated LUT row for every operator; no uncontrolled independent density/alpha tuning or between-row matching claim'}};
}
// Declared experimental R16 filtering envelope, not a portable WebGPU accuracy guarantee.
export function halfULP(x){return Math.max(2**-24,2**(Math.floor(Math.log2(Math.max(Math.abs(x),2**-14)))-10));}
export function coefficientEnvelope(raw,dist,M,p,kappa){
 const f32=2**-20,dr=2*halfULP(raw)+f32,dd=8*halfULP(dist/4)+4*f32;
 const dm=p.geometryChannels?.7*2*halfULP((M-.3)/.7)+f32:0;
 const ref=dr+(raw+dr)*(.75/p.geometryWidth)*dd;
 let dq=dr;
 if(p.geometryOperator===1)dq=dr+(raw+dr)*(.75/p.geometryWidth)*(dd+p.geometryExposure*dm);
 if(p.geometryOperator===4)dq=ref;
 if(p.geometryOperator===2)dq=ref+p.geometryRhoThreshold*dm;
 if(p.geometryOperator===3)dq=p.geometryAlpha*ref;
 return {raw:dr,distanceWorldM:dd,multiplier:dm+f32,coefficient:kappa*dq+Math.max(1e-9,kappa*f32)};
}

function validate(curves,metadata){
 if(curves.world_scale!==4||metadata.world_scale!==4||curves.reference_taper_halfwidth_world_m!==32)throw Error('Geometry world/collar contract mismatch');
 for(const [name,c] of Object.entries(curves.channels)){
  const xs=c.exposure_world_m;if(!xs?.length||xs[0]!==0)throw Error('LUT lacks zero exposure');
  for(const key of ['rho_threshold','alpha','target_integral'])if(c[key].length!==xs.length)throw Error('LUT column length mismatch');
  for(let i=0;i<xs.length;i++)if(!Number.isFinite(xs[i])||(i&&xs[i]<=xs[i-1])||c.rho_threshold[i]<0||!Number.isFinite(c.rho_threshold[i])||c.alpha[i]<0||c.alpha[i]>1||!Number.isFinite(c.alpha[i])||c.target_integral[i]<0||!Number.isFinite(c.target_integral[i]))throw Error('Invalid LUT row');
  const minimum=name==='noisy'?.3:1,required=(metadata.distance_max_inside_world_m+32+8)/minimum;
  if(c.full_extinction_exposure_world_m+1e-5<required||Math.abs(xs.at(-1)-c.full_extinction_exposure_world_m)>1e-5)throw Error('LUT endpoint does not certify extinction bound');
 }
}
export function sampleHalf(raw,side,uv){
 const pos=uv.map(v=>v*side-.5),lo=pos.map(Math.floor),f=pos.map((v,i)=>v-lo[i]);let out=0;
 for(let z=0;z<2;z++)for(let y=0;y<2;y++)for(let x=0;x<2;x++){
  const ix=Math.max(0,Math.min(side-1,lo[0]+x)),iy=Math.max(0,Math.min(side-1,lo[1]+y)),iz=Math.max(0,Math.min(side-1,lo[2]+z));
  out+=halfToFloat(raw[(iz*side+iy)*side+ix])*(x?f[0]:1-f[0])*(y?f[1]:1-f[1])*(z?f[2]:1-f[2]);
 }return out;
}
const smooth=(a,b,x)=>{const t=Math.max(0,Math.min(1,(x-a)/(b-a)));return t*t*(3-2*t);};
export function referenceAmount(q,distance,M,p){
 const ref=q*smooth(-p.geometryWidth,p.geometryWidth,distance);
 if(p.geometryOperator===0)return q;if(p.geometryOperator===4)return ref;
 if(p.geometryOperator===1)return q*smooth(-p.geometryWidth,p.geometryWidth,distance-p.geometryExposure*M);
 if(p.geometryOperator===2)return Math.max(ref-p.geometryRhoThreshold*M,0);
 return ref*p.geometryAlpha;
}
export class GeometryMaster {
 static async create(canvas,progress,quality=256,iso=.001){
  const index=await fetch('assets/geometry-index.json').then(r=>{if(!r.ok)throw Error('Awaiting frozen geometry inputs');return r.json();});
  if(!index.ready)throw Error('Awaiting frozen geometry inputs');
  const g=new GeometryMaster();g.index=index;g.pending=await g.load(quality,iso);g.base=await FrozenMaster.create(canvas,progress,'03',quality);g.scene=g.base.scene;
  try{await g.install(g.pending);g.pending=null;return g;}catch(error){await g.scene.dispose();throw error;}
 }
 get asset(){return this.base.asset;}
 async load(quality,iso){
  const record=this.index.datasets.find(v=>v.quality===quality&&v.iso_density===iso);if(!record)throw Error('No frozen geometry dataset');
  const [metadata,curves,bytes]=await Promise.all([verified(record.distanceMetadata,true),verified(record.curves,true),verified(record.distance)]);
  if(curves.distance_sha256!==record.distance.sha256||metadata.output.sha256!==record.distance.sha256||curves.density_source_sha256!==metadata.source_sha256||curves.quality!==quality||curves.iso_density!==iso||curves.noise_sha256!==this.index.noise.sha256)throw Error('Geometry/LUT dependencies differ');
  validate(curves,metadata);const distance=new Uint16Array(bytes);
  if(distance.length!==quality**3)throw Error('Distance dimensions differ');
  for(const bits of distance)if(!Number.isFinite(halfToFloat(bits)))throw Error('Nonfinite distance');
  if(!this.noiseRaw){this.noiseRaw=new Uint16Array(await verified(this.index.noise));for(const bits of this.noiseRaw){const n=halfToFloat(bits);if(!Number.isFinite(n)||n<0||n>1)throw Error('Noise outside[0,1]');}}
  return {record,metadata,curves,distance,quality,iso};
 }
 async install(data){
  const s=this.scene,g=s.gpu,d=g.device;await d.queue.onSubmittedWorkDone();
  if(this.asset.sha!==data.metadata.source_sha256)throw Error('Distance source differs from active raw master');
  const created=[];
  const upload=(label,raw,side)=>{
   const texture=d.createTexture({label,size:[side,side,side],dimension:'3d',format:'r16float',usage:GPUTextureUsage.TEXTURE_BINDING|GPUTextureUsage.COPY_DST|GPUTextureUsage.COPY_SRC});
   const record={label,bytes:raw.byteLength,size:[side,side,side],format:'r16float',role:'manual geometry POC read-only input'};g.allocations.push(record);this.geometryUploadLogicalPeakBytes=Math.max(this.geometryUploadLogicalPeakBytes??0,g.bytes);created.push({texture,record});
   d.queue.writeTexture({texture},raw,{bytesPerRow:side*2,rowsPerImage:side},[side,side,side]);return {texture,record};
  };
  try{
   const distance=upload('manual SDF '+data.quality+'/'+data.iso,data.distance,data.quality);
   const noise=this.noiseTexture??upload('manual fixed channels32',this.noiseRaw,32);
   await d.queue.onSubmittedWorkDone();if(g.errors.length)throw Error(g.errors.join('; '));
   s.groups.clear();const old=this.distanceTexture;this.distanceTexture=distance;this.noiseTexture=noise;this.data=data;
   s.geometryDistance=distance.texture;s.geometryNoise=noise.texture;
   if(old){old.texture.destroy();this.retiredDistanceCount=(this.retiredDistanceCount??0)+1;g.allocations=g.allocations.filter(v=>v!==old.record);}
  }catch(error){for(const v of created){v.texture.destroy();g.allocations=g.allocations.filter(r=>r!==v.record);}throw error;}
 }
 async selectGeometry(quality,iso){const data=await this.load(quality,iso);if(quality!==this.asset.side)await this.base.select('03',quality);await this.install(data);}
 parameters(operator,requested,channel,continuous=false){this.lastParameters=lookupGeometry(this.data.curves,channel,requested,operator,continuous);return this.lastParameters;}
 maxExposure(channel){return this.data.curves.channels[channel].full_extinction_exposure_world_m;}
 describe(){const {sourceFrozen,sourceTime,sourceSteps,...rawMaster}=this.base.describe();return {...rawMaster,underlyingMasterFrozen:sourceFrozen,underlyingMasterTime:sourceTime,underlyingMasterPdeSteps:sourceSteps,manualGeometry:{...this.lastParameters,isoDensity:this.data.iso,distance:this.data.record.distance,curves:this.data.record.curves,noise:this.index.noise,
  geometryTextureBytes:{distance:this.data.distance.byteLength,noise:this.noiseRaw.byteLength},geometryUploadLogicalPeakBytes:this.geometryUploadLogicalPeakBytes,retiredDistanceCount:this.retiredDistanceCount??0,distanceMetadata:this.data.metadata,referenceScope:this.data.curves.reference_scope,nativeMatchingScope:this.data.curves.sampling,
  geometryOperatorScope:'Manual held field or analytic exposure fixture; direct coefficient evaluation is not a production cost claim, exact continuous SDF or constant noisy-front speed'}};}
 async probe(ui,weather){
  const upload=await this.base.probe(),s=this.scene,g=s.gpu,d=g.device,bytes=32**3*16;
  this.queryPipeline??=await g.compute('manual shared coefficient32cube observer',s.codes.optics+`
   @group(0) @binding(15) var<storage,read_write> output:array<vec4f>;
   @compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){if(any(id>=vec3u(32))){return;}
    let uv=(vec3f(id)+.5)/32.;let x=mix(p.bmin.xyz,p.bmax.xyz,uv);let q=rawQ(x).xy;let c=coefficients(x,1.);
    output[(id.z*32u+id.y)*32u+id.x]=vec4f(q.x+q.y,c.x+c.y,4.*textureSampleLevel(geometryDistance,linearClamp,uv,0.).x,geometryMultiplier(uv));}`);
  const uniform=d.createBuffer({label:'manual coefficient observer uniform',size:176,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});
  const result=d.createBuffer({label:'manual coefficient observer result',size:bytes,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  const read=d.createBuffer({label:'manual coefficient observer read',size:bytes,usage:GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST});
  try{
   s.write(uniform,weather,ui,null,0);const q=s.q[0],rest=s.rest[s.currentRest];
   const group=g.group(this.queryPipeline,[[0,uniform],[1,q.createView()],[2,s.noise.createView()],[3,s.clamp],[4,rest.createView()],[10,q.createView()],[11,rest.createView()],[14,s.repeat],[15,result],[16,s.restB[s.currentRest].createView()],[17,s.restB[s.currentRest].createView()],[18,s.geometryDistance.createView()],[19,s.geometryNoise.createView()]]);
   const e=d.createCommandEncoder();s.compute(e,this.queryPipeline,group,[32,32,32]);e.copyBufferToBuffer(result,0,read,0,bytes);g.submit(e);await read.mapAsync(GPUMapMode.READ);
   const a=new Float32Array(read.getMappedRange());let invalid=0,negative=0,overRaw=0,active=0,maxRaw=0,maxDistance=0,maxM=0,maxCoefficient=0;const samples=[],normalized={raw:0,distanceWorldM:0,multiplier:0,coefficient:0},toleranceMax={raw:0,distanceWorldM:0,multiplier:0,coefficient:0};
   const kappa=.0045*(ui.extinctionScale??1);
   for(let i=0;i<32**3;i++){
    const uv=[(i%32+.5)/32,(Math.floor(i/32)%32+.5)/32,(Math.floor(i/1024)+.5)/32],v=a.subarray(i*4,i*4+4);
    if(!Array.from(v).every(Number.isFinite))invalid++;if(v[1]<0)negative++;if(v[1]>v[0]*kappa+1e-6)overRaw++;if(v[1]>0)active++;
    const raw=sampleHalf(this.asset.raw,this.asset.side,uv),dist=4*sampleHalf(this.data.distance,this.data.quality,uv),M=ui.geometryChannels ? .3+.7*sampleHalf(this.noiseRaw,32,uv) : 1;
    const expected=referenceAmount(raw,dist,M,ui)*kappa;maxRaw=Math.max(maxRaw,Math.abs(raw-v[0]));maxDistance=Math.max(maxDistance,Math.abs(dist-v[2]));maxM=Math.max(maxM,Math.abs(M-v[3]));maxCoefficient=Math.max(maxCoefficient,Math.abs(expected-v[1]));
    const tolerance=coefficientEnvelope(raw,dist,M,ui,kappa),errors={raw:Math.abs(raw-v[0]),distanceWorldM:Math.abs(dist-v[2]),multiplier:Math.abs(M-v[3]),coefficient:Math.abs(expected-v[1])};for(const key of Object.keys(normalized)){normalized[key]=Math.max(normalized[key],errors[key]/tolerance[key]);toleranceMax[key]=Math.max(toleranceMax[key],tolerance[key]);}
    if(v[0]>0&&samples.length<8)samples.push({uv,raw:v[0],sigma:v[1],distanceWorldM:v[2],M:v[3],cpuSigma:expected});
   }
   read.unmap();const endpoint=ui.geometryExposure>=this.maxExposure(ui.geometryChannels?'noisy':'uniform')-1e-5&&[1,2,3].includes(ui.geometryOperator);
   return {pass:upload.pass&&!invalid&&!negative&&!overRaw&&Object.values(normalized).every(x=>Number.isFinite(x)&&x<=1)&&(!endpoint||active===0),upload,coefficientObserver:{scope:'32768 fixed world-lattice points, shared actual coefficient function; CPU trilinear agreement gated by declared R16 filtering envelope, not portable total qualification or native geometry/topology/projected matching proof',numericalEnvelope:{maxNormalizedErrors:normalized,maxAbsoluteTolerances:toleranceMax,pass:Object.values(normalized).every(x=>Number.isFinite(x)&&x<=1),policy:'2 half ULP sample envelope plus f32 margin; smoothstep Lipschitz .75/width propagates density/distance/noise errors; experimental admission bound, not an external format accuracy specification'},operator:ui.geometryOperator,samples:32**3,invalidSamples:invalid,negativeSamples:negative,aboveRawExtinctionSamples:overRaw,activeCoefficientSamples:active,endpointExpectedZero:endpoint,
    maxAbsRawDifference:maxRaw,maxAbsDistanceWorldDifference:maxDistance,maxAbsMultiplierDifference:maxM,maxAbsCoefficientDifference:maxCoefficient,representativeSamples:samples,temporaryBytes:bytes*2+176,temporaryResourcesRetired:true}};
  }finally{uniform.destroy();result.destroy();read.destroy();}
 }
}
