/** Source/index tests only; no GPU, visual quality or timing admission. */
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const code=await readFile(new URL('./convective-hierarchy.js',import.meta.url));
const {createHierarchyRecipe,hierarchyFrame,sampleHierarchy,sampleHierarchyFrame,validateHierarchyFrame,HIERARCHY_WGSL}=await import(`data:text/javascript;base64,${code.toString('base64')}`);
const error=(fn,code)=>assert.throws(fn,e=>e instanceof RangeError&&e.message.includes(code));
const close=(a,b,label)=>assert.ok(Math.abs(a-b)<2e-8,`${label}: ${a} != ${b}`);
const norm=a=>Math.hypot(...a);
let compared=0,fullChecked=0,birthChecked=0,maxRaw=0,maxVisits=0;
const recipes=[];
for(let variant=0;variant<4;variant++){
 const r=createHierarchyRecipe(variant,1729);recipes.push(r);
 assert.deepEqual(r,createHierarchyRecipe(variant,1729),'cook is reproducible');
 assert.deepEqual(r,JSON.parse(JSON.stringify(r)),'recoverable JSON source input');
 assert.ok(Object.isFrozen(r)&&Object.isFrozen(r.cohorts)&&Object.isFrozen(r.cohorts[0].plane));
 assert.ok(r.cook.history<=1024&&r.cook.maxLive<=256&&r.cook.minBirthSigma>.0189);
 assert.ok(r.cook.maxExposure<=-.99,'every grown cap actually clears all other coarser bodies');
 assert.deepEqual(r.cook.levels,[8,96,384]);
 assert.ok(Math.max(...r.bounds.extent)<=1228.8);
 for(const n of r.cohorts){
  assert.ok(n.retire<=1800);
  assert.ok(.75*n.amplitude*n.span*n.invRadius*norm(n.plane)/n.erase<=.25);
  if(n.parent>=0){assert.ok(n.radius<=.45*r.cohorts[n.parent].radius);assert.ok(n.birth+n.grow<=r.cohorts[n.parent].stop);}
 }
 for(const time of[0,1e-20,240,650,1000,1250,1500,1800]){
  const frame=hierarchyFrame(r,time);
  assert.equal(frame.metadata.length,16+4*4096);
  assert.equal(frame.sourceParams.length,4);
  assert.equal(frame.metadata[6],frame.stats.live);
  assert.ok(frame.nodes.every(Number.isFinite)&&frame.sourceParams.every(Number.isFinite));
  if(time===1e-20){assert.ok(frame.stats.omittedBirthNodes>0);assert.ok(frame.stats.omittedBirthTau>0&&frame.stats.omittedBirthTau<=r.numerical_birth.max_omitted_ray_tau);}
  assert.ok(frame.stats.rawMax<=64&&frame.stats.nonfullVisits<=134217728&&frame.stats.indexWork<=524288);
  assert.ok(frame.stats.resolutionAdmitted);
  maxRaw=Math.max(maxRaw,frame.stats.rawMax);maxVisits=Math.max(maxVisits,frame.stats.nonfullVisits);
  for(let k=0;k<160;k++){
   const q=r.bounds.min.map((v,a)=>v+r.bounds.extent[a]*(((k*[37,59,83][a]+13*a)%257)+.5)/257);
   const a=sampleHierarchy(r,q,time),b=sampleHierarchyFrame(frame,q,true),c=sampleHierarchyFrame(frame,q,false);
   close(a,b,'recipe/index');close(b,c,'indexed/brute');compared++;
   assert.ok(a>=0&&a<=r.sigma_m_inv);
   if(time<=0||time>=1800)assert.equal(a,0);
  }
  // Probe just inside/on/outside every current sphere's six axial edges.
  for(let i=0;i<frame.stats.live;i++)for(let axis=0;axis<3;axis++)for(const sign of[-1,1]){
   const q=[...frame.nodes.slice(i*16,i*16+3)];q[axis]+=sign*(frame.nodes[i*16+3]-.005);
   close(sampleHierarchyFrame(frame,q,true),sampleHierarchyFrame(frame,q,false),'sphere/brick edge');compared++;
  }
  // Full flags are independently checked against the nonindexed evaluator at
  // all cell-hull corners and the centre (base mask is still per query).
  if(variant===0&&time===650)for(let b=0;b<4096;b++){
   const o=16+4*b;if(frame.metadata[o+2]!==1)continue;
   const xyz=[b%16,Math.floor(b/16)%16,Math.floor(b/256)];
   for(let sample=0;sample<9;sample++){
    const q=xyz.map((v,a)=>r.bounds.min[a]+(v+(sample===8?.5:((sample>>a)&1)))*16*r.bounds.extent[a]/256);
    close(sampleHierarchyFrame(frame,q,true),sampleHierarchyFrame(frame,q,false),'full-material certificate');fullChecked++;
   }
  }
 }
 // Every birth is finite even at a positive offset too small to resolve support.
 for(const n of r.cohorts){
  for(const dt of[0,1e-20,1e-8]){
   const frame=hierarchyFrame(r,n.birth+dt,[128,128,128]);
   assert.ok(frame.nodes.every(Number.isFinite));birthChecked++;
  }
 }
 const f0=hierarchyFrame(r,650,[256,256,256],{maxLevel:0});
 const f1=hierarchyFrame(r,650,[256,256,256],{maxLevel:1});
 const f2=hierarchyFrame(r,650);
 assert.ok(f0.stats.live<f1.stats.live&&f1.stats.live<f2.stats.live);
 assert.notEqual(f0.generation,f1.generation);
 let fineGain=0;
 const centres=new Map();for(let i=0;i<f2.stats.live;i++)centres.set(f2.nodes[i*16+13],[...f2.nodes.slice(i*16,i*16+3)]);
 for(let i=0;i<f2.stats.live;i++){
  const o=i*16;if(f2.nodes[o+15]!==2||f2.nodes[o+3]<14)continue;
  const c=[...f2.nodes.slice(o,o+3)],p=centres.get(f2.nodes[o+14]);if(!p)continue;
  const v=c.map((x,k)=>x-p[k]),length=norm(v),q=c.map((x,k)=>x+v[k]/length*(f2.nodes[o+3]-6));
  const a=sampleHierarchyFrame(f0,q),b=sampleHierarchyFrame(f1,q),d=sampleHierarchyFrame(f2,q);
  assert.ok(a<=b+2e-8&&b<=d+2e-8,'hierarchy ablation is monotone max union');
  if(d>b+1e-5)fineGain++;
  close(d,sampleHierarchy(r,q,650,{maxLevel:2}),'diagnostic level source');
 }
 assert.ok(fineGain>0,'fine caps must add canonical exposed relief, not only hidden nodes');
 let born=0,lost=0;
 for(let z=0;z<16;z++)for(let y=0;y<16;y++)for(let x=0;x<16;x++){
  const q=[x,y,z].map((v,k)=>r.bounds.min[k]+(v+.5)/16*r.bounds.extent[k]);
  const a=sampleHierarchy(r,q,740),b=sampleHierarchy(r,q,1140);
  if(a<1e-7&&b>1e-5)born++;if(a>1e-5&&b<1e-7)lost++;
 }
 assert.ok(born&&lost,'mature local support must be both renewed and lost');
}
// Analytic one-sphere fixture isolates erosion/full-index correctness from art.
const base=recipes[0],template=base.cohorts[0];
const sphere={...template,id:0,renewal_of:-1,parent:-1,birth:-1,grow:1,radius:140,invRadius:Math.fround(1/140),
 start:[0,160,0],end:[0,160,0],roll:0,plane:[1,0,0],stop:400,erase:150,amplitude:152,
 span:Math.fround(.32*150*140/152)};
