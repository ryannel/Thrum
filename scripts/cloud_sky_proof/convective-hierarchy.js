/** CF01 authored exposed-surface candidate, CLOUD-CU-SOURCE.md; no visual/physical pass.
 * Local metres, y=0 base, rigid event wind/yaw applied by the caller once.
 * Analytic C1 radius/centre/angle tracks retain retired parent reference poses.
 * Four vec4f/current node: centre/r; erosionDir/stop; span/T/A/delta0;
 * invFinalRadius/stableID/parentID/level. All values are float32-defined.
 * Metadata: 16u32 header [magic,version,bx,by,bz,16,live,timeBits,recipeKey,
 * nx,ny,nz,localCap,entries,valid,maxLevel], then [offset,count,flags,witness]
 * per brick. Flag1 proves full MATERIAL; always apply the base mask per query.
 * Bindings2/3/4 are nodes/metadata/IDs. Caller owns field uniform0/output1.
 * Age must match header/sourceParams exactly; caller sets field.grid.w to the
 * recipe fingerprint. hierarchyInputsValid checks this tag, age and dimensions.
 * Limits are failed admission, never coefficient clipping or dropped nodes.
 * Frame typed-array contents are immutable from CPU admission through retirement.
 * Nonempty typed arrays cannot be frozen: cached point checks detect header/view
 * replacement, not arbitrary in-place list writes. Call validateHierarchyFrame at
 * every upload/admission boundary; construct a new generation for any content edit.
 * Native correction views rejected this producer's clustered coverage/ball shapes;
 * these numerical checks do not promote its geometry or all hierarchy methods.
 */
const F = Math.fround;
const SIGMA = F(.019), EDGE = 12, BASE = 12, DURATION = 1800, EPS = 1e-6, PAD = .01;
const BRICK = 16, HEADER = 16, MAGIC = 0x43554832;
const LIMITS = Object.freeze({ live: 256, local: 64, entries: 262144, indexWork: 524288, visits: 134217728 });
const PROFILES = [
  { name: 'broad uneven heads', centres: [[-240,150,-20],[-80,210,20],[120,180,-35],[270,130,40]], radii: [210,235,225,195] },
  { name: 'compact double crown', centres: [[-155,190,-100],[120,235,-80],[-70,140,130],[190,165,120]], radii: [235,250,205,210] },
  { name: 'long shallow bank', centres: [[-270,160,0],[-90,185,90],[100,220,-50],[265,150,40]], radii: [185,200,215,180] },
  { name: 'offset rising crown', centres: [[-180,175,-80],[85,260,0],[240,140,110],[-70,130,170]], radii: [220,250,190,200] },
];
const add=(a,b)=>a.map((v,k)=>v+b[k]);
const sub=(a,b)=>a.map((v,k)=>v-b[k]);
const mul=(a,s)=>a.map(v=>v*s);
const dot=(a,b)=>a.reduce((s,v,k)=>s+v*b[k],0);
const norm=a=>Math.hypot(...a);
const unit=a=>{const n=norm(a);if(!(n>0))throw new RangeError('HIERARCHY_ZERO_BASIS');return mul(a,1/n);};
const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
const clamp=(x,a=0,b=1)=>Math.max(a,Math.min(b,x));
function smooth(a,b,x){const u=clamp((x-a)/(b-a));return u*u*(3-2*u);}
const identity=()=>[1,0,0,0,1,0,0,0,1];
const apply=(m,v)=>[m[0]*v[0]+m[3]*v[1]+m[6]*v[2],m[1]*v[0]+m[4]*v[1]+m[7]*v[2],m[2]*v[0]+m[5]*v[1]+m[8]*v[2]];
const transposeApply=(m,v)=>[dot(m.slice(0,3),v),dot(m.slice(3,6),v),dot(m.slice(6,9),v)];
const mm=(a,b)=>[...apply(a,b.slice(0,3)),...apply(a,b.slice(3,6)),...apply(a,b.slice(6,9))];
function basis(y){const x=unit(cross(Math.abs(y[1])<.9?[0,1,0]:[1,0,0],y));return [...x,...y,...cross(x,y)];}
function rotationZ(a){const c=Math.cos(a),s=Math.sin(a);return[c,s,0,-s,c,0,0,0,1];}
function hash(seed,id){let h=(seed^Math.imul(id+1,0x9e3779b1))>>>0;h=Math.imul(h^(h>>>16),0x85ebca6b)>>>0;h=Math.imul(h^(h>>>13),0xc2b2ae35)>>>0;return(h^(h>>>16))>>>0;}
const rand=(seed,id)=>hash(seed,id)/4294967296;
function freeze(x){if(x&&typeof x==='object'){Object.values(x).forEach(freeze);Object.freeze(x);}return x;}
function fail(code,detail){throw new RangeError(`${code}: ${detail}`);}
function time32(t){if(!Number.isFinite(t)||!Number.isFinite(F(t)))fail('HIERARCHY_TIME','finite float32 seconds required');return F(t);}
function inverse(x){const v=F(1/x);if(!(x>0)||!Number.isFinite(v)||v<=0)fail('HIERARCHY_RECIPROCAL',x);return v;}
function maxLevelOf(options={}){const n=typeof options==='number'?options:(options.maxLevel??2);if(!Number.isInteger(n)||n<0||n>2)fail('HIERARCHY_LEVEL',n);return n;}
function bits(t){return new Uint32Array(new Float32Array([t]).buffer)[0];}

