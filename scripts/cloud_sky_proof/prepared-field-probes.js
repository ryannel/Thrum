/** Quiescent allocation-equivalence inspector. Private source products only;
 * no viewer state, render, source clock advance, HTTP save or timing campaign.
 */
import {PreparedCu,preparedAllocation,PREPARED_SUPPORT_IDENTITY} from './prepared-cu.js';
import {FIELD_PARAMETERS_WGSL} from './scalar-field.js';
import {compileShader,hash} from './gpu.js';
function requireValue(test,message){if(!test)throw Error('Prepared field inspector: '+message);}
function wordView(bytes){
 const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength),result=new Uint16Array(bytes.byteLength/2);
 for(let i=0;i<result.length;i++)result[i]=view.getUint16(i*2,true);return result;
}
function shape(dimensions){return [dimensions[0]+2,dimensions[1]+2,Math.ceil(dimensions[2]/4)+2];}
function load(words,physical,dimensions,x,y,z){
 if(x<0||y<0||z<0||x>=dimensions[0]||y>=dimensions[1]||z>=dimensions[2])return 0;
 return words[(((Math.floor(z/4)+1)*physical[1]+y+1)*physical[0]+x+1)*4+z%4];
}
function scan(words,dimensions,visit){
 const physical=shape(dimensions);let index=0;
 for(let pz=0;pz<physical[2];pz++)for(let py=0;py<physical[1];py++)for(let px=0;px<physical[0];px++)for(let channel=0;channel<4;channel++){
  const x=px-1,y=py-1,z=(pz-1)*4+channel;
  visit(words[index],index++,x,y,z,px,py,pz,channel,x>=0&&y>=0&&z>=0&&x<dimensions[0]&&y<dimensions[1]&&z<dimensions[2]);
 }
}
/** Pure exact binary16-codeword comparison, including guards and Z padding. */
export function comparePreparedFields(fullBytes,cropBytes,fullDimensions,cropDimensions,logicalOffset){
 const fullPhysical=shape(fullDimensions),cropPhysical=shape(cropDimensions),full=wordView(fullBytes),crop=wordView(cropBytes);
 requireValue(full.length===fullPhysical.reduce((a,b)=>a*b,1)*4&&crop.length===cropPhysical.reduce((a,b)=>a*b,1)*4,'tight packed readback shape');
 requireValue(logicalOffset.every(Number.isInteger),'integral crop offset at this mip');
 let mismatch=0,logicalMismatch=0,cropGuard=0,fullGuard=0,outside=0,fullNonfinite=0,cropNonfinite=0,fullNonzero=0,cropNonzero=0;
 const rows=[],outsideRows=[];
 scan(crop,cropDimensions,(actual,index,x,y,z,px,py,pz,channel,interior)=>{
  const expected=interior?load(full,fullPhysical,fullDimensions,x+logicalOffset[0],y+logicalOffset[1],z+logicalOffset[2]):0;
  if(interior&&actual!==0)cropNonzero++;
  if((actual&0x7c00)===0x7c00)cropNonfinite++;
  if(!interior&&actual!==0)cropGuard++;
  if(actual!==expected){mismatch++;if(interior)logicalMismatch++;if(rows.length<16)rows.push({physical:[px,py,pz,channel],logical:[x,y,z],full_logical:[x+logicalOffset[0],y+logicalOffset[1],z+logicalOffset[2]],actual_half_bits:actual,expected_half_bits:expected});}
 });
 scan(full,fullDimensions,(actual,index,x,y,z,px,py,pz,channel,interior)=>{
  if(interior&&actual!==0)fullNonzero++;
  if((actual&0x7c00)===0x7c00)fullNonfinite++;
  if(!interior&&actual!==0)fullGuard++;
  if(interior&&(x<logicalOffset[0]||y<logicalOffset[1]||z<logicalOffset[2]||x>=logicalOffset[0]+cropDimensions[0]||y>=logicalOffset[1]+cropDimensions[1]||z>=logicalOffset[2]+cropDimensions[2])&&actual!==0){
   outside++;if(outsideRows.length<16)outsideRows.push({logical:[x,y,z],half_bits:actual});
  }
 });
 return {pass:mismatch===0&&cropGuard===0&&fullGuard===0&&outside===0&&fullNonfinite===0&&cropNonfinite===0,
  criterion:'Exact binary16 codewords in cropped logical subregion; all guards/padding and full logical exterior exactly +0; no infinity/NaN',
  full_dimensions:fullDimensions,crop_dimensions:cropDimensions,logical_offset:logicalOffset,full_physical:fullPhysical,crop_physical:cropPhysical,
  logical_compared:cropDimensions.reduce((a,b)=>a*b,1),crop_allocated_codewords:crop.length,full_scanned_codewords:full.length,
  mismatch_count:mismatch,logical_mismatch_count:logicalMismatch,crop_guard_nonzero:cropGuard,full_guard_nonzero:fullGuard,full_outside_crop_nonzero:outside,
  full_logical_nonzero_codewords:fullNonzero,crop_logical_nonzero_codewords:cropNonzero,
  full_nonfinite:fullNonfinite,crop_nonfinite:cropNonfinite,mismatch_rows:rows,outside_rows:outsideRows};
}
async function readTexture(device,texture,physical){
 const [w,h,d]=physical,tight=w*8,stride=Math.ceil(tight/256)*256;let staging,mapped=false;
 try{
  staging=device.createBuffer({label:'prepared field mip inspector readback',size:stride*h*d,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  const encoder=device.createCommandEncoder();encoder.copyTextureToBuffer({texture},{buffer:staging,bytesPerRow:stride,rowsPerImage:h},physical);device.queue.submit([encoder.finish()]);
  await staging.mapAsync(GPUMapMode.READ);mapped=true;const source=new Uint8Array(staging.getMappedRange()),out=new Uint8Array(tight*h*d);
  for(let row=0;row<h*d;row++)out.set(source.subarray(row*stride,row*stride+tight),row*tight);return out;
 }finally{if(mapped)staging.unmap();staging?.destroy();}
}
async function readStorage(device,buffer){
 let staging,mapped=false;
 try{
  staging=device.createBuffer({label:'prepared canonical inspector readback',size:buffer.size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  const encoder=device.createCommandEncoder();encoder.copyBufferToBuffer(buffer,0,staging,0,buffer.size);device.queue.submit([encoder.finish()]);
  await staging.mapAsync(GPUMapMode.READ);mapped=true;return new Uint8Array(staging.getMappedRange()).slice();
 }finally{if(mapped)staging.unmap();staging?.destroy();}
}
function queryPoints(){
 const points=[[0,0,0],[512,512,512],[256,144,260],[130,58,110],[382,230,410],[129,57,109],[383,231,411],[128,48,112],[384,240,416],[-1,144,260],[513,144,260],
 [134,62,114],[378,226,406],[131,59,111],[381,229,409],[132,60,112]];
 let seed=0x1a9e73;const next=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296;};
 while(points.length<48)points.push([next()*512,next()*512,next()*512]);return points.map(q=>q.map(Math.fround));
}
async function canonicalProbe(device,entries,time){
 const points=queryPoints(),size=points.length*16,owned=[];const own=x=>{owned.push(x);return x;};
 try{
  const shader=entries[0].source.shader;requireValue(entries.every(e=>e.source.shader===shader),'canonical source shader must be allocation-independent');
  const code=FIELD_PARAMETERS_WGSL+shader+`
@group(0) @binding(9) var<storage,read> preparedPoints:array<vec4f>;
@group(0) @binding(10) var<storage,read_write> preparedValues:array<vec4f>;
@compute @workgroup_size(64) fn inspectPreparedCanonical(@builtin(global_invocation_id) id:vec3u){
 if(id.x<arrayLength(&preparedPoints)){let value=canonicalSigma(preparedPoints[id.x].xyz,field.source.x);preparedValues[id.x]=vec4f(value,field.source.x,select(0.,1.,value==value&&value>=0.&&value<1.),0.);}
}`;
  const input=own(device.createBuffer({label:'prepared canonical fixed master-domain points',size,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST}));
  device.queue.writeBuffer(input,0,new Float32Array(points.flatMap(q=>[...q,0])));
  const pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,code),entryPoint:'inspectPreparedCanonical'}});
  const products=entries.map(entry=>{
   const output=own(device.createBuffer({label:'prepared canonical '+entry.role,size,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC}));
   const group=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[...entry.source.probeEntries(),{binding:9,resource:{buffer:input}},{binding:10,resource:{buffer:output}}]});
   return {...entry,output,group};
  });
  const encoder=device.createCommandEncoder(),pass=encoder.beginComputePass();pass.setPipeline(pipeline);
  for(const product of products){pass.setBindGroup(0,product.group);pass.dispatchWorkgroups(1);}pass.end();device.queue.submit([encoder.finish()]);
  const bytes=await Promise.all(products.map(p=>readStorage(device,p.output))),f=bytes.map(b=>new DataView(b.buffer,b.byteOffset,b.byteLength));
  const rows=points.map((q,i)=>{
   const actual=f.map(v=>v.getFloat32(i*16,true)),bits=f.map(v=>v.getUint32(i*16,true)),expected=entries[0].source.sample(q,time);
   const valid=f.every(v=>v.getFloat32(i*16+8,true)===1&&v.getFloat32(i*16+4,true)===Math.fround(time));
   return {point_m:q,canonical_cpu:expected,gpu_values:actual,gpu_f32_bits:bits,gpu_allocation_parity:valid&&bits.every(x=>x===bits[0]),cpu_absolute_errors:actual.map(x=>Math.abs(x-expected))};
  });
  return {scope:'Arbitrary-position canonical decoder is independent of field crop/producer specialization; CPU discrepancy reported separately',
   roles:products.map(p=>p.role),shader_sha256:await hash(new TextEncoder().encode(code)),output_sha256:await Promise.all(bytes.map(hash)),
   pass:rows.every(r=>r.gpu_allocation_parity),rows,max_cpu_absolute_error:Math.max(...rows.flatMap(r=>r.cpu_absolute_errors)),cpu_error_qualification:'reported only; no new threshold or source-quality admission'};
 }finally{for(const resource of owned)resource.destroy();}
}
async function sourceIdentity(){
 const result={};
 for(const file of ['prepared-cu.js','prepared-field-probes.js','scalar-field.js','../cloud_source_proof/source.js']){
  const response=await fetch(new URL(file,import.meta.url),{cache:'no-store'});requireValue(response.ok,'source identity fetch '+file);
  result[file]=await hash(new Uint8Array(await response.arrayBuffer()));
 }
 return result;
}
function errorRecord(error){return {name:error?.constructor?.name??error?.name??'Error',message:String(error?.message??error)};}
// Isolate device.lost's pending callback from the capture's large source objects.
function trackDeviceLoss(device){
 const state={info:null,promiseFailure:null};
 Promise.resolve(device.lost).then(info=>{state.info={reason:info?.reason??null,message:info?.message??null};},error=>{state.promiseFailure=errorRecord(error);});
 return state;
}
async function withCaptureScopes(device,work){
 requireValue(typeof device?.pushErrorScope==='function'&&typeof device?.popErrorScope==='function'&&typeof device?.lost?.then==='function'&&typeof device?.queue?.onSubmittedWorkDone==='function','GPU validation/OOM error scopes, device.lost and queue completion are required; unsupported capture API');
 const loss=trackDeviceLoss(device),stack=[],report={validation:null,out_of_memory:null,device_lost:null,device_lost_promise_failure:null,queue_completion_failure:null,push_failure:null,pop_failures:[],filters:['validation','out-of-memory'],pass:false};
 let result,problem;
 try{
  // Both scopes precede even private source/resource creation. Keep each
  // successful push on the stack so a later push failure still retires it.
  for(const filter of report.filters){
   try{device.pushErrorScope(filter);stack.push(filter);}catch(error){report.push_failure={filter,...errorRecord(error)};throw error;}
  }
  result=await work();
 }catch(error){problem=error;}
 finally{
  try{await device.queue.onSubmittedWorkDone();}catch(error){report.queue_completion_failure=errorRecord(error);}
  while(stack.length){
   const filter=stack.pop();
   try{
    const error=await device.popErrorScope();
    if(error)report[filter==='validation'?'validation':'out_of_memory']=errorRecord(error);
   }catch(error){report.pop_failures.push({filter,...errorRecord(error)});}
  }
  // Let an already-resolved device.lost notification run before admission.
  await Promise.resolve();report.device_lost=loss.info;report.device_lost_promise_failure=loss.promiseFailure;
 }
 report.pass=!report.validation&&!report.out_of_memory&&!report.device_lost&&!report.device_lost_promise_failure&&!report.queue_completion_failure&&!report.push_failure&&report.pop_failures.length===0;
 if(problem||!report.pass){
  const error=new Error('Prepared field capture failed'+(problem?': '+String(problem.message??problem):' GPU observer scope')+'; capture_gpu_scopes='+JSON.stringify(report),problem?{cause:problem}:undefined);
  error.captureScopeErrors=report;throw error;
 }
 result.capture_gpu_scopes=report;return result;
}
export async function capturePreparedFieldComparison(device,nominalSide,age){
 preparedAllocation(nominalSide);requireValue(Number.isFinite(age)&&age>=0&&age<=240,'finite source age0..240');
 return withCaptureScopes(device,async()=>{
 const time=Math.fround(age),entries=[];
 try{
  const create=async(role,options)=>{const source=await PreparedCu.create(device,nominalSide,options);entries.push({role,source});source.prepare(time);return source;};
  const generic=await create('full-generic',{crop:false,specialize128:false});
  const full=nominalSide===128?await create('full-specialized',{crop:false,specialize128:true}):generic;
  const crop=await create(nominalSide===128?'crop-specialized':'crop-generic',{crop:true,specialize128:true});
  requireValue(entries.every(e=>e.source.recipe.bounds.min.every(x=>x===0)&&e.source.recipe.bounds.extent.every(x=>x===512)),'master-domain recipe changed');
  const encoder=device.createCommandEncoder(),pass=encoder.beginComputePass();for(const e of entries)e.source.encode(pass);pass.end();device.queue.submit([encoder.finish()]);for(const e of entries)e.source.markSubmitted();
  const levels=[];
  for(let level=0;level<3;level++){
   const fullLevel=full.field.levels[level],cropLevel=crop.field.levels[level],genericLevel=generic.field.levels[level];
   const fullBytes=await readTexture(device,fullLevel.texture,fullLevel.physical),cropBytes=await readTexture(device,cropLevel.texture,cropLevel.physical);
   const offset=crop.allocationProfile.logicalMin.map(x=>x/2**level);
   const comparison=comparePreparedFields(fullBytes,cropBytes,fullLevel.dimensions,cropLevel.dimensions,offset);
   let specialization=null;
   if(nominalSide===128){
    const genericBytes=await readTexture(device,genericLevel.texture,genericLevel.physical);
    specialization={...comparePreparedFields(genericBytes,fullBytes,genericLevel.dimensions,fullLevel.dimensions,[0,0,0]),generic_sha256:await hash(genericBytes),specialized_sha256:await hash(fullBytes)};
   }
   levels.push({level,full_sha256:await hash(fullBytes),crop_sha256:await hash(cropBytes),full_bytes:fullBytes.byteLength,crop_bytes:cropBytes.byteLength,
    full_world:{min:fullLevel.lo,pitch:fullLevel.voxelPitch,extent:fullLevel.extent},crop_world:{min:cropLevel.lo,pitch:cropLevel.voxelPitch,extent:cropLevel.extent},
    full_vs_crop:comparison,generic_vs_specialized:specialization});
  }
  const canonical=await canonicalProbe(device,entries,time),identities=await sourceIdentity();
  const profiles=entries.map(({role,source})=>({role,allocation:source.allocationProfile,field_bounds:source.fieldBounds,
   encoded_source_bytes:source.bytes.byteLength,field_bytes:source.field.levels.reduce((n,l)=>n+l.bytes,0),
   field_shader_sha256:null,canonical_shader_sha256:null}));
  for(let i=0;i<profiles.length;i++){
   profiles[i].field_shader_sha256=await hash(new TextEncoder().encode(entries[i].source.fieldShader));
   profiles[i].canonical_shader_sha256=await hash(new TextEncoder().encode(entries[i].source.shader));
  }
  return {schema:'thrum.prepared-field-allocation-probe.v1',scope:'Finite actual GPU full/cropped packed-field/mip/guard codeword equivalence and separate canonical-query parity; no appearance, lateral-quality, full-sky or performance pass',
   nominal_side:nominalSide,requested_age_s:age,source_time_s:time,source_calibration:{sigma_max_per_m:generic.sigmaMax,calibration_only:true,production_source_admission:false},
   source_support_identity:PREPARED_SUPPORT_IDENTITY,source_hashes:identities,master_bounds:generic.recipe.bounds,allocation_profiles:profiles,levels,canonical_point_parity:canonical,
   pass:levels.every(l=>l.full_vs_crop.pass&&(!l.generic_vs_specialized||l.generic_vs_specialized.pass))&&canonical.pass,
   bytes:{private_source_payloads:profiles.reduce((n,p)=>n+p.encoded_source_bytes,0),private_packed_fields:profiles.reduce((n,p)=>n+p.field_bytes,0),source_and_field_uniforms:entries.length*(64+3*64+2*16),
    retained_raw_field_payloads:0,readback_policy:'Per-mip sequential full/crop/(128generic); compact hashes and bounded mismatch witnesses retained. Backend residency/peak and timing not measured.'},
   limits:['All-time support follows the pinned CPU metadata proof; these finite GPU ages do not replace that proof','Exact crop/mip codewords depend on four-cell-aligned offsets and fixed original pitch at each mip','The128-only producer shortcut is checked against full generic all-codeword output; canonical source queries always retain trilinear semantics','Private inspector allocations/submissions are observer work and never part of a frame-time claim']};
 }finally{
  const errors=[];for(const entry of entries){try{entry.source.destroy();}catch(error){errors.push(error);}}
  if(errors.length)throw new AggregateError(errors,'Prepared private-source cleanup failed');
 }
 });
}
