// Requested GPU counters only: no voxel readback/traversal in the live frame path.
export async function observeMaterial(scene,ui){
 if(scene.family!==1)return null;
 const g=scene.gpu,d=g.device;
 if(!scene.materialObserver){
  const code=`@group(0) @binding(0) var q:texture_3d<f32>;
  @group(0) @binding(1) var a:texture_3d<f32>;@group(0) @binding(2) var b:texture_3d<f32>;
  @group(0) @binding(3) var<storage,read_write> c:array<atomic<u32>,16>;
  @compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){
   let n=textureDimensions(a);if(any(id>=n)){return;}
   let source=textureLoad(q,vec3i(id*2u+1u),0);if(source.x+source.y<.015){return;}
   let ma=textureLoad(a,vec3i(id),0).w;let mb=textureLoad(b,vec3i(id),0).w;
   if(ma!=ma||mb!=mb){atomicAdd(&c[9],1u);return;}
   let la=clamp(log2(max(64.*ma*64./3200.,1.)),0.,6.);
   let lb=clamp(log2(max(64.*mb*64./3200.,1.)),0.,6.);
   atomicAdd(&c[0],1u);atomicAdd(&c[1],u32(round(la*1000.)));atomicAdd(&c[2],u32(round(lb*1000.)));
   atomicMax(&c[3],u32(round(ma*100.)));atomicMax(&c[4],u32(round(mb*100.)));
   atomicAdd(&c[5],u32(round(ma*100.)));atomicAdd(&c[6],u32(round(mb*100.)));
   if(la>=3.){atomicAdd(&c[7],1u);}if(lb>=3.){atomicAdd(&c[8],1u);}
  }`;
  scene.materialObserver={pipeline:await g.compute('requested material deformation observer',code),counts:g.buffer('material observer counters',64,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST|GPUBufferUsage.COPY_SRC),read:g.buffer('material observer readback',64,GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST)};
 }
 const {pipeline,counts,read}=scene.materialObserver;
 const e=d.createCommandEncoder();e.clearBuffer(counts);
 scene.compute(e,pipeline,g.group(pipeline,[[0,scene.q[scene.current].createView()],[1,scene.rest[scene.currentRest].createView()],[2,scene.restB[scene.currentRest].createView()],[3,counts]]),scene.restDims);
 e.copyBufferToBuffer(counts,0,read,0,64);g.submit(e);await read.mapAsync(GPUMapMode.READ);const c=Array.from(new Uint32Array(read.getMappedRange().slice(0)));read.unmap();
 const tick=scene.steps,phase=tick%256,w=phase===0?0:phase===128?1:Math.sin(Math.PI*phase/256)**2;
 const occupied=c[0],meanLod=occupied? [c[1]/1000/occupied,c[2]/1000/occupied]:[0,0];
 return {scope:'Requested current-tick observer; one q voxel per carrier-cell center; potential filtered advected-map LOD regardless of active world/filter choices; sampled maximum-gradient metric, not singular-value or visible-art proof',sourceTick:tick,dualRest:ui.dualRest,chartAgeSeconds:ui.dualRest?[tick%256*.25,tick<128?tick*.25:(tick+128)%256*.25]:[tick*.25,null],chartWeights:ui.dualRest?[w,1-w]:[1,0],renderSettings:{materialSpace:ui.materialSpace,materialFilter:ui.materialFilter},occupiedCarrierCells:occupied,potentialFilteredCarrierMeanLod:meanLod,maxGradientMetric:[c[3]/100,c[4]/100],meanGradientMetric:occupied?[c[5]/100/occupied,c[6]/100/occupied]:[0,0],potentialFilteredCarrierWeightedMeanLod:ui.dualRest?w*meanLod[0]+(1-w)*meanLod[1]:meanLod[0],potentialFilteredCarrierLodAtLeast3Counts:[c[7],c[8]],nonfinite:c[9]};
}