function poses(nodes,t){
  const result=[];
  for(const n of nodes){
    const g=smooth(n.birth,n.birth+n.grow,t), r=n.radius*g;
    let c,orientation;
    const roll=n.roll*smooth(n.birth,n.birth+n.rollDuration,t);
    if(n.parent<0){c=n.start.map((v,k)=>v+(n.end[k]-v)*g);orientation=rotationZ(roll);}
    else{
      const p=result[n.parent];
      const outward=apply(p.orientation,n.normal);
      c=add(p.c,mul(outward,p.r-(1-n.beta)*r-n.attach*(1-g)));
      orientation=mm(mm(p.orientation,basis(n.normal)),rotationZ(roll));
    }
    const plane=unit(apply(orientation,n.plane)).map(F);
    const current={c:c.map(F),r:F(r),orientation,plane,node:n,
      active:t>n.birth&&t<n.retire&&r>=EPS};
    if(!current.c.every(Number.isFinite)||!Number.isFinite(current.r))fail('HIERARCHY_POSE',n.id);
    result.push(current);
  }
  return result;
}
function erosion(p,q,t){const n=p.node;const projection=dot(p.plane,sub(q,p.c))*n.invRadius;const delay=n.span*.5*clamp(1+projection,0,2);return n.amplitude*smooth(n.stop+delay,n.stop+delay+n.erase,t);}
function distance(p,q,t){return p.r-norm(sub(q,p.c))-erosion(p,q,t);}
function materialValue(ps,q,t,maxLevel=2){let d=-Infinity;for(const p of ps)if(p.active&&p.node.level<=maxLevel)d=Math.max(d,distance(p,q,t));return SIGMA*smooth(0,EDGE,d)*smooth(0,BASE,q[1]);}

function candidates(seed){const a=[];for(let i=0;i<768;i++){const y=1-2*(i+.5)/768,p=i*2.399963229728653+rand(seed,17)*Math.PI*2,s=Math.sqrt(1-y*y);a.push([Math.cos(p)*s,y,Math.sin(p)*s]);}return a;}
function selectSites(nodes,parent,birth,grow,radii,seed,level){
  const result=[], pool=candidates(seed);
  const initial=poses(nodes,birth+grow)[parent];
  let preferred;
  if(level===1){const world=unit([initial.c[0]*.7,240,initial.c[2]*.7]);preferred=transposeApply(initial.orientation,world);}
  else preferred=[0,1,0];
  pool.sort((a,b)=>dot(b,preferred)-dot(a,preferred));
  for(let index=0;index<radii.length;index++){
    const radius=radii[index], beta=.6, t=birth+index*(level===1?9:7)+grow;
    const ps=poses(nodes,t),pp=ps[parent];
    let chosen;
    for(const n of pool){
      if(level===1&&apply(pp.orientation,n)[1]<.08)continue;
      if(level===2&&n[1]<.42)continue;
      let nearest=Infinity, allowed=true;
      for(let j=0;j<result.length;j++){
        const arc=Math.acos(clamp(dot(n,result[j]),-1,1))*nodes[parent].radius;
        const ratio=arc/((radius+radii[j])*.5);
        if(ratio<1.45){allowed=false;break;}
        nearest=Math.min(nearest,ratio);
      }
      if(!allowed||(result.length&&nearest>1.8))continue;
      const worldN=apply(pp.orientation,n), peak=add(pp.c,mul(worldN,pp.r+beta*radius));
      let hidden=false;
      for(const other of ps){
        if(!other.active||other.node.id===parent||other.node.level>=level)continue;
        if(distance(other,peak,t)>-1){hidden=true;break;}
      }
      if(hidden)continue;
      chosen=n;break;
    }
    if(!chosen)fail('HIERARCHY_EXPOSED_SITES',`level${level} parent${parent} placed${result.length}/${radii.length}`);
    result.push(chosen);
  }
  return result;
}

