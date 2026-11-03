import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {lookupGeometry,referenceAmount,sampleHalf} from '../src/geometry-adapter.js';
const root=new URL('../',import.meta.url),index=JSON.parse(await readFile(new URL('assets/geometry-index.json',root)));
const hash=x=>createHash('sha256').update(x).digest('hex');
assert.equal(index.parent_frozen_master_build,'adb13e8a53d0bd95');
const base=JSON.parse(await readFile(new URL('assets/geometry-renderer-base.json',root)));
for(const name of ['src/gpu.js','shaders/light.wgsl','shaders/view.wgsl','shaders/shadow.wgsl','shaders/display.wgsl'])assert.equal(hash(await readFile(new URL(name,root))),base.baseline_files[name],`qualifiedmathunchanged ${name}`);
for(const dataset of index.datasets){
 const read=async record=>{const b=await readFile(new URL('assets/'+record.file,root));assert.equal(b.length,record.bytes);assert.equal(hash(b),record.sha256);return b;};
 const curves=JSON.parse(await read(dataset.curves)),meta=JSON.parse(await read(dataset.distanceMetadata));await read(dataset.distance);
 assert.equal(curves.distance_sha256,dataset.distance.sha256);assert.equal(curves.density_source_sha256,meta.source_sha256);
 for(const channel of ['uniform','noisy']){
  const c=curves.channels[channel],zero=lookupGeometry(curves,channel,0,1),end=lookupGeometry(curves,channel,c.full_extinction_exposure_world_m,1);
  assert.equal(zero.geometryExposure,0);assert.equal(end.geometryExposure,c.full_extinction_exposure_world_m);
  assert.equal(referenceAmount(.2,100,.3,zero),.2);
  assert.equal(referenceAmount(.2,meta.distance_max_inside_world_m,channel==='noisy'?.3:1,end),0,'SDFendpointglobalbound');
  const snap=lookupGeometry(curves,channel,61.1,2);assert.equal(snap.geometryRhoThreshold,c.rho_threshold[snap.geometryLookup.index]);assert.equal(snap.geometryAlpha,c.alpha[snap.geometryLookup.index]);
  assert.equal(snap.geometryExposure,c.exposure_world_m[snap.geometryLookup.index]);
  assert.equal(referenceAmount(0,-100,1,snap),0);
  const alpha=lookupGeometry(curves,channel,60,3),support=lookupGeometry(curves,channel,60,1);
  assert.equal(referenceAmount(.2,300,1,support),.2,'deepinterior staysdense forsupportoffset');
  assert.ok(referenceAmount(.2,300,1,alpha)<.2,'matchedalpha changesinterior');
 }
}
const noise=await readFile(new URL('assets/'+index.noise.file,root));assert.equal(hash(noise),index.noise.sha256);
const constant=new Uint16Array(8).fill(0x3800);assert.equal(sampleHalf(constant,2,[0,0,0]),.5);assert.equal(sampleHalf(constant,2,[1,1,1]),.5);
console.log('Four pinned SDF/LUT sets, fixednoise, endpointbounds, exactrowmatching, nonfadeinterior response and unchangedlighting/view/shadow checks passed');

// Independent root-authored probe values exercise sampling/units and the LUT seam.
for(const dataset of index.datasets){
 const curves=JSON.parse(await readFile(new URL('assets/'+dataset.curves.file,root)));
 const bytes=await readFile(new URL(`assets/cloud-03-${dataset.quality}.density.r16f`,root));
 const q=new Uint16Array(bytes.buffer.slice(bytes.byteOffset,bytes.byteOffset+bytes.byteLength));
 const db=await readFile(new URL('assets/'+dataset.distance.file,root)),distance=new Uint16Array(db.buffer.slice(db.byteOffset,db.byteOffset+db.byteLength));
 const {halfToFloat}=await import('../src/reference.js');
 for(const channel of ['uniform','noisy']){
  const selection=curves.selected_topology[channel].find(x=>x.exposure_world_m===60);if(!selection)continue;
  const p=lookupGeometry(curves,channel,60,1),n=dataset.quality;
  for(let j=0;j<curves.interior_probe_positions_flat.length;j++){
   const i=curves.interior_probe_positions_flat[j],uv=[(i%n+.5)/n,(Math.floor(i/n)%n+.5)/n,(Math.floor(i/(n*n))+.5)/n];
   const M=channel==='noisy'?.3+.7*sampleHalf(new Uint16Array(noise.buffer.slice(noise.byteOffset,noise.byteOffset+noise.byteLength)),32,uv):1;
   const value=referenceAmount(halfToFloat(q[i]),halfToFloat(distance[i])*4,M,p);
   assert.ok(Math.abs(value-selection.sdf_probe_values[j])<1e-6,'root native reference and JSoperator agree at named probes');
  }
 }
}
const {Scene}=await import('../src/scene.js');
const scene=Object.create(Scene.prototype);let packed;
Object.assign(scene,{scratch:new Float32Array(44),family:1,steps:0,dims:[256,256,256],viewSize:[1280,720],bounds:()=>[[0,0,0],[2048,2048,2048]],target:()=>[1024,1024,1024],cameraPosition:()=>[100,200,300],gpu:{device:{queue:{writeBuffer(_buffer,_offset,array){packed=array.slice();}}}}});
const curves=JSON.parse(await readFile(new URL('assets/cloud-03-128-iso0p001.curves.json',root))),p=lookupGeometry(curves,'noisy',60,2);
scene.write({}, {wind:0,windIntegral:0,moist:0,dry:0,lift:0,time:0,frontX:1e6,storm:0},{...p,preparedDensity:true,sun:30,mode:0,mirror:true,main:true,viewSteps:384,provider:0,alpha:1,extinctionScale:1},null,0);
assert.equal(packed[28],32);assert.equal(packed[29],1);assert.equal(packed[30],384);assert.equal(packed[31],1);
assert.equal(packed[40],2);assert.equal(packed[41],60);assert.equal(packed[42],Math.fround(p.geometryRhoThreshold));assert.equal(packed[43],Math.fround(p.geometryAlpha));
console.log('Root native probe reference, world-distance scaling and optical uniform packing preserve ray/main flags');
