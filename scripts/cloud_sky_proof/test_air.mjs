// CPU tests/mocks only. Fake GPU outputs below test observer plumbing, NEVER GPU accuracy.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {webcrypto} from 'node:crypto';
import {AirSun,AIR_SUN_PINS,verifyAirSunPackage,withAirGpuScopes} from './air.js';
import {checkAirSun,airProbeCases,airCpuWorldGeometry,airCpuCoordinates,airCpuTableLookup,airProbeAcceptance} from './air-probes.js';
globalThis.crypto??=webcrypto;
globalThis.GPUBufferUsage={UNIFORM:1,COPY_DST:2,COPY_SRC:4,MAP_READ:8,STORAGE:16};
globalThis.GPUTextureUsage={TEXTURE_BINDING:1,COPY_DST:2,COPY_SRC:4};
globalThis.GPUShaderStage={COMPUTE:4,FRAGMENT:2};globalThis.GPUMapMode={READ:1};
let fetched=[];
globalThis.fetch=async url=>{fetched.push(String(url));const data=await readFile(url);return {ok:true,arrayBuffer:async()=>data.buffer.slice(data.byteOffset,data.byteOffset+data.byteLength)};};
const base=new URL('./fixtures/air-sun/',import.meta.url);
const profileBytes=new Uint8Array(await readFile(new URL('profile.json',base))),tableBytes=new Uint8Array(await readFile(new URL('transmittance.rgba16f',base))),oracle=JSON.parse(await readFile(new URL('queries.json',base),'utf8'));
const profile=JSON.parse(new TextDecoder().decode(profileBytes)),cases=airProbeCases(oracle);
function floatWords(input){return Array.from(new Float32Array(new Uint32Array(input).buffer));}
function mock(config={}){
 const objects=[],stack=[],calls=[];let usedMapFailure=false;
 const device={objects,calls,config,cases,lost:config.lost?Promise.resolve({reason:'destroyed',message:'mock loss'}):new Promise(()=>{}),
  limits:{maxTextureDimension2D:8192,maxBufferSize:268435456,maxUniformBufferBindingSize:65536,maxBindingsPerBindGroup:1000,maxSamplersPerShaderStage:16,maxSampledTexturesPerShaderStage:16,maxStorageBufferBindingSize:134217728,maxStorageBuffersPerShaderStage:8,...config.limits},
  pushErrorScope(filter){calls.push('push:'+filter);if(config.pushFailure&&filter==='out-of-memory')throw Error('mock push');stack.push(filter);},
  async popErrorScope(){const filter=stack.pop();calls.push('pop:'+filter);if(config.popFailure&&filter==='out-of-memory')throw Error('mock pop');return config.scopeError===filter?{message:'mock '+filter}:null;},
  createTexture(d){assert.equal(stack.length,2);const t={...d,width:d.size[0],height:d.size[1],dimension:'2d',data:new Uint8Array(131072),createView(){return {texture:this};},destroy(){this.dead=true;}};objects.push(t);return t;},
  createSampler(d){return d;},
  createBuffer(d){assert.equal(stack.length,2);if(config.bufferFailure)throw Error('mock buffer');const b={...d,data:new ArrayBuffer(d.size),async mapAsync(){if(config.mapFailure&&!usedMapFailure){usedMapFailure=true;throw Error('mock map');}},getMappedRange(){return this.data;},unmap(){},destroy(){this.dead=true;}};objects.push(b);return b;},
  createShaderModule({code}){calls.push('compile');assert(code.includes('inspectAirSun'));assert(!/\bmeta\b/.test(code));return {async getCompilationInfo(){return {messages:config.compileFailure?[{type:'error',lineNum:1,linePos:1,message:'mock compile'}]:[]};}};},
  createBindGroupLayout(d){return d;},createPipelineLayout(d){return d;},createBindGroup(d){return d;},async createComputePipelineAsync(d){return d;},
  createCommandEncoder(){const operations=[];return {beginComputePass(){let group;return {setPipeline(){},setBindGroup(_i,g){group=g;},dispatchWorkgroups(){const g=group;operations.push(()=>{
   const output=g.entries.find(e=>e.binding===24).resource.buffer,out=new Float32Array(output.data);
   for(let i=0;i<cases.length;i++){
    const q=cases[i],v=floatWords(q.input_u32),expected=q.expected;let hm=[-1,-1],coordinates=[-1,-1];
    if(q.mode==='table_coordinate')coordinates=v.slice(0,2);
    else if(q.mode==='height_mu')hm=v.slice(0,2);else hm=airCpuWorldGeometry(v.slice(0,3),v.slice(4,7),profile)??[-1,-1];
    if(expected.status==='visible')coordinates=airCpuCoordinates(...hm,profile);
    const value=expected.valid?[...expected.rgb,1]:[-1,-1,-1,0];out.set([...value,...hm,...coordinates],i*8);
   }
   if(config.allZero)out.fill(0);
   if(config.badRgb)out[0]+=.01;
   if(config.componentCancellation){out[200*8+6]=0;out[200*8+7]=0;}
  });},end(){}};},
  copyBufferToBuffer(a,ao,b,bo,size){operations.push(()=>new Uint8Array(b.data,bo,size).set(new Uint8Array(a.data,ao,size)));},
  copyTextureToBuffer({texture},{buffer}){operations.push(()=>new Uint8Array(buffer.data).set(texture.data));},finish(){return operations;}};},
  queue:{writeBuffer(buffer,offset,input){new Uint8Array(buffer.data,offset,input.byteLength).set(new Uint8Array(input.buffer,input.byteOffset,input.byteLength));},
   writeTexture({texture},bytes,layout,size){assert.equal(layout.bytesPerRow,2048);assert.deepEqual(size,[256,64]);texture.data.set(bytes);},
   submit(commands){for(const operations of commands)for(const operation of operations)operation();},async onSubmittedWorkDone(){if(config.queueFailure)throw Error('mock queue');}},
 };
 return device;
}