export function createHierarchyRecipe(variant=0,seed=1729){
  if(!Number.isInteger(variant)||variant<0||variant>3)fail('HIERARCHY_VARIANT',variant);
  if(!Number.isInteger(seed)||seed<0||seed>0xffffffff)fail('HIERARCHY_SEED',seed);
  const profile=PROFILES[variant],nodes=[];
  const addNode=data=>{
    const id=nodes.length,R=F(data.radius),erase=[300,150,90][data.level];
    const az=rand(seed,id*19+5)*2*Math.PI;
    const plane=unit([Math.cos(az),.35,Math.sin(az)]).map(F);
    const amplitude=F(R+EDGE),span=F(.32*erase*R/(R+EDGE));
    const n={id,parent:data.parent,renewal_of:data.renewal_of??-1,level:data.level,birth:F(data.birth),grow:F(data.grow),radius:R,
      stop:F(data.stop),erase,span,amplitude,invRadius:inverse(R),plane,
      normal:data.normal??[0,1,0],beta:data.beta??.6,attach:18,
      start:data.start??[0,0,0],end:data.end??[0,0,0],
      roll:(rand(seed,id*19+9)-.5)*.12,rollDuration:400};
    n.retire=F(n.stop+n.span+n.erase);
    if(n.retire>DURATION)fail('HIERARCHY_RETIREMENT',`${id} retires${n.retire}`);
    if(.75*n.amplitude*n.span*n.invRadius*norm(n.plane)/n.erase>.25)fail('HIERARCHY_EROSION_GRADIENT',id);
    nodes.push(n);return n;
  };
  const rootIds=[];
  for(let gen=0;gen<2;gen++)for(let i=0;i<4;i++){
    const end=profile.centres[i].map((v,k)=>v+(gen?(k===1?15:(rand(seed,400+i*3+k)-.5)*16):0));
    const start=gen?[...profile.centres[i]]:[end[0]*.25,0,end[2]*.25];
    const n=addNode({level:0,parent:-1,renewal_of:gen?i:-1,birth:(gen?820:0)+i*45,grow:260,
      stop:gen?1395:1100+i*12,radius:profile.radii[i],start,end});
    rootIds.push({id:n.id,gen});
  }
  const mediumIds=[];
  for(const entry of rootIds){
    const parent=nodes[entry.id];
    for(const offset of entry.gen?[0]:[0,260,520]){
      const birth=parent.birth+232+offset;
      const radii=Array.from({length:6},(_,j)=>parent.radius*(.37+.02*rand(seed,parent.id*1000+offset+j)));
      const ns=selectSites(nodes,parent.id,birth,100,radii,hash(seed,parent.id+offset),1);
      for(let j=0;j<6;j++)mediumIds.push(addNode({level:1,parent:parent.id,birth:birth+j*9,grow:100,
        stop:birth+j*9+210,radius:radii[j],normal:ns[j]}).id);
    }
  }
  // All coarser siblings exist before fine exposure is tested; future live siblings
  // cannot be accidentally ignored merely because they were added later in a loop.
  for(const id of mediumIds){
    const n=nodes[id];
    const fineR=Array.from({length:4},(_,k)=>Math.min(38,Math.max(28.5,n.radius*(.395+.02*rand(seed,n.id*100+k)))));
    const fb=n.birth+90;
    const fns=selectSites(nodes,n.id,fb,42,fineR,hash(seed,n.id+99),2);
    for(let k=0;k<4;k++)addNode({level:2,parent:n.id,birth:fb+k*7,grow:42,stop:fb+k*7+65,radius:fineR[k],normal:fns[k]});
  }
  if(nodes.length>1024)fail('HIERARCHY_HISTORY_CAP',nodes.length);
  const bounds={min:[-608,0,-480],extent:[1216,704,960]};
  const recipe={schema:'thrum.authored-cu-hierarchy.v1',id:`cu-hierarchy-v1:${variant}:${seed}`,variant,seed,name:profile.name,
    fingerprint:hash(seed,variant+917),duration_s:DURATION,bounds,
    sigma_m_inv:SIGMA,w_edge_m:EDGE,base_feather_m:BASE,limits:LIMITS,
    provenance:'CLOUD-CU-SOURCE.md authored geometry correction; no fluid, GPU, appearance or performance qualification.',
    stages:{formation:[0,400],growth:[400,650],mature:[650,1350],breakup:[1350,1800]},
    normal_intervals:[[210,270],[570,630],[990,1050],[1470,1530]],cohorts:nodes,
    numerical_birth:{minimum_radius_m:EPS,max_omitted_ray_tau:2*EPS*SIGMA*256*(1+128*Number.EPSILON),pose:'float32 current centre/radius/erosion inputs'},
    scope:'Authored spherical surface hierarchy, not a fluid simulation or accepted appearance. First wind is rigid; maxLevel is diagnostic only.'};
  // Exact event sweep gives a live-count bound independent of frame-rate sampling.
  const events=nodes.flatMap(n=>[[n.birth,1],[n.retire,-1]]).sort((a,b)=>a[0]-b[0]||a[1]-b[1]);
  let live=0,maxLive=0;for(const [,delta]of events){live+=delta;maxLive=Math.max(maxLive,live);}
  if(maxLive>LIMITS.live)fail('HIERARCHY_LIVE_CAP',maxLive);
  let minBirthSigma=Infinity,maxExposure=-Infinity;
  for(const node of nodes){
    if(node.parent<0){
      if(node.renewal_of>=0){const ps=poses(nodes,node.birth);if(materialValue([ps[node.renewal_of]],ps[node.id].c,node.birth)<SIGMA*.999)fail('HIERARCHY_ROOT_RENEWAL',node.id);}
      continue;
    }
    if(node.birth+node.grow>nodes[node.parent].stop)fail('HIERARCHY_GROWTH_ATTACHMENT',node.id);
    const ps=poses(nodes,node.birth),parent=ps[node.parent],current=ps[node.id];
    if(node.radius>.45*parent.r+1e-5)fail('HIERARCHY_CURVATURE',`${node.id} child${node.radius} parent${parent.r}`);
    const parentSigma=materialValue([parent],current.c,node.birth);
    minBirthSigma=Math.min(minBirthSigma,parentSigma);
    if(parentSigma<SIGMA*.999)fail('HIERARCHY_PARENT_PLATEAU',`${node.id} sigma${parentSigma}`);
    if(node.level===2&&node.beta*node.radius<3*4.75)fail('HIERARCHY_FINE_RESOLUTION',node.id);
    const fullTime=F(node.birth+node.grow),complete=poses(nodes,fullTime),self=complete[node.id],pp=complete[node.parent];
    const outward=unit(sub(self.c,pp.c)),peak=add(self.c,mul(outward,self.r));
    for(const other of complete){
      if(!other.active||other.node.id===node.parent||other.node.level>=node.level)continue;
      const f=distance(other,peak,fullTime);maxExposure=Math.max(maxExposure,f);
      if(f>-1+PAD)fail('HIERARCHY_BURIED_CAP',`${node.id} hidden by${other.node.id} d${f}`);
    }
  }
  // Sphere/Minkowski reach bounds cover all roll angles and intermediate growth.
  for(const node of nodes){
    let root=node;
    while(root.parent>=0)root=nodes[root.parent];
    const maxReach=1.37*root.radius; // >= R0+.65*.39R0+.65*.45*.39R0.
    for(let k=0;k<3;k++){
      const low=Math.min(root.start[k],root.end[k])-maxReach,high=Math.max(root.start[k],root.end[k])+maxReach;
      if(k!==1&&low<bounds.min[k]+9.5)fail('HIERARCHY_BOUNDS',`axis${k} low${low}`);
      if(high>bounds.min[k]+bounds.extent[k]-9.5)fail('HIERARCHY_BOUNDS',`axis${k} high${high}`);
    }
  }
  recipe.cook={history:nodes.length,maxLive,minBirthSigma,maxExposure,levels:[0,1,2].map(l=>nodes.filter(n=>n.level===l).length)};
  return freeze(recipe);
}

