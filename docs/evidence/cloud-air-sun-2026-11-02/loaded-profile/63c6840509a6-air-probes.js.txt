/** Explicit Sun-air inspector. Lazy independent oracle; isolated GPU inputs and
 * outputs; no Scene writes, render, source clock or HTTP save. */
import {compileShader,hash} from './gpu.js';
import {AIR_SUN_PINS,airHalfValue,withAirGpuScopes} from './air.js';
const WIDTH=256,HEIGHT=64,TOLERANCE=.0015;
function requireValue(test,message){if(!test)throw Error('Sun-air probe: '+message);}
function words(values){const b=new ArrayBuffer(values.length*4),v=new DataView(b);values.forEach((x,i)=>v.setFloat32(i*4,x,true));return Array.from(new Uint32Array(b));}
function floatValues(input){const b=input instanceof Uint8Array?input:new Uint8Array(input),v=new DataView(b.buffer,b.byteOffset,b.byteLength);return Array.from({length:b.byteLength/4},(_,i)=>v.getFloat32(i*4,true));}
function finiteJSON(value){return Number.isFinite(value)?value:null;}
export function airProbeCases(oracle){
 requireValue(oracle.schema==='thrum.air-sun-oracle.v1'&&oracle.gpu_total_absolute_tolerance===TOLERANCE,'oracle schema/fixed target');
 requireValue(oracle.knots.shape[0]===HEIGHT&&oracle.knots.shape[1]===WIDTH&&oracle.knots.reference_rgb.length===WIDTH*HEIGHT,'oracle knot shape');
 const result=[];
 for(let row=0;row<HEIGHT;row++)for(let column=0;column<WIDTH;column++){
  const rgb=oracle.knots.reference_rgb[row*WIDTH+column];requireValue(rgb.length===3&&rgb.every(x=>Number.isFinite(x)&&x>=0&&x<=1),'finite knot oracle');
  result.push({id:'knot-'+(row*WIDTH+column),kind:'medium-table-knot',mode:'table_coordinate',input_u32:words([column/(WIDTH-1),row/(HEIGHT-1),0,2,0,0,0,0]),
   true_coordinates:[column/(WIDTH-1),row/(HEIGHT-1)],expected:{status:'medium-knot',valid:1,rgb}});
 }
 for(const q of oracle.queries){
  requireValue(['height_mu','world'].includes(q.mode)&&q.input_u32.length===8&&q.input_u32.every(x=>Number.isInteger(x)&&x>=0&&x<=0xffffffff),'exact oracle input words');
  requireValue(['visible','planet-blocked','unsupported'].includes(q.expected.status)&&q.expected.valid===(q.expected.status==='unsupported'?0:1),'oracle validity policy');
  const mode=floatValues(new Uint32Array(q.input_u32).buffer)[3];requireValue(mode===(q.mode==='world'?1:0),'oracle mode word');
  result.push({...q,id:'query-'+q.id});
 }
 return result;
}
export function airCpuCoordinates(height,mu,profile){
 const R=profile.calibration.radius_m,A=profile.calibration.top_m;
 if(height===A&&mu>=0)return [0,1];
 const r=R+height,rho=Math.sqrt(height*(2*R+height)),H=Math.sqrt(A*(2*R+A)),delta=A-height,C=delta*(2*R+A+height),b=r*mu,root=Math.sqrt(b*b+C);
 const difference=mu>=0?2*r*delta*(1-mu)/(root+b+delta):root-b-delta;
 return [difference/(rho+H-delta),rho/H];
}
export function airCpuWorldGeometry(point,sun,profile){
 const R=profile.calibration.radius_m,d=profile.domain;
 if(![...point,...sun].every(Number.isFinite)||Math.abs(point[0])>d.world_abs_xz_max_m||Math.abs(point[2])>d.world_abs_xz_max_m||point[1]<d.world_y_min_m||point[1]>d.world_y_max_m)return null;
 const length=Math.hypot(...sun);if(Math.abs(length-1)>d.unit_sun_length_tolerance)return null;
 const radial=[point[0],R+point[1],point[2]],radius=Math.hypot(...radial);
 const h=(point[0]**2+point[2]**2+point[1]*(2*R+point[1]))/(radius+R),mu=radial.reduce((n,x,i)=>n+x*sun[i]/length,0)/radius;
 return [h,mu];
}
export function airCpuTableLookup(tableBytes,coordinates){
 requireValue(coordinates.every(Number.isFinite)&&coordinates.every(x=>x>=-2e-6&&x<=1+2e-6),'manual lookup coordinate domain');
 const [x,y]=coordinates.map((v,i)=>Math.max(0,Math.min(1,v))*[WIDTH-1,HEIGHT-1][i]),ix=Math.floor(x),iy=Math.floor(y),fx=x-ix,fy=y-iy;
 const v=new DataView(tableBytes.buffer,tableBytes.byteOffset,tableBytes.byteLength),out=[];
 const at=(px,py,c)=>airHalfValue(v.getUint16(((py*WIDTH+px)*4+c)*2,true));
 for(let c=0;c<3;c++)out.push((1-fy)*((1-fx)*at(ix,iy,c)+fx*at(Math.min(ix+1,WIDTH-1),iy,c))+fy*((1-fx)*at(ix,Math.min(iy+1,HEIGHT-1),c)+fx*at(Math.min(ix+1,WIDTH-1),Math.min(iy+1,HEIGHT-1),c)));
 return out;
}
async function readBuffer(device,buffer){let staging,mapped=false;try{
 staging=device.createBuffer({label:'Sun-air output readback',size:buffer.size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
 const encoder=device.createCommandEncoder();encoder.copyBufferToBuffer(buffer,0,staging,0,buffer.size);device.queue.submit([encoder.finish()]);
 await staging.mapAsync(GPUMapMode.READ);mapped=true;return new Uint8Array(staging.getMappedRange()).slice();
}finally{if(mapped)staging.unmap();staging?.destroy();}}
async function readTable(device,table){let staging,mapped=false;try{
 staging=device.createBuffer({label:'Sun-air exact half table readback',size:131072,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
 const encoder=device.createCommandEncoder();encoder.copyTextureToBuffer({texture:table},{buffer:staging,bytesPerRow:2048,rowsPerImage:64},[256,64,1]);device.queue.submit([encoder.finish()]);
 await staging.mapAsync(GPUMapMode.READ);mapped=true;return new Uint8Array(staging.getMappedRange()).slice();
}finally{if(mapped)staging.unmap();staging?.destroy();}}
function metrics(rows,key){
 const measured=rows.filter(r=>Array.isArray(r[key])&&r[key].every(Number.isFinite)),max=[0,0,0];
 for(const row of measured)for(let c=0;c<3;c++)max[c]=Math.max(max[c],row[key][c]);
 return {measured:measured.length,max_absolute_rgb:measured.length?max:null};
}
// The declared component allocations are finite acceptance gates too: a small
// total must not hide cancellation between table/filter/mapping errors.
export function airProbeAcceptance(expectedStatus,statusPass,totalError,errors,allocations){
 const applies=expectedStatus==='visible'||expectedStatus==='medium-knot',validExpected=expectedStatus!=='unsupported';
 const measured=value=>Array.isArray(value)&&value.length===3&&value.every(Number.isFinite);
 const totalPass=!validExpected||(measured(totalError)&&totalError.every(x=>x<=TOLERANCE));
 const checks={},failed=[];
 if(!statusPass)failed.push('status');if(!totalPass)failed.push('total_T');
 for(const name of ['table_quadrature_storage','hardware_filtering','mapping_numerics']){
  const allocation=allocations[name],value=errors[name];
  requireValue(Number.isFinite(allocation)&&allocation>0,'declared component allocation '+name);
  const finite=measured(value),pass=applies?finite&&value.every(x=>x<=allocation):null;
  checks[name]={applicable:applies,measured:finite,allocation,pass};
  if(applies&&!pass)failed.push(name);
 }
 return {pass:failed.length===0,total_pass:totalPass,component_checks:checks,failing_components:failed};
}
export async function checkAirSun(device,air){
 requireValue(air&&!air.destroyed&&air.table&&air.uniform,'active Sun-air product');
 const latched=air.identity;
 const response=await fetch(air.oracleURL,{cache:'no-store'});requireValue(response.ok,'lazy oracle fetch');const oracleBytes=new Uint8Array(await response.arrayBuffer());
 requireValue(oracleBytes.byteLength===air.profile.files.oracle.bytes&&await hash(oracleBytes)===AIR_SUN_PINS.oracle,'independent oracle SHA/bytes');
 const oracle=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(oracleBytes));
 requireValue(oracle.profile_id===air.profile.profile_id&&oracle.source_hashes.air_reference===air.profile.source_hashes.air_reference&&oracle.source_hashes.optical_reference===air.profile.source_hashes.optical_reference,'independent reference/profile identity');
 const cases=airProbeCases(oracle),inputWords=new Uint32Array(cases.flatMap(q=>q.input_u32)),size=cases.length*32;
 requireValue(device.limits.maxStorageBufferBindingSize>=size&&device.limits.maxBufferSize>=size&&device.limits.maxStorageBuffersPerShaderStage>=2&&device.limits.maxBindingsPerBindGroup>24,'probe storage/binding limits');
 const code=air.queryWGSL+`
struct AirProbeInput {a:vec4f,b:vec4f}
@group(0) @binding(23) var<storage,read> airProbeInputs:array<AirProbeInput>;
@group(0) @binding(24) var<storage,read_write> airProbeOutputs:array<vec4f>;
@compute @workgroup_size(64) fn inspectAirSun(@builtin(global_invocation_id) id:vec3u){
 if(id.x>=arrayLength(&airProbeInputs)){return;}
 let q=airProbeInputs[id.x];var value=airSunInvalid();var hm=vec2f(-1.);var coordinates=vec2f(-1.);
 if(q.a.w==0.){
  hm=q.a.xy;value=airSunTransmittanceAtHeightMu(hm.x,hm.y);
  if(airSunClassifyHeightMu(hm.x,hm.y)==1){coordinates=airSunCoordinates(hm.x,hm.y);}
 }else if(q.a.w==1.){
  value=airSunTransmittance(q.a.xyz,q.b.xyz);let geometry=airSunWorldGeometry(q.a.xyz,q.b.xyz);
  if(geometry.valid){hm=vec2f(geometry.height,geometry.mu);if(airSunClassifyHeightMu(hm.x,hm.y)==1){coordinates=airSunCoordinates(hm.x,hm.y);}}
 }else if(q.a.w==2.){coordinates=q.a.xy;value=airSunSampleCoordinates(coordinates);}
 airProbeOutputs[id.x*2u]=value;airProbeOutputs[id.x*2u+1u]=vec4f(hm,coordinates);
}`;
 const {result,scope}=await withAirGpuScopes(device,async()=>{
  const owned=[];const own=x=>{owned.push(x);return x;};
  try{
   const input=own(device.createBuffer({label:'Sun-air isolated exact probe inputs',size,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST})),output=own(device.createBuffer({label:'Sun-air RGBT and mapping probes',size,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC}));
   device.queue.writeBuffer(input,0,inputWords);
   const V=GPUShaderStage.COMPUTE,layout=device.createBindGroupLayout({entries:[...air.layoutEntries(V),{binding:23,visibility:V,buffer:{type:'read-only-storage',minBindingSize:size}},{binding:24,visibility:V,buffer:{type:'storage',minBindingSize:size}}]});
   const pipeline=await device.createComputePipelineAsync({layout:device.createPipelineLayout({bindGroupLayouts:[layout]}),compute:{module:await compileShader(device,code),entryPoint:'inspectAirSun'}});
   const group=device.createBindGroup({layout,entries:[...air.bindings(),{binding:23,resource:{buffer:input}},{binding:24,resource:{buffer:output}}]});
   const encoder=device.createCommandEncoder(),pass=encoder.beginComputePass();pass.setPipeline(pipeline);pass.setBindGroup(0,group);pass.dispatchWorkgroups(Math.ceil(cases.length/64));pass.end();device.queue.submit([encoder.finish()]);
   const [outputs,table]=await Promise.all([readBuffer(device,output),readTable(device,air.table)]);return {outputs,table};
  }finally{for(const resource of owned)resource.destroy();}
 });
 const values=floatValues(result.outputs),tableHash=await hash(result.table),uploadedExact=tableHash===AIR_SUN_PINS.table;
 const rows=cases.map((q,i)=>{
  const v=values.slice(i*8,i*8+8),rgb=v.slice(0,3),expected=q.expected,valid=v[3]===1;
  const statusPass=expected.valid?valid&&rgb.every(x=>Number.isFinite(x)&&x>=0&&x<=1)&&(expected.status!=='planet-blocked'||rgb.every(x=>x===0)):v[3]===0&&rgb.every(x=>x===-1);
  const total=expected.valid&&valid?rgb.map((x,c)=>Math.abs(x-expected.rgb[c])):null;
  let tableError=null,filterError=null,mappingError=null,trueCoordinates=q.true_coordinates;
  if(!trueCoordinates&&expected.status==='visible'){
   const inputs=floatValues(new Uint32Array(q.input_u32).buffer),hm=q.mode==='world'?airCpuWorldGeometry(inputs.slice(0,3),inputs.slice(4,7),air.profile):inputs.slice(0,2);
   if(hm)trueCoordinates=airCpuCoordinates(...hm,air.profile);
  }
  if(trueCoordinates&&valid){
   const trueSample=airCpuTableLookup(air.recovery.tableBytes,trueCoordinates);
   tableError=trueSample.map((x,c)=>Math.abs(x-expected.rgb[c]));
   const gpuCoordinates=v.slice(6,8);
   if(gpuCoordinates.every(x=>Number.isFinite(x)&&x>=-2e-6&&x<=1+2e-6)){
    const gpuCoordinateSample=airCpuTableLookup(air.recovery.tableBytes,gpuCoordinates);
    filterError=rgb.map((x,c)=>Math.abs(x-gpuCoordinateSample[c]));mappingError=gpuCoordinateSample.map((x,c)=>Math.abs(x-trueSample[c]));
   }
  }
  const acceptance=airProbeAcceptance(expected.status,statusPass,total,
   {table_quadrature_storage:tableError,hardware_filtering:filterError,mapping_numerics:mappingError},air.profile.acceptance.experimental_allocations);
  return {index:i,id:q.id,kind:q.kind,mode:q.mode,expected_status:expected.status,input_u32:q.input_u32,expected_rgb:expected.rgb,gpu_rgb:rgb.map(finiteJSON),gpu_valid_word:v[3],gpu_height_mu:v.slice(4,6).map(finiteJSON),gpu_coordinates:v.slice(6,8).map(finiteJSON),
   status_pass:statusPass,total_error:total,table_error:tableError,hardware_filter_error:filterError,mapping_error:mappingError,
   ...acceptance};
 });
 const descriptors={},binaryData={table:result.table,inputs:new Uint8Array(inputWords.buffer).slice(),outputs:result.outputs};
 for(const [key,payload] of Object.entries(binaryData))descriptors[key]={filename:{table:'table.rgba16f',inputs:'inputs.u32',outputs:'outputs.f32'}[key],bytes:payload.byteLength,sha256:await hash(payload),shape:key==='table'?[64,256,4]:[cases.length,8],format:key==='table'?'little-endian binary16 RGBA':key==='inputs'?'little-endian uint32 exact float32 input bits':'little-endian float32 [RGB,valid,height,mu,x_mu,x_height]'};
 const selected=new Set(),include=row=>{if(row&&selected.size<128)selected.add(row.index);};
 for(const row of rows.filter(r=>r.expected_status==='unsupported'))include(row);
 for(const row of rows.filter(r=>r.mode==='world').slice(0,16))include(row);
 for(const row of rows.filter(r=>r.expected_status==='planet-blocked').slice(0,8))include(row);
 for(const key of ['total_error','table_error','hardware_filter_error','mapping_error'])for(let c=0;c<3;c++)include(rows.reduce((best,row)=>Array.isArray(row[key])&&(!best||row[key][c]>best[key][c])?row:best,null));
 for(const index of [0,255,256,WIDTH*HEIGHT-1,WIDTH*HEIGHT,Math.floor(rows.length*.97)])include(rows[index]);
 const failureNames=['status','total_T','table_quadrature_storage','hardware_filtering','mapping_numerics'];
 const failureCounts=group=>Object.fromEntries(failureNames.map(name=>[name,group.filter(row=>row.failing_components.includes(name)).length]));
 const componentSummary={};for(const name of failureNames.slice(2)){
  const applicable=rows.filter(row=>row.component_checks[name].applicable);
  componentSummary[name]={allocation:air.profile.acceptance.experimental_allocations[name],applicable:applicable.length,
   measured:applicable.filter(row=>row.component_checks[name].measured).length,
   missing_measurements:applicable.filter(row=>!row.component_checks[name].measured).length,
   failed:applicable.filter(row=>!row.component_checks[name].pass).length};
 }
 const groups={};for(const name of ['medium-table-knot','height_mu','world']){const group=rows.filter(r=>name==='medium-table-knot'?r.kind===name:r.mode===name);groups[name]={count:group.length,status_failures:group.filter(r=>!r.status_pass).length,failed:group.filter(r=>!r.pass).length,total_error:metrics(group,'total_error'),failing_components:failureCounts(group)};}
 const generationValid=!air.destroyed&&air.identity===latched;
 const receipt={schema:'thrum.air-sun-probe.v1',scope:'Finite actual table-upload, medium-knot filtering and Sun-only height/mu/world/invalid/ambiguous query checks; no physical sky, view ratios, A0, cloud appearance or performance pass',
  identity:{...latched,oracle:await hash(oracleBytes),probe_shader:await hash(new TextEncoder().encode(code))},source_reference_hashes:oracle.source_hashes,
  generation_valid:generationValid,gpu_scopes:scope,profile_id:air.profile.profile_id,uniform_u32:Array.from(air.uniformWords),
  acceptance:{total_absolute_T_tolerance:TOLERANCE,exact_uploaded_half_bytes_required:true,set_before_gpu:true,component_allocations_enforced:true,experimental_component_allocations:air.profile.acceptance.experimental_allocations},
  exact_uploaded_half_bytes:uploadedExact,query_count:cases.length,oracle_query_count:oracle.queries.length,groups,
  total_error:metrics(rows,'total_error'),table_quadrature_storage_error:metrics(rows,'table_error'),hardware_filter_error:metrics(rows,'hardware_filter_error'),mapping_error:metrics(rows,'mapping_error'),
  component_gate_summary:componentSummary,failing_components:failureCounts(rows),
  status_failures:rows.filter(r=>!r.status_pass).length,failed_queries:rows.filter(r=>!r.pass).length,sampled_rows:[...selected].map(i=>rows[i]),
  case_order:'First 16384 medium-only knots height-major/mu-minor, then queries.json queries in order. Knots include grazing LIMIT; height/mu and world APIs retain separate opaque/unsupported branches.',
  binaries:descriptors,bytes:{gpu_product:air.bytes,probe_input:size,probe_output:size,output_readback:size,table_readback:131072,lazy_oracle:oracleBytes.byteLength,retained_binaries:Object.values(binaryData).reduce((n,b)=>n+b.byteLength,0)},
  implementation_checks_pass:generationValid&&scope.pass&&uploadedExact&&rows.every(row=>row.pass),
  uniform_certificate:false,limits:['Total target is finite probe acceptance, not a uniform/backend or atmosphere-model certificate','The manual half lookup isolates table/filter/mapping errors; the independent Gauss fixture supplies the total reference','Opaque planet centre-direction visibility is not finite solar-disk coverage','Normal startup does not fetch the oracle; no Scene/source/cloud clock is modified']};
 return {receipt,binaries:binaryData};
}
