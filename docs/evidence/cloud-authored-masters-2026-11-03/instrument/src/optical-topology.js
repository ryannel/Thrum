import { analyzeDensity } from './morphology.js';
import { halfToFloat } from './reference.js';
// Paused/busy observer borrows retired reverse scratch and the existing readback/value arrays.
// No source authority/history texture is modified; the next reverse pass overwrites its scratch.
export async function observeOpticalTopology(scene,ui,feedRoot,queryWeather){
 const g=scene.gpu,d=g.device;
 if(!scene.opticalTopologyPipeline){
  const code=scene.codes.optics+`@group(0) @binding(15) var out:texture_storage_3d<rgba16float,write>;
  @compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){let n=textureDimensions(out);if(any(id>=n)){return;}
   let x=mix(p.bmin.xyz,p.bmax.xyz,(vec3f(id)+.5)/vec3f(n));let c=coefficients(x,1.);textureStore(out,id,vec4f(c/.0045,0,0));}`;
  scene.opticalTopologyPipeline=await g.compute('requested full shaped optical topology',code);
  scene.opticalTopologyUniform=g.buffer('optical topology uniform',176,GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST);
 }
 scene.write(scene.opticalTopologyUniform,queryWeather,{...ui,alpha:1},null,scene.time);
 const q=scene.q[scene.current],a=scene.rest[scene.currentRest],b=scene.restB[scene.currentRest];
 const group=g.group(scene.opticalTopologyPipeline,[[0,scene.opticalTopologyUniform],[1,q.createView()],[2,scene.noise.createView()],[3,scene.clamp],[4,a.createView()],[10,q.createView()],[11,a.createView()],[14,scene.repeat],[15,scene.rev.createView()],[16,b.createView()],[17,b.createView()]]);
 const e=d.createCommandEncoder();scene.compute(e,scene.opticalTopologyPipeline,group,scene.dims);
 e.copyTextureToBuffer({texture:scene.rev},{buffer:scene.morphologyRead,bytesPerRow:scene.dims[0]*8,rowsPerImage:scene.dims[1]},scene.dims);g.submit(e);
 await scene.morphologyRead.mapAsync(GPUMapMode.READ);const raw=new Uint16Array(scene.morphologyRead.getMappedRange());
 for(let i=0;i<scene.morphologyValues.length;i++)scene.morphologyValues[i]=halfToFloat(raw[i*4])+halfToFloat(raw[i*4+1]);
 scene.morphologyRead.unmap();
 const samples=[];
 for(const threshold of [.02,.08,.2]){
  const m=analyzeDensity(scene.morphologyValues,scene.dims,scene.bounds(),ui.thermals?scene.thermalDriver.data:[],scene.morphologyScratch,{threshold,feedRoot,coreRadialScale:scene.thermalDriver.frontMorph?.5:1});
  samples.push({thresholdSigmaPerMeter:threshold*.0045,components:m.components,substantialComponents:m.substantialComponents,largest:m.largest,heads:m.heads.map(h=>({id:h.id,position:h.position,centerSigmaPerMeter:h.centerQ*.0045,frontSigmaPerMeter:h.frontQ*.0045,centerConnectedToFeed:h.centerConnectedToFeed,frontConnectedToFeed:h.frontConnectedToFeed,plateau:{nativeCells:h.plateau.nativeCells,occupiedCells:h.plateau.occupiedCells,connectedCells:h.plateau.connectedCells,meanSigmaPerMeter:h.plateau.meanQ*.0045,peakSigmaPerMeter:h.plateau.peakQ*.0045,connectedOccupiedFraction:h.plateau.connectedOccupiedFraction}}))});
 }
 return {scope:'Requested native-grid sampling of the actual shared shaped liquid+ice extinction, current snapshot;6-neighbour threshold connectivity, not projected visibility or continuous subvoxel topology',sourceTime:scene.time,feedRoot,coefficientNormalizationPerMeter:.0045,samples};
}