const cache=new WeakMap();
function current(recipe,time,maxLevel){
  if(!Number.isFinite(recipe.sigma_m_inv)||recipe.sigma_m_inv<0||recipe.sigma_m_inv>.1||!(recipe.w_edge_m>0)||!Number.isFinite(recipe.w_edge_m)||!(recipe.base_feather_m>0)||!Number.isFinite(recipe.base_feather_m))fail('HIERARCHY_OPTICAL_PROFILE','invalid coefficient/ramp; never clamp');
  const t=time32(time),key=`${t}:${maxLevel}`;
  let entries=cache.get(recipe);if(!entries){entries=new Map();cache.set(recipe,entries);}
  if(entries.has(key))return entries.get(key);
  const all=poses(recipe.cohorts,t),live=all.filter(p=>p.active&&p.node.level<=maxLevel&&t>0&&t<DURATION);
  if(live.length>256)fail('HIERARCHY_LIVE_CAP',live.length);
  const nodes=new Float32Array(Math.max(16,live.length*16));
  for(let i=0;i<live.length;i++){
    const p=live[i],n=p.node;
    nodes.set([...p.c,p.r,...p.plane,n.stop,n.span,n.erase,n.amplitude,0,n.invRadius,n.id,n.parent,n.level],i*16);
  }
  if(!nodes.every(Number.isFinite))fail('HIERARCHY_NONFINITE','upload rejected');
  const omittedBirthNodes=all.filter(p=>p.node.level<=maxLevel&&t>p.node.birth&&t<p.node.retire&&!p.active&&t>0&&t<DURATION).length;
  const out={t,all,live,nodes,omittedBirthNodes};entries.set(key,out);if(entries.size>2)entries.delete(entries.keys().next().value);return out;
}
function packedDistance(nodes,i,q,t){const o=i*16;const dx=q[0]-nodes[o],dy=q[1]-nodes[o+1],dz=q[2]-nodes[o+2];
  const delay=nodes[o+11]+nodes[o+8]*.5*clamp(1+(dx*nodes[o+4]+dy*nodes[o+5]+dz*nodes[o+6])*nodes[o+12],0,2);
  return nodes[o+3]-Math.hypot(dx,dy,dz)-nodes[o+10]*smooth(nodes[o+7]+delay,nodes[o+7]+delay+nodes[o+9],t);}