test('pinned package admission rejects byte and manifest changes',async()=>{
 const p=await verifyAirSunPackage(profileBytes,tableBytes);assert.equal(p.identity.table,AIR_SUN_PINS.table);
 const changed=tableBytes.slice();changed[0]^=1;await assert.rejects(()=>verifyAirSunPackage(profileBytes,changed),/SHA mismatch/);
 const bad=JSON.parse(new TextDecoder().decode(profileBytes));bad.dimensions.mu=257;await assert.rejects(()=>verifyAirSunPackage(new TextEncoder().encode(JSON.stringify(bad)),tableBytes),/SHA mismatch/);
});
test('normal creation is lazy, has48B ABI, and retires idempotently',async()=>{
 fetched=[];const device=mock(),air=await AirSun.create(device);assert(!fetched.some(url=>url.endsWith('queries.json')));assert(!device.calls.includes('compile'));
 assert.equal(air.bytes.totalGPU,131120);assert.equal(air.uniform.size,48);assert.deepEqual(air.bindings().map(x=>x.binding),[20,21,22]);
 assert.equal(air.layoutEntries()[2].buffer.minBindingSize,48);assert.deepEqual(Array.from(new Float32Array(air.uniform.data).slice(0,4)),[6371000,100000,8000,1200]);
 air.destroy();air.destroy();assert(device.objects.every(x=>x.dead));assert.throws(()=>air.bindings(),/retired/);
});
test('creation failures clean partial objects and retire every pushed scope',async()=>{
 for(const config of [{bufferFailure:true},{limits:{maxTextureDimension2D:255}},{scopeError:'validation'},{scopeError:'out-of-memory'},{lost:true},{popFailure:true},{pushFailure:true},{queueFailure:true}]){
  const device=mock(config);await assert.rejects(()=>AirSun.create(device));assert(device.objects.every(x=>x.dead));assert.equal(device.calls.filter(x=>x.startsWith('pop:')).length,config.pushFailure?1:2);
 }
 await assert.rejects(()=>withAirGpuScopes({queue:{}},async()=>{}),/unsupported/);
});
test('stable world height, strict bounds, and manual half lookup match independent fine fixtures',()=>{
 assert(Math.abs(airCpuWorldGeometry([0,.001,0],[0,1,0],profile)[0]-.001)<1e-12);
 assert(airCpuWorldGeometry([100000,0,0],[0,1,0],profile)[0]>700);
 assert.equal(airCpuWorldGeometry([0,-.001,0],[0,1,0],profile),null);assert.equal(airCpuWorldGeometry([0,0,0],[0,2,0],profile),null);
 let maximum=0;
 for(const q of cases){
  if(!q.expected.valid||q.expected.status==='planet-blocked')continue;
  const v=floatWords(q.input_u32);let coordinates=q.true_coordinates;
  if(!coordinates){const hm=q.mode==='world'?airCpuWorldGeometry(v.slice(0,3),v.slice(4,7),profile):v.slice(0,2);coordinates=airCpuCoordinates(...hm,profile);}
  const actual=airCpuTableLookup(tableBytes,coordinates);maximum=Math.max(maximum,...actual.map((x,c)=>Math.abs(x-q.expected.rgb[c])));
 }
 assert(maximum<.0005);assert.equal(cases.length,16384+oracle.queries.length);
});
test('probe receipt preserves raw input/output/table identities and fixed error target',async()=>{
 const device=mock(),air=await AirSun.create(device),result=await checkAirSun(device,air);
 assert(result.receipt.implementation_checks_pass);assert(result.receipt.exact_uploaded_half_bytes);assert.equal(result.receipt.acceptance.total_absolute_T_tolerance,.0015);
 assert.equal(result.binaries.table.byteLength,131072);assert.equal(result.binaries.inputs.byteLength,cases.length*32);assert.equal(result.binaries.outputs.byteLength,cases.length*32);
 assert(result.receipt.groups['medium-table-knot'].count===16384);assert(result.receipt.sampled_rows.length<=128);assert(result.receipt.gpu_scopes.pass);
 assert(device.objects.filter(x=>x!==air.table&&x!==air.uniform).every(x=>x.dead));air.destroy();assert(device.objects.every(x=>x.dead));
});
test('zero outputs, out-of-tolerance RGB and wrong actual table never pass',async()=>{
 for(const config of [{allZero:true},{badRgb:true},{}]){
  const device=mock(config),air=await AirSun.create(device);if(!config.allZero&&!config.badRgb)air.table.data[0]^=1;
  const result=await checkAirSun(device,air);assert.equal(result.receipt.implementation_checks_pass,false);if(Object.keys(config).length===0)assert.equal(result.receipt.exact_uploaded_half_bytes,false);air.destroy();assert(device.objects.every(x=>x.dead));
 }
});
test('probe shader/readback/GPU scope failures reject and clean observer resources',async()=>{
 for(const fault of ['compileFailure','mapFailure','gpuValidation']){
  const device=mock(),air=await AirSun.create(device);if(fault==='gpuValidation')device.config.scopeError='validation';else device.config[fault]=true;
  await assert.rejects(()=>checkAirSun(device,air));assert(device.objects.filter(x=>x!==air.table&&x!==air.uniform).every(x=>x.dead));air.destroy();assert(device.objects.every(x=>x.dead));
 }
});

