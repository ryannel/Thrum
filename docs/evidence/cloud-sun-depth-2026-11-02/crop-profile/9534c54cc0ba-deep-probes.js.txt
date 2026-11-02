/** Quiescent finite deep-Sun capture. No render, clock advance, product writes or
 * HTTP saves. Probe submissions/readbacks are observer work, never a benchmark.
 */
import {compileShader,hash} from './gpu.js';
import {PACKED_LOAD_WGSL} from './scalar-field.js';
import {deepSunShaderSections} from './deep-sun.js';

const N=64,P=66,K=48,STEPS=512,ENDPOINTS=513;
const NODE_BYTES=P*P*K*8,METADATA_BYTES=P*P*32;
const SCENE_WGSL=`
struct Scene {
 lo:vec4f,extent:vec4f,camera:vec4f,lookAt:vec4f,
 sun:vec4f,U:vec4f,V:vec4f,W:vec4f,
 E:vec4f,sky:vec4f,grid:vec4u,switches:vec4u
}
@group(0) @binding(0) var density:texture_3d<f32>;
@group(0) @binding(1) var filtered:sampler;
@group(0) @binding(4) var<uniform> scene:Scene;
@group(0) @binding(5) var<storage,read_write> errors:array<atomic<u32>>;
fn sigmaAt(p:vec3f)->f32 {
 return samplePacked(density,filtered,(p-scene.lo.xyz)/(scene.extent.xyz/vec3f(scene.grid.xyz)),scene.grid.xyz);
}
fn rayBox(o:vec3f,d:vec3f,lo:vec3f,hi:vec3f)->vec2f {
 var near=-1e20;var far=1e20;
 for(var k=0;k<3;k++){
  if(abs(d[k])<1e-8){if(o[k]<lo[k]||o[k]>hi[k]){return vec2f(1.,-1.);}}
  else{let a=(lo[k]-o[k])/d[k];let b=(hi[k]-o[k])/d[k];near=max(near,min(a,b));far=min(far,max(a,b));}
 }
 return vec2f(near,far);
}
fn tauDirect(o:vec3f,d:vec3f,count:u32)->f32 {
 let halo=.5*scene.extent.xyz/vec3f(scene.grid.xyz);
 let interval=rayBox(o,d,scene.lo.xyz-halo,scene.lo.xyz+scene.extent.xyz+halo);
 let start=max(0.,interval.x);let end=max(start,interval.y);let h=(end-start)/f32(count);var tau=0.;
 for(var i=0u;i<count;i++){tau+=sigmaAt(o+d*(start+(f32(i)+.5)*h))*h;}
 return tau;
}`;
const PATH_ENTRY=`
@group(0) @binding(13) var<storage,read> capturedColumns:array<vec2u>;
@group(0) @binding(14) var<storage,read_write> capturedPaths:array<vec4f>;
@compute @workgroup_size(1) fn captureDeepEndpoints(@builtin(global_invocation_id) id:vec3u) {
 if(id.x>=arrayLength(&capturedColumns)){return;}
 let selected=capturedColumns[id.x];let base=id.x*(DEEP_STEPS+1u);
 let shape=vec3u(scene.grid.xy+vec2u(2u),scene.grid.z/4u+select(0u,1u,scene.grid.z%4u!=0u)+2u);
 if(!deepFrameValid()||any(textureDimensions(deepDensity)!=shape)){
  for(var i=0u;i<=DEEP_STEPS;i++){capturedPaths[base+i]=vec4f(-1.,-1.,0.,0.);}return;
 }
 // Match builder expression ordering, quadrature positions and accumulation.
 let halfSpan=deepHalf();let span=2.*halfSpan.z;
 let xy=((vec2f(selected)-.5)/f32(DEEP_N)-.5)*2.*halfSpan.xy;
 let origin=scene.lo.xyz+scene.extent.xyz*.5+scene.U.xyz*xy.x+scene.V.xyz*xy.y-scene.W.xyz*halfSpan.z;
 let h=span/f32(DEEP_STEPS);var tau=0.;
 capturedPaths[base]=vec4f(0.,0.,0.,1.);
 for(var step=0u;step<DEEP_STEPS;step++){
  let sigma=deepSigma(origin+scene.W.xyz*((f32(step)+.5)*h));tau+=sigma*h;
  let valid=deepFinite(sigma)&&sigma>=0.&&deepFinite(tau)&&tau>=0.;
  capturedPaths[base+step+1u]=vec4f(f32(step+1u)*h,tau,sigma,select(0.,1.,valid));
 }
}`;
const QUERY_ENTRY=`
@group(0) @binding(13) var<storage,read> capturedPoints:array<vec4f>;
@group(0) @binding(14) var<storage,read_write> capturedTerms:array<vec4f>;
@compute @workgroup_size(32) fn captureDeepQueries(@builtin(global_invocation_id) id:vec3u){
 if(id.x>=arrayLength(&capturedPoints)){return;}
 let p=capturedPoints[id.x].xyz;let terms=deepSunTerms(p);let direct=tauDirect(p,scene.sun.xyz,512u);
 let valid=deepFinite(terms.x)&&deepFinite(terms.y)&&all(terms>=vec2f(0.))&&all(terms<=vec2f(1.))&&deepFinite(direct)&&direct>=0.;
 capturedTerms[id.x]=vec4f(terms,direct,select(0.,1.,valid));
}`;
function requireValue(test,message){if(!test)throw Error('Deep probe: '+message);}
function bytesOf(value){
 if(value instanceof Uint8Array)return value;
 if(value instanceof ArrayBuffer)return new Uint8Array(value);
 if(ArrayBuffer.isView(value))return new Uint8Array(value.buffer,value.byteOffset,value.byteLength);
 throw TypeError('Deep probe requires binary view');
}
function f32Values(value){const b=bytesOf(value),v=new DataView(b.buffer,b.byteOffset,b.byteLength);return Array.from({length:b.byteLength/4},(_,i)=>v.getFloat32(i*4,true));}
function sceneBytes(words){
 if(Array.isArray(words)){
  requireValue(words.length===48&&words.every(x=>Number.isInteger(x)&&x>=0&&x<=0xffffffff),'48 exact u32 Scene words');
  const result=new Uint8Array(192),v=new DataView(result.buffer);words.forEach((x,i)=>v.setUint32(i*4,x,true));return result;
 }
 const result=bytesOf(words).slice();requireValue(result.byteLength===192,'Scene192 exact byte length');return result;
}
async function sourceBytes(relative){
 const response=await fetch(new URL(relative,import.meta.url),{cache:'no-store'});
 if(!response.ok)throw Error('Deep probe source fetch '+response.status+' '+relative);
 return new Uint8Array(await response.arrayBuffer());
}
async function readStorage(device,buffer,size){
 let staging,mapped=false;
 try{
  staging=device.createBuffer({label:'deep probe buffer readback',size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  const encoder=device.createCommandEncoder();encoder.copyBufferToBuffer(buffer,0,staging,0,size);device.queue.submit([encoder.finish()]);
  await staging.mapAsync(GPUMapMode.READ);mapped=true;return new Uint8Array(staging.getMappedRange()).slice();
 }finally{if(mapped)staging.unmap();staging?.destroy();}
}
// Tight actual texture readback, with all staging retired even on map failure.
async function readTexture(device,texture,[width,height,depth]){
 const tight=width*8,stride=Math.ceil(tight/256)*256,size=stride*height*depth;let staging,mapped=false;
 try{
  staging=device.createBuffer({label:'deep packed field readback',size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  const encoder=device.createCommandEncoder();encoder.copyTextureToBuffer({texture},{buffer:staging,bytesPerRow:stride,rowsPerImage:height},[width,height,depth]);device.queue.submit([encoder.finish()]);
  await staging.mapAsync(GPUMapMode.READ);mapped=true;const src=new Uint8Array(staging.getMappedRange()),out=new Uint8Array(tight*height*depth);
  for(let row=0;row<height*depth;row++)out.set(src.subarray(row*stride,row*stride+tight),row*tight);
  return out;
 }finally{if(mapped)staging.unmap();staging?.destroy();}
}
function add(a,b){return a.map((v,i)=>v+b[i]);}
function scale(a,s){return a.map(v=>v*s);}
function dot(a,b){return a.reduce((n,v,i)=>n+v*b[i],0);}

/** Pure finite plan helper for CPU/mock checks; never qualifies GPU arithmetic. */
export function deepProbePlan(nodesInput,metadataInput,uniformWords){
 const raw=sceneBytes(uniformWords),view=new DataView(raw.buffer),u=Array.from({length:48},(_,i)=>view.getUint32(i*4,true)),f=f32Values(raw);
 const nodes=f32Values(nodesInput),records=f32Values(metadataInput);
 requireValue(nodes.length===P*P*K*2&&records.length===P*P*8,'actual node/metadata lengths');
 const grid=u.slice(40,43),lo=f.slice(0,3),extent=f.slice(4,7),U=f.slice(20,23),V=f.slice(24,27),W=f.slice(28,31);
 requireValue(grid.every(x=>x>0)&&extent.every(x=>Number.isFinite(x)&&x>0)&&[...lo,...U,...V,...W].every(Number.isFinite),'finite Scene domain');
 const halfSupport=extent.map((v,i)=>v*.5+v/grid[i]*.5),rawHalf=[U,V,W].map(b=>dot(b.map(Math.abs),halfSupport));
 const half=[rawHalf[0]/(1-4/N),rawHalf[1]/(1-4/N),rawHalf[2]],centre=add(lo,scale(extent,.5));
 const columns=Array.from({length:P*P},(_,index)=>{
  const record=records.slice(index*8,index*8+8),count=record[0];
  const shaped=Number.isInteger(count)&&count>=2&&count<=K;
  const last=shaped?nodes[(index*K+count-1)*2+1]:NaN;
  return {physical_xy:[index%P,Math.floor(index/P)],linear_index:index,count,compression_valid:record[1]===1,
   nominal_bounds:record.slice(2,4),work:{coefficient_requests:record[4],endpoint_visits:record[5],anchor_builds:record[6],failure_bits:record[7]},final_tau:Number.isFinite(last)?last:null};
 });
 const chosen=[];const push=(column,kind)=>{if(column&&chosen.length<16&&!chosen.some(c=>c.linear_index===column.linear_index))chosen.push({...column,selection_kind:kind});};
 const clear=columns.filter(c=>c.compression_valid&&c.final_tau===0);
 push(clear[0],'actual-clear');push(clear.at(-1),'actual-clear');
 for(const index of [0,P-1,(P-1)*P,P*P-1])push(columns[index],'guard-corner');
 const active=columns.filter(c=>c.compression_valid&&c.final_tau>0);
 const highest=[...active].sort((a,b)=>b.count-a.count||a.linear_index-b.linear_index);
 for(const column of highest.slice(0,4))push(column,'highest-knot-count');
 for(const fraction of [.15,.35,.5,.65,.85])push(active[Math.floor((active.length-1)*fraction)],'mid-active');
 push(columns.find(c=>!c.compression_valid),'invalid-if-present');
 for(const index of [32*P+32,32*P+33,33*P+32,33*P+33])push(columns[index],'centre');
 requireValue(chosen.length>0&&chosen.length<=16,'finite selected columns');
 const rows=[];
 const world=(x,y,depth)=>add(add(add(centre,scale(U,x)),scale(V,y)),scale(W,depth-half[2]));
 const emit=(x,y,depth,kind,extra={})=>{if(rows.length<128)rows.push({kind,...extra,light_xy_m:[x,y],projected_depth_m:depth,world_m:world(x,y,depth).map(Math.fround)});};
 const xy=c=>c.physical_xy.map((v,i)=>((v-.5)/N-.5)*2*half[i]);
 for(const column of chosen){const p=xy(column);for(const z of [.125,.375,.625,.875])emit(...p,2*half[2]*z,'selected-column-interior',{column_xy:column.physical_xy});}
 for(const column of chosen.slice(0,4)){const p=xy(column);for(const z of [-.01,0,1,1.01])emit(...p,2*half[2]*z,'column-terminal',{column_xy:column.physical_xy});}
 for(const column of chosen.slice(0,8)){const p=xy(column).map((v,i)=>v+half[i]/N);emit(...p,half[2],'between-four-columns',{column_xy:column.physical_xy});}
 const edgeEpsilon=Math.max(.0001,Math.min(...extent.map((v,i)=>v/grid[i]))*.001);
 for(let axis=0;axis<2;axis++)for(const sign of [-1,1])for(const location of [rawHalf[axis]-edgeEpsilon,rawHalf[axis],rawHalf[axis]+edgeEpsilon,half[axis]-edgeEpsilon,half[axis]+edgeEpsilon]){
  const p=[0,0];p[axis]=sign*location;emit(...p,half[2],'critical-lateral-edge',{edge_axis:axis,edge_sign:sign});
 }
 for(const column of highest.slice(0,4)){
  const p=xy(column),middle=Math.floor(column.count/2),depth=nodes[(column.linear_index*K+middle)*2];
  for(const offset of [-edgeEpsilon,edgeEpsilon])emit(...p,depth+offset,'retained-knot-neighbour',{column_xy:column.physical_xy});
 }
 let seed=0x74c21;
 const random=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296;};
 for(let i=0;i<8;i++)emit((random()*2-1)*rawHalf[0],(random()*2-1)*rawHalf[1],random()*2*half[2],'seeded-interior');
 const finiteRows=rows.filter(row=>row.world_m.every(Number.isFinite));requireValue(finiteRows.length===rows.length,'finite GPU query points');
 const counts=columns.filter(c=>c.compression_valid).map(c=>c.count),failures={};
 for(const c of columns)failures[String(c.work.failure_bits)]=(failures[String(c.work.failure_bits)]??0)+1;
 return {uniform_u32:u,uniform_f32:f,logical_dimensions:grid,lo_m:lo,extent_m:extent,basis:{U,V,W},projection:{raw_half_m:rawHalf,half_m:half,guard_m:rawHalf.map(x=>1e-5*Math.max(x,1)),nominal_pitch_m:half.slice(0,2).map(x=>2*x/N),derived_from_xyz_ignoring_scene_basis_w:true},
  source_time_s:f[3],selected_columns:chosen,query_points:rows,metadata_summary:{columns:P*P,valid:counts.length,invalid:P*P-counts.length,count_min:counts.length?Math.min(...counts):null,count_max:counts.length?Math.max(...counts):null,failure_bits_histogram:failures,
   coefficient_requests:columns.reduce((n,c)=>n+c.work.coefficient_requests,0),endpoint_visits:columns.reduce((n,c)=>n+c.work.endpoint_visits,0),anchor_builds:columns.reduce((n,c)=>n+c.work.anchor_builds,0)}};
}

export async function captureDeepSun(device,deep,field,sceneBuffer,lastFrame,uniformWords){
 requireValue(deep&&!deep.destroyed&&deep.nodes&&deep.metadata,'active deep product');
 requireValue(lastFrame&&Number.isFinite(lastFrame.source_time_s)&&lastFrame.depth_model==='deep','committed deep frame');
 const scene=sceneBytes(uniformWords),level=field?.levels?.[0]??field,texture=level?.texture;
 requireValue(texture&&texture.format==='rgba16float'&&texture.dimension==='3d','actual packed base field');
 requireValue(sceneBuffer?.size>=192,'Scene192 buffer');
 const logical=[...new Uint32Array(scene.buffer).slice(40,43)],physical=[texture.width,texture.height,texture.depthOrArrayLayers];
 requireValue(physical.every((v,i)=>v===[logical[0]+2,logical[1]+2,Math.ceil(logical[2]/4)+2][i]),'Scene and actual texture layout mismatch');
 if(level.dimensions)requireValue(level.dimensions.every((v,i)=>v===logical[i]),'base logical dimensions mismatch');
 requireValue(deep.nodes.size===NODE_BYTES&&deep.metadata.size===METADATA_BYTES,'selected K48/32B product layout');
 const latched=deep.identity,frameFingerprint=JSON.stringify(lastFrame),frameSnapshot=JSON.parse(frameFingerprint);
 requireValue(lastFrame.deep_identity?.shader===latched.shader,'frame/deep shader generation mismatch');
 const sourceNames=['deep-probes.js','deep-sun.js','deep-sun.wgsl','scalar-field.js','gpu.js','../cloud_coupled_proof/source.js'];
 const sources=await Promise.all(sourceNames.map(sourceBytes)),sourceHashes=await Promise.all(sources.map(hash));
 const sourceIdentity=Object.fromEntries(sourceNames.map((name,i)=>[name,sourceHashes[i]]));
 requireValue(sourceIdentity['deep-sun.wgsl']===latched.shader,'served/compiled deep shader mismatch');
 const sections=deepSunShaderSections(new TextDecoder('utf-8',{fatal:true}).decode(sources[2]));
 requireValue(deep.queryWGSL===sections.queryWGSL,'compiled query generation mismatch');
 const [nodeBytes,metadataBytes,fieldBytes]=await Promise.all([readStorage(device,deep.nodes,NODE_BYTES),readStorage(device,deep.metadata,METADATA_BYTES),readTexture(device,texture,physical)]);
 const plan=deepProbePlan(nodeBytes,metadataBytes,scene);
 // Preserve one material-space witness across changes to the field allocation.
 // Keep the deterministic finite query ceiling; this is observer data only.
 if(lastFrame.reference_point_m?.length===3&&lastFrame.reference_point_m.every(Number.isFinite)){
  const point=lastFrame.reference_point_m.map(Math.fround);
  const delta=point.map((v,i)=>v-(plan.lo_m[i]+plan.extent_m[i]*.5));
  const row={kind:'pinned-material-witness',world_m:point,
   light_xy_m:[dot(delta,plan.basis.U),dot(delta,plan.basis.V)],
   projected_depth_m:dot(delta,plan.basis.W)+plan.projection.half_m[2]};
  if(plan.query_points.length===128)plan.query_points.pop();
  plan.query_points.push(row);
 }
 requireValue(plan.source_time_s===Math.fround(lastFrame.source_time_s),'source clock/Scene mismatch');
 for(const [key,value] of [['lo_m',plan.lo_m],['extent_m',plan.extent_m],['sun_direction',plan.uniform_f32.slice(16,19)]]){
  requireValue(lastFrame[key]?.length===3&&lastFrame[key].every((v,i)=>Math.fround(v)===value[i]),'frame/Scene '+key+' mismatch');
 }
 requireValue(plan.uniform_f32[19]>0,'actual directional light product required');
 const pathsCode=sections.buildWGSL+PATH_ENTRY,queriesCode=PACKED_LOAD_WGSL+SCENE_WGSL+deep.queryWGSL+QUERY_ENTRY;
 const resources=[];const own=buffer=>{resources.push(buffer);return buffer;};
 let pathBytes,queryBytes,counterBytes;
 try{
  const pathSize=plan.selected_columns.length*ENDPOINTS*16,querySize=plan.query_points.length*16;
  const pathInput=own(device.createBuffer({label:'selected deep physical columns',size:plan.selected_columns.length*8,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST}));
  const pathOutput=own(device.createBuffer({label:'actual GPU cumulative endpoint paths',size:pathSize,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC}));
  const queryInput=own(device.createBuffer({label:'finite deep query points',size:querySize,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST}));
  const queryOutput=own(device.createBuffer({label:'deep/direct query terms',size:querySize,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC}));
  const counters=own(device.createBuffer({label:'isolated deep probe counters',size:16,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC|GPUBufferUsage.COPY_DST}));
  device.queue.writeBuffer(pathInput,0,new Uint32Array(plan.selected_columns.flatMap(c=>c.physical_xy)));
  device.queue.writeBuffer(queryInput,0,new Float32Array(plan.query_points.flatMap(q=>[...q.world_m,0])));
  device.queue.writeBuffer(counters,0,new Uint32Array(4));
  const V=GPUShaderStage.COMPUTE,baseLayout=[{binding:0,visibility:V,texture:{sampleType:'float',viewDimension:'3d'}},{binding:1,visibility:V,sampler:{type:'filtering'}},{binding:4,visibility:V,buffer:{type:'uniform',minBindingSize:192}}];
  const io=(inputSize,outputSize)=>[{binding:13,visibility:V,buffer:{type:'read-only-storage',minBindingSize:inputSize}},{binding:14,visibility:V,buffer:{type:'storage',minBindingSize:outputSize}}];
  const pathLayout=device.createBindGroupLayout({entries:[...baseLayout,...io(pathInput.size,pathOutput.size)]});
  const queryLayout=device.createBindGroupLayout({entries:[...baseLayout,{binding:5,visibility:V,buffer:{type:'storage',minBindingSize:16}},...deep.layoutEntries(V),...io(queryInput.size,queryOutput.size)]});
  const [pathModule,queryModule]=await Promise.all([compileShader(device,pathsCode),compileShader(device,queriesCode)]);
  const [pathPipeline,queryPipeline]=await Promise.all([
   device.createComputePipelineAsync({layout:device.createPipelineLayout({bindGroupLayouts:[pathLayout]}),compute:{module:pathModule,entryPoint:'captureDeepEndpoints'}}),
   device.createComputePipelineAsync({layout:device.createPipelineLayout({bindGroupLayouts:[queryLayout]}),compute:{module:queryModule,entryPoint:'captureDeepQueries'}}),
  ]);
  const baseEntries=[{binding:0,resource:texture.createView()},{binding:1,resource:deep.sampler},{binding:4,resource:{buffer:sceneBuffer}}];
  const pathGroup=device.createBindGroup({layout:pathLayout,entries:[...baseEntries,{binding:13,resource:{buffer:pathInput}},{binding:14,resource:{buffer:pathOutput}}]});
  const queryGroup=device.createBindGroup({layout:queryLayout,entries:[...baseEntries,{binding:5,resource:{buffer:counters}},...deep.bindings(),{binding:13,resource:{buffer:queryInput}},{binding:14,resource:{buffer:queryOutput}}]});
  const encoder=device.createCommandEncoder(),pathPass=encoder.beginComputePass();pathPass.setPipeline(pathPipeline);pathPass.setBindGroup(0,pathGroup);pathPass.dispatchWorkgroups(plan.selected_columns.length);pathPass.end();
  const queryPass=encoder.beginComputePass();queryPass.setPipeline(queryPipeline);queryPass.setBindGroup(0,queryGroup);queryPass.dispatchWorkgroups(Math.ceil(plan.query_points.length/32));queryPass.end();device.queue.submit([encoder.finish()]);
  [pathBytes,queryBytes,counterBytes]=await Promise.all([readStorage(device,pathOutput,pathSize),readStorage(device,queryOutput,querySize),readStorage(device,counters,16)]);
 }finally{for(const resource of resources)resource.destroy();}
 const paths=f32Values(pathBytes),query=f32Values(queryBytes),counters=Array.from(new Uint32Array(counterBytes.buffer,counterBytes.byteOffset,4));
 const binaryData={nodes:nodeBytes,metadata:metadataBytes,field:fieldBytes,paths:pathBytes};
 const names={nodes:'nodes.f32',metadata:'metadata.f32',field:'field.rgba16f',paths:'paths.f32'};
 const shapes={nodes:[P,P,K,2],metadata:[P,P,8],field:[physical[2],physical[1],physical[0],4],paths:[plan.selected_columns.length,ENDPOINTS,4]};
 const descriptors={};for(const key of Object.keys(binaryData))descriptors[key]={filename:names[key],bytes:binaryData[key].byteLength,sha256:await hash(binaryData[key]),shape:shapes[key],format:key==='field'?'little-endian float16':'little-endian float32'};
 const queryRows=plan.query_points.map((point,i)=>({...point,terms:query.slice(i*4,i*4+2),direct_tau_512:query[i*4+2],valid:query[i*4+3]===1}));
 const pathRows=plan.selected_columns.map((column,i)=>{
  const start=i*ENDPOINTS*4,end=start+ENDPOINTS*4;let valid=0;
  for(let k=start+3;k<end;k+=4)if(paths[k]===1)valid++;
  return {physical_xy:column.physical_xy,endpoint_count:ENDPOINTS,valid_endpoint_count:valid,first:paths.slice(start,start+4),last:paths.slice(end-4,end)};
 });
 const generationValid=!deep.destroyed&&deep.identity===latched&&JSON.stringify(lastFrame)===frameFingerprint&&level.texture===texture&&(!field.levels||field.levels[0]===level)&&sourceIdentity['deep-sun.wgsl']===deep.identity.shader;
 requireValue(generationValid,'generation changed during quiescent capture');
 const receipt={schema:'thrum.deep-sun-probe.v1',scope:'Finite actual GPU input-stream/compression and filtered query/direct-path capture; no appearance, full SKY-G, source quality, uniform numerical certificate or performance pass',
  quiescent_caller_required:true,generation_valid:generationValid,uniform_provenance:'exact caller-supplied192B snapshot of the Scene used for source/light/view; existing uniform buffer not independently read back',
  identity:{deep:{...latched},fit:lastFrame.fit_identity??null,sources:sourceIdentity,shaders:{path_sha256:await hash(new TextEncoder().encode(pathsCode)),query_sha256:await hash(new TextEncoder().encode(queriesCode))},uniform_sha256:await hash(scene)},
  frame:frameSnapshot,source_time_s:plan.source_time_s,uniform_u32:plan.uniform_u32,logical_dimensions:plan.logical_dimensions,field_physical_dimensions:physical,
  scene:{lo_m:plan.lo_m,extent_m:plan.extent_m,sun_direction:plan.uniform_f32.slice(16,19),basis:plan.basis},profile:{...deep.profile},projection:plan.projection,
  layout:{nodes:'[physical_y,physical_x,knot,depth_m/cumulative_tau]',metadata:{stride_bytes:32,words:['count','compression_valid','nominal_primary_bound','nominal_kappa_bound','coefficient_requests','endpoint_visits','anchor_builds','failure_bits'],word_type:'all f32'},
   field:'[physical_z,physical_y,physical_x,RGBA16F]; X fastest, four consecutive logical Z per RGBA; one packed texel halo, tight rows',paths:'[selected_column_index,endpoint_index,depth_m/cumulative_tau/sigma_midpoint_per_m/valid]; endpoint0=[0,0,0,1]'},
  binaries:descriptors,metadata_summary:plan.metadata_summary,selected_columns:plan.selected_columns,path_summaries:pathRows,
  query:{rows:queryRows,count:queryRows.length,valid_count:queryRows.filter(row=>row.valid).length,counters:{unused_view_caps:counters[0],fallback_requests:counters[1],invalid_footprints:counters[2],unsupported:counters[3]},
   operator:'physical g=64*(dot(p-centre,U/V)/(2*Huv)+.5)+.5; floor/fract four neighbours, interpolate each retained ordinary tau at dot(p-centre,W)+Hw, average exp(-tau) and exp(-.25tau) separately',
   boundary:'clear before -rawW-guardW or laterally beyond Huv+guard; otherwise invalidtap requests one bounded direct path; domainflag1/invalidframe or fallback request>=4096 returnsnegative invalid',
   direct:'512 uniform midpoint samples on finite half-voxel source ray-box intersection toward scene.sun.xyz; same packed field; separate quadrature from full-projection input stream'},
  bytes:{retained_binaries:Object.values(binaryData).reduce((n,b)=>n+b.byteLength,0),query_readback:queryBytes.byteLength,probe_counter_readback:16,field_readback_staging:Math.ceil(physical[0]*8/256)*256*physical[1]*physical[2],deep_product:deep.bytes},
  checks:{capture_inputs_and_layout:true,compression_quality:'OFFLINE_VALIDATOR_REQUIRED',source_quadrature:'UNQUALIFIED',lateral_filtering_quality:'UNQUALIFIED',radiance_appearance:'UNQUALIFIED',performance:'NOT_MEASURED'},
  limits:['Paths are a replay using the builder exact deepSigma/frame/quadrature expressions; original build input stream was not retained','Compression numerical reserve2e-6 is an unqualified candidate, not a universal GPU certificate','Query direct reference shares packed source reconstruction; it does not qualify canonical-source or atmospheric/full-sky accuracy','Probe counters are isolated from the rendered graph and have no per-query atomic ordering meaning']};
 return {receipt,binaries:binaryData};
}