export function sampleHierarchy(recipe,q,time,options={}){
  if(!q||q.length!==3||![...q].every(Number.isFinite))fail('HIERARCHY_QUERY','finite local metres required');
  const c=current(recipe,time,maxLevelOf(options));if(q[1]<=0)return 0;
  let d=-Infinity;for(let i=0;i<c.live.length;i++)d=Math.max(d,packedDistance(c.nodes,i,q,c.t));
  return recipe.sigma_m_inv*smooth(0,recipe.w_edge_m,d)*smooth(0,recipe.base_feather_m,q[1]);
}
const boxCache=new Map();
function brickBoxes(dims,bounds,bs){
  const key=[...dims,...bounds.min,...bounds.extent].join(':');
  if(boxCache.has(key))return boxCache.get(key);
  const result=[];for(let z=0;z<bs[2];z++)for(let y=0;y<bs[1];y++)for(let x=0;x<bs[0];x++)result.push(boxFor([x,y,z],dims,bounds));
  boxCache.set(key,result);if(boxCache.size>2)boxCache.delete(boxCache.keys().next().value);return result;
}
function boxFor(b,dims,bounds){const lo=[],hi=[];for(let k=0;k<3;k++){const pitch=bounds.extent[k]/dims[k];lo[k]=bounds.min[k]+b[k]*16*pitch-PAD;hi[k]=bounds.min[k]+Math.min((b[k]+1)*16,dims[k])*pitch+PAD;}return{lo,hi};}
function intersects(nodes,i,box){let d=0;const o=i*16;for(let k=0;k<3;k++){const v=nodes[o+k],x=Math.max(box.lo[k]-v,0,v-box.hi[k]);d+=x*x;}return d<=(nodes[o+3]+PAD)**2;}
function full(nodes,i,box,t,wedge){const o=i*16;let far=0,projection=0;
  for(let k=0;k<3;k++){const c=nodes[o+k],v=Math.max(Math.abs(box.lo[k]-c),Math.abs(box.hi[k]-c));far+=v*v;projection+=nodes[o+4+k]*((nodes[o+4+k]>=0?box.lo[k]:box.hi[k])-c);}
  const delta=nodes[o+11]+nodes[o+8]*.5*clamp(1+projection*nodes[o+12],0,2);
  const e=nodes[o+10]*smooth(nodes[o+7]+delta,nodes[o+7]+delta+nodes[o+9],t);
  return nodes[o+3]-Math.sqrt(far)-e>=wedge+PAD;
}

