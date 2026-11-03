import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {prepareMaster,verifyMaster} from '../src/master-adapter.js';
const root=new URL('../',import.meta.url),baseline=JSON.parse(await readFile(new URL('assets/renderer-baseline.json',root))),placements=JSON.parse(await readFile(new URL('assets/master-placements.json',root)));
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
assert.equal(baseline.baseline_build,'6045d1ce1c9ccebe');
for(const [name,expected] of Object.entries(baseline.baseline_files)){
 if(!name.startsWith('src/')&&!name.startsWith('shaders/'))continue;
 let bytes=await readFile(new URL(name,root));
 if(name==='src/scene.js')bytes=bytes.toString().replace(' | (stage === null && ui.preparedDensity ? 32 : 0)','');
 if(name==='shaders/optics.wgsl')bytes=bytes.toString().replace('\n  if((u32(p.control.w)&32u)!=0u){return q*vec2f(p.bmin.w,p.bmax.w);}','');
 assert.equal(hash(bytes),expected,`only the two authorized renderer edits: ${name}`);
}
for(const id of ['03','08']){
 let anchor;
 for(const side of [128,256]){
  const meta=JSON.parse(await readFile(new URL(`assets/cloud-${id}-${side}.json`,root))),data=await readFile(new URL(`assets/cloud-${id}-${side}.density.r16f`,root));
  const bytes=data.buffer.slice(data.byteOffset,data.byteOffset+data.byteLength);
  assert.equal(await verifyMaster(bytes,meta,id),meta.output.sha256);
  const p=prepareMaster(bytes,meta,placements[id]);assert.equal(p.raw.byteLength,side**3*2);assert.equal(p.raw.buffer,bytes);assert.equal(p.rgba,undefined,'noRGBAexpansion');
  assert.equal(p.sum,meta.output.density_stats.sum);assert.equal(p.nonzero,meta.output.density_stats.nonzero);
  assert.equal(p.transform.bounds[1][0]-p.transform.bounds[0][0],2048);assert.equal(p.transform.pitch_m[0],2048/side);
  if(side===128){anchor=p.transform;assert.equal(anchor.bounds[0][1]+meta.output.support_bbox_min_zyx[1]*p.transform.pitch_m[1],1100);}
  else{assert.deepEqual(p.transform.bounds,anchor.bounds);assert.deepEqual(p.transform.centroid_world_m,anchor.centroid_world_m);}
 }
}
const masterRoot=new URL('../../cook/',import.meta.url);
assert.equal(hash(await readFile(new URL('cook.py',masterRoot))),hash(await readFile(new URL('cook-128-frozen.py',masterRoot))),'original128recipe preserved');
console.log('Four exact scalar hashes/raw half arrays, noRGBAexpansion, resolution-independent pose, sourcepitches, preserved128recipe and exact renderer math checks passed');