sphere.retire=Math.fround(sphere.stop+sphere.span+sphere.erase);
const fixture={...base,id:'analytic-eroding-sphere',cohorts:[sphere]};
const dense=hierarchyFrame(fixture,200),cleared=hierarchyFrame(fixture,575);
assert.ok(dense.stats.fullBricks>0);
assert.equal(cleared.stats.live,1,'nominal sphere still alive in its finite time window');
assert.equal(cleared.stats.fullBricks,0,'cleared parent must not use nominal-radius full shortcut');
assert.equal(sampleHierarchyFrame(cleared,[0,160,0]),0);
// Retired parent reference is retained for a child; compact density lists never
// become attachment storage, and no orphan snaps when the parent is removed.
const parent={...sphere,radius:200,invRadius:Math.fround(.005),amplitude:212};
parent.span=Math.fround(.32*150*200/212);parent.retire=Math.fround(parent.stop+parent.span+150);
const child={...sphere,id:1,parent:0,level:1,birth:100,grow:100,radius:70,invRadius:Math.fround(1/70),
 stop:900,amplitude:82,normal:[0,1,0],beta:.6,attach:18};
child.span=Math.fround(.32*150*70/82);child.retire=Math.fround(900+child.span+150);
const orphan={...base,id:'orphan-reference-fixture',cohorts:[parent,child]};
const before=hierarchyFrame(orphan,550),after=hierarchyFrame(orphan,650);
assert.equal(after.stats.live,1);
const bi=[...before.nodes].findIndex((v,i)=>i%16===13&&v===1)-13;
assert.deepEqual([...before.nodes.slice(bi,bi+3)],[...after.nodes.slice(0,3)]);
assert.ok(sampleHierarchyFrame(after,[0,332,0])>.0189);
const r=recipes[0],frame=hierarchyFrame(r,650);
for(const [limits,reason]of[[{local:1},'LOCAL_CAP'],[{indexWork:0},'INDEX_WORK'],[{entries:1},'ENTRY_CAP'],[{visits:1},'FIELD_WORK'],[{live:1},'LIVE_CAP']])error(()=>hierarchyFrame(r,650,[256,256,256],{limits}),reason);
const corrupt={...frame,sourceParams:frame.sourceParams.slice()};corrupt.sourceParams[0]++;
error(()=>sampleHierarchyFrame(corrupt,[0,200,0]),'GENERATION');
const invalid={...frame,metadata:frame.metadata.slice()};invalid.metadata[14]=0;
error(()=>sampleHierarchyFrame(invalid,[0,200,0]),'GENERATION');
error(()=>hierarchyFrame(r,1e40),'TIME');
error(()=>sampleHierarchy(r,[NaN,0,0],0),'QUERY');
error(()=>sampleHierarchyFrame(frame,[0,NaN,0]),'QUERY');
error(()=>hierarchyFrame(r,600,[256,256,256],{maxLevel:3}),'LEVEL');
error(()=>hierarchyFrame(r,600,[257,256,256]),'DIMENSIONS');
error(()=>hierarchyFrame(r,600,[256,256,256],{limits:{local:NaN}}),'LIMIT');
error(()=>hierarchyFrame({...r,sigma_m_inv:.2},600),'OPTICAL_PROFILE');
// Structural admission is separate from logical pool allocation: stale physical
// tail values must never become live nodes through a malformed list reference.
const cloneFrame=()=>({...frame,nodes:frame.nodes.slice(),metadata:frame.metadata.slice(),ids:frame.ids.slice(),
 sourceParams:frame.sourceParams.slice(),dimensions:[...frame.dimensions],bounds:{min:[...frame.bounds.min],extent:[...frame.bounds.extent]}});