export function hierarchyFrame(recipe,time,dimensions=[256,256,256],options={}){
  if(dimensions.length!==3||dimensions.some(n=>!Number.isInteger(n)||n<16||n>256||n%16))fail('HIERARCHY_DIMENSIONS','multiples of16 in16..256');
  const maxLevel=maxLevelOf(options),c=current(recipe,time,maxLevel),dims=[...dimensions],bs=dims.map(n=>n/16),B=bs[0]*bs[1]*bs[2];
  const limits={...LIMITS,...(options.limits??{})};for(const k of Object.keys(LIMITS))if(!Number.isSafeInteger(limits[k])||limits[k]<0||limits[k]>LIMITS[k])fail('HIERARCHY_LIMIT',k);
  if(c.live.length>limits.live)fail('HIERARCHY_LIVE_CAP',c.live.length);
  const counts=new Uint32Array(B),offsets=new Uint32Array(B+1),cursor=new Uint32Array(B),boxes=brickBoxes(dims,recipe.bounds,bs);
  let indexWork=0,rawMax=0;
  const walk=callback=>{for(let i=0;i<c.live.length;i++){
    const o=i*16,r=c.nodes[o+3]+PAD,min=[],max=[];
    for(let k=0;k<3;k++){const pitch=recipe.bounds.extent[k]/bs[k];min[k]=Math.max(0,Math.floor((c.nodes[o+k]-r-recipe.bounds.min[k])/pitch));max[k]=Math.min(bs[k]-1,Math.floor((c.nodes[o+k]+r-recipe.bounds.min[k])/pitch));}
    for(let z=min[2];z<=max[2];z++)for(let y=min[1];y<=max[1];y++)for(let x=min[0];x<=max[0];x++){
      if(++indexWork>limits.indexWork)fail('HIERARCHY_INDEX_WORK',indexWork);
      const b=x+bs[0]*(y+bs[1]*z);if(intersects(c.nodes,i,boxes[b]))callback(b,i);
    }
  }};
  walk(b=>{const n=++counts[b];rawMax=Math.max(rawMax,n);if(n>limits.local)fail('HIERARCHY_LOCAL_CAP',`brick${b} count${n}`);});
  for(let b=0;b<B;b++)offsets[b+1]=offsets[b]+counts[b];
  if(offsets[B]>limits.entries)fail('HIERARCHY_ENTRY_CAP',offsets[B]);
  const ids=new Uint32Array(Math.max(1,offsets[B]));
  walk((b,i)=>{ids[offsets[b]+cursor[b]++]=i;});
  const metadata=new Uint32Array(HEADER+4*B);metadata.set([MAGIC,1,...bs,16,c.live.length,bits(c.t),recipe.fingerprint,...dims,limits.local,offsets[B],1,maxLevel]);
  let fullBricks=0,emptyBricks=0,nonfullVisits=0,fullTests=0;
  for(let b=0;b<B;b++){
    if(cursor[b]!==counts[b])fail('HIERARCHY_FILL_MISMATCH',b);
    let witness=0xffffffff;
    for(let j=offsets[b];j<offsets[b+1];j++){fullTests++;if(full(c.nodes,ids[j],boxes[b],c.t,recipe.w_edge_m)){witness=ids[j];break;}}
    const flag=witness!==0xffffffff?1:0;
    if(flag)fullBricks++;else if(!counts[b])emptyBricks++;else nonfullVisits+=4096*counts[b];
    if(nonfullVisits>limits.visits)fail('HIERARCHY_FIELD_WORK',nonfullVisits);
    metadata.set([offsets[b],counts[b],flag,witness],HEADER+4*b);
  }
  const sourceParams=new Float32Array([c.t,recipe.sigma_m_inv,recipe.w_edge_m,recipe.base_feather_m]);
  const stats={live:c.live.length,levels:[0,1,2].map(l=>c.live.filter(p=>p.node.level===l).length),entries:offsets[B],rawMax,nonfullVisits,fullBricks,emptyBricks,indexWork,fullTests,
    poseRecords:c.all.length,
    bytes:{nodes:c.nodes.byteLength,metadata:metadata.byteLength,ids:ids.byteLength,total:c.nodes.byteLength+metadata.byteLength+ids.byteLength,
      cpuIndexScratch:counts.byteLength+offsets.byteLength+cursor.byteLength,cachedBrickBoundScalarsLogical:B*6*8},
    omittedBirthNodes:c.omittedBirthNodes,omittedBirthTau:2*EPS*SIGMA*c.omittedBirthNodes*(1+128*Number.EPSILON),resolutionAdmitted:Math.max(...recipe.bounds.extent.map((v,k)=>v/dims[k]))<=4.8};
  const frame={nodes:c.nodes,metadata,ids,sourceParams,bounds:recipe.bounds,dimensions:dims,maxLevel,stats,
    generation:`${recipe.id}:${bits(c.t)}:${dims.join('x')}:L${maxLevel}`,recipeFingerprint:recipe.fingerprint};
  return validateHierarchyFrame(frame);
}
const admittedFrames=new WeakMap();
function headerAdmission(frame){
  if(!frame||!(frame.nodes instanceof Float32Array)||!(frame.metadata instanceof Uint32Array)||!(frame.ids instanceof Uint32Array)||!(frame.sourceParams instanceof Float32Array))fail('HIERARCHY_STRUCTURE','typed frame views required');
  const m=frame.metadata,p=frame.sourceParams,dims=frame.dimensions,bounds=frame.bounds;
  if(m.length<HEADER||p.length!==4||!p.every(Number.isFinite)||p[1]<0||p[1]>.1||!(p[2]>0)||!(p[3]>0))fail('HIERARCHY_STRUCTURE','header/source parameters');
  if(!Array.isArray(dims)||dims.length!==3||[0,1,2].some(k=>!Number.isInteger(dims[k])||dims[k]<16||dims[k]>256||dims[k]%16))fail('HIERARCHY_STRUCTURE','logical grid');
  if(!bounds||!Array.isArray(bounds.min)||!Array.isArray(bounds.extent)||bounds.min.length!==3||bounds.extent.length!==3||[0,1,2].some(k=>!Number.isFinite(bounds.min[k])||!Number.isFinite(bounds.extent[k])||bounds.extent[k]<=0))fail('HIERARCHY_STRUCTURE','bounds');
  if(m[0]!==MAGIC||m[1]!==1||m[14]!==1||m[7]!==bits(p[0])||m[8]!==frame.recipeFingerprint||m[15]!==frame.maxLevel||!Number.isInteger(frame.maxLevel)||frame.maxLevel<0||frame.maxLevel>2)fail('HIERARCHY_GENERATION','frame/header mismatch');
  if(m[5]!==BRICK||dims.some((n,k)=>m[9+k]!==n||m[2+k]!==n/BRICK))fail('HIERARCHY_STRUCTURE','brick grid/stride mismatch');
  const bricks=m[2]*m[3]*m[4],live=m[6],entries=m[13];
  if(m.length<HEADER+4*bricks||m.length>HEADER+4*4096||m[12]>LIMITS.local||live>LIMITS.live||entries>LIMITS.entries||entries>bricks*m[12])fail('HIERARCHY_STRUCTURE','logical header capacities');
  if(frame.nodes.length<Math.max(16,live*16)||frame.nodes.length%16||frame.nodes.length>LIMITS.live*16||frame.ids.length<Math.max(1,entries)||frame.ids.length>LIMITS.entries)fail('HIERARCHY_STRUCTURE','physical pool capacities');
  if(live===0&&entries!==0)fail('HIERARCHY_STRUCTURE','empty logical nodes with entries');
  return{m,p,dims,bounds,bricks,live,entries};
}
function matchesAdmission(frame,a){
  return a&&a.nodes===frame.nodes&&a.metadata===frame.metadata&&a.ids===frame.ids&&a.params===frame.sourceParams&&a.generation===frame.generation
    &&a.header.every((v,k)=>v===frame.metadata[k])&&a.source.every((v,k)=>v===frame.sourceParams[k])
    &&a.bounds.every((v,k)=>v===(k<3?frame.bounds.min[k]:frame.bounds.extent[k-3]));
}
/** Full CPU admission, once at an upload/generation boundary. Trailing pooled
 * capacity is permitted; every list reference must address LOGICAL live nodes.
 * Expected fields are optional recipeFingerprint, sourceTime, dimensions,
 * bounds, maxLevel and sourceParams. Returns frame; errors never mean clear sky.
 */