test('component gates reject cancellation even when total error passes',async()=>{
 const device=mock({componentCancellation:true}),air=await AirSun.create(device),{receipt}=await checkAirSun(device,air);
 assert(receipt.exact_uploaded_half_bytes&&receipt.gpu_scopes.pass);
 assert(Math.max(...receipt.total_error.max_absolute_rgb)<.0015);
 assert.equal(receipt.implementation_checks_pass,false);
 assert(receipt.failing_components.hardware_filtering>0&&receipt.failing_components.mapping_numerics>0);
 const row=receipt.sampled_rows.find(row=>row.index===200);assert(row);
 assert(row.total_pass&&row.total_error.every(x=>x<.0015));
 assert(row.failing_components.includes('hardware_filtering')&&row.failing_components.includes('mapping_numerics'));
 assert.equal(receipt.acceptance.component_allocations_enforced,true);
 air.destroy();assert(device.objects.every(x=>x.dead));
});
test('each declared allocation is inclusive, named, and never skipped for a visible query',()=>{
 const limits=profile.acceptance.experimental_allocations,zero=[0,0,0],names=Object.keys(limits);
 for(const name of names){
  const errors=Object.fromEntries(names.map(key=>[key,zero]));errors[name]=[.00050001,0,0];
  const result=airProbeAcceptance('visible',true,[.0001,0,0],errors,limits);
  assert.equal(result.pass,false);assert(result.total_pass);assert.deepEqual(result.failing_components,[name]);
  errors[name]=[.0005,0,0];assert(airProbeAcceptance('medium-knot',true,[.0001,0,0],errors,limits).pass);
 }
 const missing=airProbeAcceptance('visible',true,zero,{table_quadrature_storage:zero,hardware_filtering:null,mapping_numerics:zero},limits);
 assert.deepEqual(missing.failing_components,['hardware_filtering']);assert.equal(missing.component_checks.hardware_filtering.measured,false);
 assert(airProbeAcceptance('planet-blocked',true,zero,{},limits).pass);
 assert(airProbeAcceptance('unsupported',true,null,{},limits).pass);
});
