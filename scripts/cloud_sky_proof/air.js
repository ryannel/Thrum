/** Immutable, authored Sun-air table. No cloud clock or per-frame preparation.
 * This product is not a finite-segment ratio, A0 or physical-sky implementation.
 */
import {hash} from './gpu.js';
export const AIR_SUN_PINS=Object.freeze({
 profile:'e925609eed47278bd8982b29a1320289957ba1652b674b5bb8f838b8dde561b2',
 table:'e4bcb732eb12a664e207456c5d6c8d9d2c9f8f9e5331d1c20c416323e0a2bda9',
 oracle:'be7e2cb2537ca7d261529558643e4a5aaf98b12b0bfdbdf1acd43e97da1371b7',
});
const WIDTH=256,HEIGHT=64,TABLE_BYTES=131072;
const PROFILE_ID='earth-authored.sun-air.256x64.midpoint512.v1';
const MAPPING_ID='rho-local-height.top-distance.sun-only.v1';
function requireValue(test,message){if(!test)throw Error('Sun-air admission: '+message);}
export function airHalfValue(bits){
 const sign=(bits&0x8000)?-1:1,exponent=(bits>>>10)&31,mantissa=bits&1023;
 if(exponent===31)return mantissa?NaN:sign*Infinity;
 return sign*(exponent===0?mantissa*2**-24:(1+mantissa/1024)*2**(exponent-15));
}
async function fetchBytes(url){const response=await fetch(url,{cache:'no-store'});if(!response.ok)throw Error('Sun-air fetch failed: '+response.status+' '+url);return new Uint8Array(await response.arrayBuffer());}
function asBytes(input){return input instanceof Uint8Array?input:new Uint8Array(input);}
/** All byte/domain checks precede resource creation. Oracle is lazy probe-only. */
export async function verifyAirSunPackage(profileInput,tableInput){
 const profileBytes=asBytes(profileInput),tableBytes=asBytes(tableInput);
 const [profileHash,tableHash]=await Promise.all([hash(profileBytes),hash(tableBytes)]);
 requireValue(profileHash===AIR_SUN_PINS.profile&&tableHash===AIR_SUN_PINS.table,'pinned profile/table SHA mismatch');
 const profile=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(profileBytes));
 requireValue(profile.schema==='thrum.air-sun-profile.v1'&&profile.profile_id===PROFILE_ID&&profile.mapping_id===MAPPING_ID&&!profile.uniform_certificate,'profile/schema/mapping identity');
 requireValue(profile.dimensions.mu===WIDTH&&profile.dimensions.height===HEIGHT&&profile.format==='rgba16float','table dimensions/format');
 const p=profile.calibration;
 requireValue(p.radius_m===6371000&&p.top_m===100000&&p.rayleigh_height_m===8000&&p.aerosol_height_m===1200&&p.aerosol_scattering_per_m===3.996e-6&&p.aerosol_extinction_per_m===4.4e-6&&p.aerosol_g===.8&&p.ozone==='off','authored atmosphere calibration');
 requireValue(p.rayleigh_scattering_per_m.length===3&&p.rayleigh_scattering_per_m.every((v,i)=>v===[5.802e-6,13.558e-6,33.1e-6][i]),'Rayleigh coefficients');
 requireValue(profile.cook.midpoint_samples===512&&profile.acceptance.gpu_total_absolute_T===.0015&&profile.acceptance.exact_uploaded_half_bytes===true,'cook/evidence target');
 requireValue(profile.files.table.filename==='transmittance.rgba16f'&&profile.files.table.bytes===TABLE_BYTES&&profile.files.table.sha256===tableHash&&tableBytes.byteLength===TABLE_BYTES,'table payload descriptor');
 requireValue(profile.files.oracle.filename==='queries.json'&&profile.files.oracle.sha256===AIR_SUN_PINS.oracle&&profile.files.oracle.lazy_probe_only===true,'lazy oracle identity');
 const domain=profile.domain;
 for(const [key,value] of Object.entries({horizon_ambiguity_mu:1e-6,unit_sun_length_tolerance:1e-5,cosine_round_allowance:2e-6,coordinate_round_allowance:2e-6,world_abs_xz_max_m:100000,world_y_min_m:0,world_y_max_m:20000}))requireValue(domain[key]===value,'domain policy '+key);
 requireValue(domain.height_m[0]===0&&domain.height_m[1]===100000&&domain.mu[0]===-1&&domain.mu[1]===1,'primitive domain');
 const view=new DataView(tableBytes.buffer,tableBytes.byteOffset,tableBytes.byteLength);
 for(let i=0;i<WIDTH*HEIGHT*4;i++){
  const bits=view.getUint16(i*2,true),value=airHalfValue(bits);
  requireValue(Number.isFinite(value)&&value>=0&&value<=1,'finite unit-interval half payload');
  if(i%4===3)requireValue(bits===0x3c00,'table alpha must be exactly one');
 }
 return {profile,profileBytes,tableBytes,identity:{profile:profileHash,table:tableHash,oracle:AIR_SUN_PINS.oracle,mapping:MAPPING_ID,cooker:profile.source_hashes.cooker}};
}
function errorRecord(error){return {name:error?.constructor?.name??error?.name??'Error',message:String(error?.message??error)};}
function lossState(device){const state={info:null,failure:null};Promise.resolve(device.lost).then(info=>{state.info={reason:info?.reason??null,message:info?.message??null};},error=>{state.failure=errorRecord(error);});return state;}
/** Quiescent creation/probe guard; GPU errors must not yield zero-filled passes. */
export async function withAirGpuScopes(device,work){
 requireValue(typeof device.pushErrorScope==='function'&&typeof device.popErrorScope==='function'&&typeof device.lost?.then==='function'&&typeof device.queue.onSubmittedWorkDone==='function','GPU error scopes/device loss/completion API unsupported');
 const loss=lossState(device),stack=[],report={validation:null,out_of_memory:null,pop_failures:[],queue_failure:null,device_lost:null,device_lost_promise_failure:null,pass:false};let result,problem;
 try{for(const filter of ['validation','out-of-memory']){device.pushErrorScope(filter);stack.push(filter);}result=await work();}catch(error){problem=error;}
 finally{
  try{await device.queue.onSubmittedWorkDone();}catch(error){report.queue_failure=errorRecord(error);}
  while(stack.length){const filter=stack.pop();try{const error=await device.popErrorScope();if(error)report[filter==='validation'?'validation':'out_of_memory']=errorRecord(error);}catch(error){report.pop_failures.push({filter,...errorRecord(error)});}}
  await Promise.resolve();report.device_lost=loss.info;report.device_lost_promise_failure=loss.failure;
 }
 report.pass=!report.validation&&!report.out_of_memory&&!report.queue_failure&&!report.device_lost&&!report.device_lost_promise_failure&&report.pop_failures.length===0;
 if(problem||!report.pass){const error=new Error('Sun-air GPU operation failed'+(problem?': '+String(problem.message??problem):'')+'; gpu_scopes='+JSON.stringify(report),problem?{cause:problem}:undefined);error.gpuScopeErrors=report;throw error;}
 return {result,scope:report};
}
export class AirSun {
 static async create(device){
  const base=new URL('./fixtures/air-sun/',import.meta.url);
  // Never load the 1.25 MiB oracle at normal startup.
  const [profileBytes,tableBytes,shaderBytes]=await Promise.all([fetchBytes(new URL('profile.json',base)),fetchBytes(new URL('transmittance.rgba16f',base)),fetchBytes(new URL('./air.wgsl',import.meta.url))]);
  const admitted=await verifyAirSunPackage(profileBytes,tableBytes),result=new AirSun();
  result.device=device;result.profile=admitted.profile;result.recovery=admitted;result.oracleURL=new URL('queries.json',base);
  result.identity={...admitted.identity,shader:await hash(shaderBytes)};result.queryWGSL=new TextDecoder('utf-8',{fatal:true}).decode(shaderBytes);result.destroyed=false;
  result.bytes={table:TABLE_BYTES,uniform:48,totalGPU:TABLE_BYTES+48,cpuRecoveryPayload:profileBytes.byteLength+tableBytes.byteLength,cpuUniformSnapshot:48,shaderSourceUTF8:shaderBytes.byteLength,normalStartupFetchedPayload:profileBytes.byteLength+tableBytes.byteLength+shaderBytes.byteLength,normalStartupOracleBytes:0,uploadBytesPerRow:WIDTH*8,uploadRows:HEIGHT};
  try{
   const {scope}=await withAirGpuScopes(device,async()=>{
    const limits=device.limits;
    requireValue(limits.maxTextureDimension2D>=WIDTH&&limits.maxBufferSize>=48&&limits.maxUniformBufferBindingSize>=48&&limits.maxBindingsPerBindGroup>22&&limits.maxSamplersPerShaderStage>=1&&limits.maxSampledTexturesPerShaderStage>=1,'adapter texture/buffer/binding limits');
    result.table=device.createTexture({label:'Sun-air 256 x 64 RGBT',size:[WIDTH,HEIGHT],format:'rgba16float',usage:GPUTextureUsage.TEXTURE_BINDING|GPUTextureUsage.COPY_DST|GPUTextureUsage.COPY_SRC});
    result.tableView=result.table.createView();result.sampler=device.createSampler({minFilter:'linear',magFilter:'linear',mipmapFilter:'nearest',addressModeU:'clamp-to-edge',addressModeV:'clamp-to-edge'});
    result.uniform=device.createBuffer({label:'Sun-air optical profile 48 B',size:48,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});
    const p=result.profile.calibration,record=new Float32Array([p.radius_m,p.top_m,p.rayleigh_height_m,p.aerosol_height_m,...p.rayleigh_scattering_per_m,0,p.aerosol_scattering_per_m,p.aerosol_extinction_per_m,p.aerosol_g,0]);
    result.uniformWords=new Uint32Array(record.buffer).slice();device.queue.writeBuffer(result.uniform,0,record);
    device.queue.writeTexture({texture:result.table},tableBytes,{bytesPerRow:WIDTH*8,rowsPerImage:HEIGHT},[WIDTH,HEIGHT]);
   });
   result.creationGpuScopes=scope;return result;
  }catch(error){result.destroy();throw error;}
 }
 layoutEntries(visibility=GPUShaderStage.FRAGMENT){return [{binding:20,visibility,texture:{sampleType:'float',viewDimension:'2d'}},{binding:21,visibility,sampler:{type:'filtering'}},{binding:22,visibility,buffer:{type:'uniform',minBindingSize:48}}];}
 bindings(){if(this.destroyed)throw Error('Sun-air product retired');return [{binding:20,resource:this.tableView},{binding:21,resource:this.sampler},{binding:22,resource:{buffer:this.uniform}}];}
 destroy(){if(this.destroyed)return;this.destroyed=true;this.table?.destroy();this.uniform?.destroy();}
}