export function validateHierarchyFrame(frame,expected={}){
  const {m,p,dims,bounds,bricks,live,entries}=headerAdmission(frame);
  if(expected.recipeFingerprint!==undefined&&m[8]!==expected.recipeFingerprint)fail('HIERARCHY_GENERATION','unexpected recipe tag');
  if(expected.sourceTime!==undefined&&m[7]!==bits(time32(expected.sourceTime)))fail('HIERARCHY_GENERATION','unexpected source time');
  if(expected.maxLevel!==undefined&&m[15]!==expected.maxLevel)fail('HIERARCHY_GENERATION','unexpected diagnostic level');
  if(expected.dimensions&&dims.some((n,k)=>n!==expected.dimensions[k]))fail('HIERARCHY_GENERATION','unexpected logical grid');
  if(expected.bounds&&bounds.min.some((n,k)=>n!==expected.bounds.min[k])||expected.bounds&&bounds.extent.some((n,k)=>n!==expected.bounds.extent[k]))fail('HIERARCHY_GENERATION','unexpected bounds');
  if(expected.sourceParams&&p.some((n,k)=>n!==Math.fround(expected.sourceParams[k])))fail('HIERARCHY_GENERATION','unexpected source parameters');
  const stableIds=new Set(),levels=[0,0,0];
  for(let i=0;i<live;i++){
    const o=i*16;
    for(let k=0;k<16;k++)if(!Number.isFinite(frame.nodes[o+k]))fail('HIERARCHY_STRUCTURE',`nonfinite live node${i}`);
    if(!(frame.nodes[o+3]>0)||frame.nodes[o+8]<0||!(frame.nodes[o+9]>0)||frame.nodes[o+10]<0||!(frame.nodes[o+12]>0))fail('HIERARCHY_STRUCTURE',`invalid live node${i}`);
    const directionLength=Math.hypot(frame.nodes[o+4],frame.nodes[o+5],frame.nodes[o+6]);
    if(!(directionLength>0)||.75*frame.nodes[o+10]*frame.nodes[o+8]*frame.nodes[o+12]*directionLength/frame.nodes[o+9]>.25)fail('HIERARCHY_STRUCTURE',`packed erosion gradient${i}`);
    const id=frame.nodes[o+13],parent=frame.nodes[o+14],level=frame.nodes[o+15];
    if(!Number.isInteger(id)||id<0||id>=1024||stableIds.has(id)||!Number.isInteger(parent)||parent< -1||parent>=1024||!Number.isInteger(level)||level<0||level>frame.maxLevel)fail('HIERARCHY_STRUCTURE',`live identity/level${i}`);
    stableIds.add(id);levels[level]++;
  }
  let prefix=0,rawMax=0,fullBricks=0,emptyBricks=0,nonfullVisits=0;
  for(let b=0;b<bricks;b++){
    const o=HEADER+b*4,offset=m[o],count=m[o+1],flag=m[o+2],witness=m[o+3];
    if(offset!==prefix||count>m[12]||offset>entries||count>entries-offset||flag>1)fail('HIERARCHY_STRUCTURE',`brick${b} offset/count/flag`);
    let previous=-1,witnessFound=false;
    for(let j=offset;j<offset+count;j++){
      const id=frame.ids[j];
      if(id>=live||id<=previous)fail('HIERARCHY_STRUCTURE',`brick${b} ID${id} outside logical live or not strictly ordered`);
      if(id===witness)witnessFound=true;previous=id;
    }
    if(flag===1&&(!count||witness>=live||!witnessFound)||flag===0&&witness!==0xffffffff)fail('HIERARCHY_STRUCTURE',`brick${b} full witness`);
    prefix+=count;rawMax=Math.max(rawMax,count);
    if(flag)fullBricks++;else if(!count)emptyBricks++;else nonfullVisits+=4096*count;
  }
  if(prefix!==entries||nonfullVisits>LIMITS.visits)fail('HIERARCHY_STRUCTURE','entry end/work bound');
  if(frame.stats){
    for(const [key,value]of Object.entries({live,entries,rawMax,fullBricks,emptyBricks,nonfullVisits}))if(frame.stats[key]!==undefined&&frame.stats[key]!==value)fail('HIERARCHY_STRUCTURE',`stats ${key}`);
    if(frame.stats.levels&&levels.some((n,k)=>frame.stats.levels[k]!==n))fail('HIERARCHY_STRUCTURE','stats levels');
    if(frame.stats.indexWork!==undefined&&(!Number.isInteger(frame.stats.indexWork)||frame.stats.indexWork<2*entries||frame.stats.indexWork>LIMITS.indexWork))fail('HIERARCHY_STRUCTURE','stats index work');
  }
  admittedFrames.set(frame,{nodes:frame.nodes,metadata:frame.metadata,ids:frame.ids,params:frame.sourceParams,generation:frame.generation,
    header:[...m.slice(0,HEADER)],source:[...p],bounds:[...bounds.min,...bounds.extent]});
  return frame;
}
export function sampleHierarchyFrame(frame,q,indexed=true){
  if(!q||q.length!==3||![...q].every(Number.isFinite))fail('HIERARCHY_QUERY','finite local metres required');
  headerAdmission(frame);
  if(!matchesAdmission(frame,admittedFrames.get(frame)))validateHierarchyFrame(frame);
  const t=frame.sourceParams[0];
  if(q[1]<=0)return 0;
  const lo=frame.bounds.min,ex=frame.bounds.extent;
  if(q.some((v,k)=>v<lo[k]||v>=lo[k]+ex[k]))return 0;
  let d=-Infinity;
  if(!indexed){for(let i=0;i<frame.metadata[6];i++)d=Math.max(d,packedDistance(frame.nodes,i,q,t));}
  else{
    const bs=[frame.metadata[2],frame.metadata[3],frame.metadata[4]],xyz=q.map((v,k)=>Math.min(bs[k]-1,Math.floor((v-lo[k])/ex[k]*frame.dimensions[k]/16)));
    const b=xyz[0]+bs[0]*(xyz[1]+bs[1]*xyz[2]),o=HEADER+4*b;
    if(frame.metadata[o+2]===1)return frame.sourceParams[1]*smooth(0,frame.sourceParams[3],q[1]);
    for(let j=0;j<frame.metadata[o+1];j++)d=Math.max(d,packedDistance(frame.nodes,frame.ids[frame.metadata[o]+j],q,t));
  }
  return frame.sourceParams[1]*smooth(0,frame.sourceParams[2],d)*smooth(0,frame.sourceParams[3],q[1]);
}

