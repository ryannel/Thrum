import { Scene } from './scene.js';
import { Gpu } from './gpu.js';
import { halfToFloat } from './reference.js';
const EXPECTED={
 '03':{128:'c0365090769b38b48f33b206a0155f2cd0f34be58ba809999d4901bbd99fb844',256:'42431f9da2d76e253d649cc35802fd1be7b23ddb698e73c6c4247ac3c90a97c9'},
 '08':{128:'cadf6caa98017c9811690f9e311a4bbae648c659a5895b98c0839165856c21a4',256:'4c55318ec1664d40e73a0eed0bcfeefd5563a1b65bb60564f80b3ffbd5fad990'},
};
const SHADERS=['source','field-noise','thermal-shape','normal-stencil','mix','carriers','witness','support','dilate','velocity','noise','mip','curl','optics','light','shadow','air','view','display'];
export async function verifyMaster(bytes,metadata,id){
 const side=metadata.output.dimensions_xyz[0];
 if(![128,256].includes(side)||metadata.output.dimensions_xyz.some(v=>v!==side)||bytes.byteLength!==side**3*2||metadata.output.bytes!==bytes.byteLength)throw Error('Master dimensions/byte count differ from scalarR16F');
 const sha=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),v=>v.toString(16).padStart(2,'0')).join('');
 if(sha!==EXPECTED[id]?.[side]||sha!==metadata.output.sha256)throw Error('Master SHA-256 mismatch');return sha;
}
export function prepareMaster(bytes,metadata,placement){
 const side=metadata.output.dimensions_xyz[0],raw=new Uint16Array(bytes);let sum=0,nonzero=0;
 for(let i=0;i<raw.length;i++){const q=halfToFloat(raw[i]);if(!Number.isFinite(q)||q<0||q>1)throw Error('Master density outside finite[0,1]');sum+=q;if(q>0)nonzero++;}
 if(sum<=0)throw Error('Empty master');
 return {raw,side,sum,nonzero,transform:{scale:4,pitch_m:metadata.output.voxel_pitch_xyz_m.map(v=>v*4),bounds:placement.bounds,
  local_bounds:metadata.output,world_from_local:'world=origin+4*local',origin_world_m:placement.origin_world_m,
  centroid_world_m:placement.centroid_world_m,placement_anchor128_sha256:placement.anchor128_sha256,
  placement_policy:'Exact128worldorigin/target retained across source qualities; no resolution-driven pose change'}};
}
async function fetchAsset(id,quality){
 const manifest=await fetch('assets/master-placements.json').then(r=>{if(!r.ok)throw Error('Master placement unavailable');return r.json();});
 const metadata=await fetch(`assets/cloud-${id}-${quality}.json`).then(r=>{if(!r.ok)throw Error('Master metadata unavailable');return r.json();});
 const bytes=await fetch(`assets/cloud-${id}-${quality}.density.r16f`).then(r=>{if(!r.ok)throw Error('Master binary unavailable');return r.arrayBuffer();});
 const sha=await verifyMaster(bytes,metadata,id),prepared=prepareMaster(bytes,metadata,manifest[id]);return {id,metadata,sha,...prepared};
}
export class FrozenMaster {
 static async create(canvas,progress,id='03',quality=256){
  const asset=await fetchAsset(id,quality);progress('Compiling qualified renderer…');
  const gpu=await Gpu.open(canvas),codes=Object.fromEntries(await Promise.all(SHADERS.map(async name=>[name,await fetch(`shaders/${name}.wgsl`).then(r=>{if(!r.ok)throw Error(`Shader unavailable:${name}`);return r.text();})])));
  codes.source=codes['field-noise']+codes['thermal-shape']+codes.source;codes.mix=codes['field-noise']+codes['thermal-shape']+codes['normal-stencil']+codes.mix;
  const adapter=new FrozenMaster(),scene=new Scene(gpu,codes,0,1);adapter.scene=scene;adapter.asset=asset;
  // Frozen scalar provider: obsolete PDE/rest allocations are1voxel dummies, never stepped.
  scene.dims=[1,1,1];scene.restDims=[1,1,1];scene.traceFeed=false;
  scene.bounds=()=>adapter.asset.transform.bounds;
  scene.target=()=>{const c=adapter.asset.transform.centroid_world_m;return [c[0]+scene.camera.panX,c[1]+scene.camera.panY,c[2]];};
  scene.camera={yaw:.3,pitch:-.05,distance:3000,panX:0,panY:0};
  try{await scene.init(progress);adapter.dummyQ=[...scene.q];await adapter.upload();return adapter;}catch(error){await scene.dispose();throw error;}
 }
 async upload(){
  const s=this.scene,g=s.gpu,d=g.device,a=this.asset;await d.queue.onSubmittedWorkDone();
  const side=a.side,texture=d.createTexture({label:`frozen R16 ${a.id}/${side}`,size:[side,side,side],dimension:'3d',format:'r16float',
   usage:GPUTextureUsage.TEXTURE_BINDING|GPUTextureUsage.COPY_DST|GPUTextureUsage.COPY_SRC});
  const record={label:texture.label,bytes:a.raw.byteLength,format:'r16float',size:[side,side,side],role:'single sampled frozen scalar; current/previous alias'};
  g.allocations.push(record);this.uploadLogicalPeakBytes=Math.max(this.uploadLogicalPeakBytes??0,g.bytes);
  try{d.queue.writeTexture({texture},a.raw,{bytesPerRow:side*2,rowsPerImage:side},[side,side,side]);await d.queue.onSubmittedWorkDone();if(g.errors.length)throw Error(g.errors.join('; '));}
  catch(error){texture.destroy();g.allocations=g.allocations.filter(v=>v!==record);throw error;}
  const old=this.texture,oldRecord=this.textureRecord;s.groups.clear();
  this.texture=texture;this.textureRecord=record;s.q=[texture,texture];s.dims=[side,side,side];s.current=0;
  if(old){old.destroy();this.retiredSourceCount=(this.retiredSourceCount??0)+1;this.retiredSourceBytes=(this.retiredSourceBytes??0)+oldRecord.bytes;g.allocations=g.allocations.filter(v=>v!==oldRecord);}
  s.time=0;s.steps=0;s.lastSourceInput={time:0,wind:0,windIntegral:0,moist:0,dry:0,lift:0,storm:0,frontX:1e6};
 }
 async select(id,quality){const asset=await fetchAsset(id,quality),previous=this.asset,changedId=previous.id!==id;await this.scene.gpu.device.queue.onSubmittedWorkDone();this.asset=asset;try{await this.upload();}catch(error){this.asset=previous;throw error;}if(changedId){this.scene.camera.panX=0;this.scene.camera.panY=0;}}
 describe(){const a=this.asset;return {id:a.id,side:a.side,sha256:a.sha,bytes:a.metadata.output.bytes,metadata:a.metadata,worldTransform:a.transform,cpuInputBytes:a.raw.byteLength,
  sourceFormat:'r16float',currentPreviousAlias:true,sourceChannels:{density:'R',ice:'implicitG=0',age:'notrepresented',birth:'notrepresented',sampledAlpha:'formatdefault1;notage/birth'},
  obsoletePdeRestDimensions:[1,1,1],sourceFrozen:true,sourceTime:0,sourceSteps:0,uploadLogicalPeakBytes:this.uploadLogicalPeakBytes,retiredSourceCount:this.retiredSourceCount??0,retiredSourceBytes:this.retiredSourceBytes??0};}
 async probe(){
  const s=this.scene,g=s.gpu,d=g.device,a=this.asset;await d.queue.onSubmittedWorkDone();
  const buffer=d.createBuffer({label:'requested frozen scalar readback',size:a.raw.byteLength,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  const counts=d.createBuffer({label:'requested scalar sampled-channel check',size:16,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST|GPUBufferUsage.COPY_SRC});
  const countsRead=d.createBuffer({label:'requested scalar sampled-channel readback',size:16,usage:GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST});
  try{
   this.probePipeline??=await g.compute('frozenR16 actual sampled phase defaults',`
    @group(0) @binding(0) var density:texture_3d<f32>;
    @group(0) @binding(1) var<storage,read_write> counts:array<atomic<u32>,4>;
    @compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){let n=textureDimensions(density);if(any(id>=n)){return;}
     let v=textureLoad(density,vec3i(id),0);if(v.y!=0.){atomicAdd(&counts[0],1u);}if(v.z!=0.){atomicAdd(&counts[1],1u);}if(v.w!=1.){atomicAdd(&counts[2],1u);}
     if(any(v!=v)||any(abs(v)>vec4f(65504))||v.x<0.||v.x>1.){atomicAdd(&counts[3],1u);}}
   `);
   const e=d.createCommandEncoder();e.clearBuffer(counts);
   s.compute(e,this.probePipeline,g.group(this.probePipeline,[[0,this.texture.createView()],[1,counts]]),s.dims);
   e.copyTextureToBuffer({texture:this.texture},{buffer,bytesPerRow:a.side*2,rowsPerImage:a.side},s.dims);e.copyBufferToBuffer(counts,0,countsRead,0,16);g.submit(e);
   await Promise.all([buffer.mapAsync(GPUMapMode.READ),countsRead.mapAsync(GPUMapMode.READ)]);
   const data=new Uint16Array(buffer.getMappedRange()),values=new Uint32Array(countsRead.getMappedRange());let differences=0,sum=0;
   for(let i=0;i<a.raw.length;i++){if(data[i]!==a.raw[i])differences++;sum+=halfToFloat(data[i]);}
   const sampled={nonzeroGreenVoxels:values[0],nonzeroBlueVoxels:values[1],alphaNotOneVoxels:values[2],invalidVoxels:values[3]};
   const alias=s.q[0]===s.q[1]&&s.q[0]===this.texture;buffer.unmap();countsRead.unmap();
   return {scope:'Requested exactR16F readback of one frozen texture aliasedbycurrent/previous plusactualshaderG0/B0/A1query; no lifecycle or rendereraccuracy qualification',pass:alias&&differences===0&&sum===a.sum&&Object.values(sampled).every(v=>v===0),
    aliasedSnapshots:alias,differentScalarHalfVoxels:differences,sumLiquid:sum,expectedSum:a.sum,sampled,temporaryGpuReadbackBytes:a.raw.byteLength+32,temporaryResourceRetired:true};
  }finally{buffer.destroy();counts.destroy();countsRead.destroy();}
 }
}