const expected={recipeFingerprint:r.fingerprint,sourceTime:650,dimensions:[256,256,256],bounds:r.bounds,maxLevel:2,sourceParams:frame.sourceParams};
assert.equal(validateHierarchyFrame(frame,expected),frame);
const witnessQ=[95,161.33333333333334,-255];
close(sampleHierarchyFrame(frame,witnessQ),.002618281430486036,'reviewer valid witness');
const malformed=(mutate,reason='STRUCTURE')=>{const f=cloneFrame();mutate(f);error(()=>validateHierarchyFrame(f),reason);error(()=>sampleHierarchyFrame(f,witnessQ),reason);};
malformed(f=>{f.metadata[2]=15;});
malformed(f=>{f.metadata[5]=8;});
malformed(f=>{f.metadata[9]=128;});
malformed(f=>{f.metadata=f.metadata.slice(0,-1);});
malformed(f=>{f.metadata[6]=257;});
malformed(f=>{f.metadata[12]=65;});
malformed(f=>{f.metadata[13]=262145;});
malformed(f=>{f.nodes=f.nodes.slice(0,16);});
malformed(f=>{f.ids=f.ids.slice(0,1);});
malformed(f=>{f.dimensions=Array(3);});
malformed(f=>{f.bounds.min=Array(3);});
const occupied=Array.from({length:4096},(_,b)=>16+b*4).find(o=>frame.metadata[o+1]>=2);
assert.ok(occupied!==undefined);
malformed(f=>{f.metadata[occupied]++;});
malformed(f=>{f.metadata[occupied+1]=f.metadata[12]+1;});
malformed(f=>{f.metadata[13]++;});
malformed(f=>{f.ids[f.metadata[occupied]+1]=f.ids[f.metadata[occupied]];});
malformed(f=>{f.metadata[occupied+2]=2;});
malformed(f=>{f.metadata[occupied+2]=1;f.metadata[occupied+3]=f.metadata[6];});
malformed(f=>{f.sourceParams[1]=NaN;});
malformed(f=>{f.sourceParams[1]=.2;});
malformed(f=>{f.sourceParams[2]=0;});
malformed(f=>{f.sourceParams[3]=-1;});
malformed(f=>{f.nodes[4]=0;f.nodes[5]=0;f.nodes[6]=0;});
malformed(f=>{f.nodes[8]=1e6;});
malformed(f=>{f.stats={...f.stats,entries:f.stats.entries+1};});
const padded=cloneFrame();
padded.nodes=new Float32Array(256*16).fill(NaN);padded.nodes.set(frame.nodes);
padded.ids=new Uint32Array(262144).fill(0xffffffff);padded.ids.set(frame.ids);
assert.equal(validateHierarchyFrame(padded,expected),padded,'unused pooled capacity is permitted');
close(sampleHierarchyFrame(padded,witnessQ),sampleHierarchyFrame(frame,witnessQ),'pooled logical result');
padded.ids[padded.metadata[occupied]]=padded.metadata[6];
error(()=>validateHierarchyFrame(padded),'STRUCTURE'); // full explicit validation rescans in-place writes
const cached=cloneFrame();validateHierarchyFrame(cached);cached.metadata[2]=15;
error(()=>sampleHierarchyFrame(cached,witnessQ),'STRUCTURE'); // cheap guard detects changed header
for(const mismatch of[{recipeFingerprint:r.fingerprint^1},{sourceTime:651},{dimensions:[128,256,256]},
 {bounds:{min:[0,0,0],extent:r.bounds.extent}},{maxLevel:1},{sourceParams:[650,.018,12,12]}]){
 error(()=>validateHierarchyFrame(frame,mismatch),'GENERATION');
}
error(()=>sampleHierarchyFrame(frame,Array(3)),'QUERY');
assert.ok(HIERARCHY_WGSL.includes('arrayLength(&hierarchyMeta)<16u+count*4u'));
assert.ok(HIERARCHY_WGSL.includes('hierarchyMeta[5]!=16u'));
assert.ok(HIERARCHY_WGSL.includes('hierarchyInputsValid'));
assert.ok(HIERARCHY_WGSL.includes('hierarchyMeta[8]==field.grid.w'));
console.log(JSON.stringify({result:'CPU source/index checks pass; GPU/appearance unproven',compared,fullChecked,birthChecked,maxRaw,maxVisits,recipes:recipes.map(r=>({variant:r.variant,cook:r.cook}))},null,2));