export const HIERARCHY_WGSL=/*wgsl*/`
@group(0) @binding(2) var<storage,read> hierarchyNodes:array<vec4f>;
@group(0) @binding(3) var<storage,read> hierarchyMeta:array<u32>;
@group(0) @binding(4) var<storage,read> hierarchyIds:array<u32>;
fn hierarchyInputsValid(age:f32)->bool {
 if(arrayLength(&hierarchyMeta)<16u){return false;}
 if(any(field.grid.xyz<vec3u(16u)) || any(field.grid.xyz>vec3u(256u)) || any(field.grid.xyz%vec3u(16u)!=vec3u(0u))){return false;}
 let bricks=field.grid.xyz/16u;
 let count=bricks.x*bricks.y*bricks.z;
 if(hierarchyMeta[5]!=16u || any(vec3u(hierarchyMeta[2],hierarchyMeta[3],hierarchyMeta[4])!=bricks)) {return false;}
 if(arrayLength(&hierarchyMeta)<16u+count*4u || arrayLength(&hierarchyMeta)>16400u) {return false;}
 if(hierarchyMeta[6]>256u || hierarchyMeta[12]>64u || hierarchyMeta[13]>262144u || hierarchyMeta[13]>count*hierarchyMeta[12]) {return false;}
 if(arrayLength(&hierarchyNodes)<max(4u,hierarchyMeta[6]*4u) || arrayLength(&hierarchyNodes)>1024u || arrayLength(&hierarchyNodes)%4u!=0u || arrayLength(&hierarchyIds)<max(1u,hierarchyMeta[13]) || arrayLength(&hierarchyIds)>262144u) {return false;}
 if(hierarchyMeta[6]==0u && hierarchyMeta[13]!=0u){return false;}
 // Per-brick ranges/sorted IDs are admitted once by the host before upload;
 // these immutable generation buffers are not rescanned for every source point.
 return hierarchyMeta[0]==0x43554832u && hierarchyMeta[1]==1u && hierarchyMeta[14]==1u && hierarchyMeta[7]==bitcast<u32>(age)
 && field.source.x==age && hierarchyMeta[8]==field.grid.w && hierarchyMeta[15]<=2u
 && hierarchyMeta[9]==field.grid.x && hierarchyMeta[10]==field.grid.y && hierarchyMeta[11]==field.grid.z;
}
fn canonicalSigma(q:vec3f,age:f32)->f32 {
 if(!hierarchyInputsValid(age)){return 0.;} // Host rejects invalid; point wrapper reports unsupported.
 if(q.y<=0. || any(q<field.lo.xyz) || any(q>=field.lo.xyz+field.extent.xyz)){return 0.;}
 let cell=vec3u(floor((q-field.lo.xyz)/field.extent.xyz*vec3f(field.grid.xyz)));
 let xyz=min(cell/16u,vec3u(hierarchyMeta[2],hierarchyMeta[3],hierarchyMeta[4])-vec3u(1u));
 let brick=xyz.x+hierarchyMeta[2]*(xyz.y+hierarchyMeta[3]*xyz.z);
 let o=16u+brick*4u;
 let base=smoothstep(0.,field.source.w,q.y);
 if(hierarchyMeta[o+2u]==1u){return field.source.y*base;}
 var d=-1e30;
 for(var j=0u;j<hierarchyMeta[o+1u];j=j+1u){
   let k=hierarchyIds[hierarchyMeta[o]+j]*4u;
   let sphere=hierarchyNodes[k];let plane=hierarchyNodes[k+1u];let loss=hierarchyNodes[k+2u];let extra=hierarchyNodes[k+3u];
   let v=q-sphere.xyz;
   let delay=loss.w+.5*loss.x*clamp(1.+dot(plane.xyz,v)*extra.x,0.,2.);
   let e=loss.z*smoothstep(plane.w+delay,plane.w+delay+loss.y,age);
   d=max(d,sphere.w-length(v)-e);
 }
 return field.source.y*smoothstep(0.,field.source.z,d)*base;
}
`;
