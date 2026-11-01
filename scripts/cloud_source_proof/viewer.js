// Source-only WebGPU instrument: selected packed layout + portable R16F copy.
// No engine integration or complete Thrum rendering claim.
const $ = (id) => document.getElementById(id);
const canvas = document.querySelector('canvas');
const clip = new URLSearchParams(location.search).get('clip') || '../../tmp/cloud-source-proof/authored-ranked/';
const base = new URL(clip, location.href);
let playing = false, age = 90, prior = performance.now(), measuring = false;
let device, pipeline, computeGroup, pack, densityTexture, lightTexture, lightPipeline, lightGroup, renderPipeline, renderGroup, context, uniform;
let m, densityBytes, flowBytes, paddedFlow, stamp, queryRead, queryResolve;
let recording = false, recorder, recordedChunks=[], recordStart, recordSamples=[];
let instrumentHash='', computeHash='';
let sampleIdentity = '', lastRenderTime = performance.now(), cadences = [];
const shader = `
struct Params { headA:u32, bodyA:u32, headB:u32, bodyB:u32, flowOffset:u32, side:u32, theta:f32, duration:f32, scales:vec4<f32>, mode:u32, pad0:u32, pad1:u32,pad2:u32 }
@group(0) @binding(0) var<storage,read> density:array<u32>;
@group(0) @binding(1) var<storage,read> flow:array<u32>;
@group(0) @binding(2) var<uniform> p:Params;
@group(0) @binding(3) var<storage,read_write> result:array<u32>;
fn byteAt(i:u32)->u32 { return (density[i/4u] >> ((i%4u)*8u))&255u; }
fn wordAt(i:u32)->u32 {return density[i/4u];}
fn voxel(c:vec3<i32>,head:u32,body:u32)->f32 {
 if(any(c<vec3<i32>(0))||any(c>=vec3<i32>(i32(p.side)))){return 0.;}
 let q=vec3<u32>(c);let n=p.side/8u;
 let brick=(q.z/8u*n+q.y/8u)*n+q.x/8u;
 let pos=head+brick*2u;let index=byteAt(pos)|(byteAt(pos+1u)<<8u);
 if(index==65535u){return 0.;}
 let local=(q.z%8u*8u+q.y%8u)*8u+q.x%8u;
 var code=0u;
 if(p.pad1==1u){
 let pointer=wordAt(body+index*4u);let block=head+(pointer&0xfffffffeu);
 if((pointer&1u)==0u){code=byteAt(block+local);}else{
 let wi=local/32u;let bit=local%32u;let mask=wordAt(block+wi*4u);
 if((mask&(1u<<bit))==0u){return 0.;}
 let prefixAt=block+64u+(local/64u)*2u;
 var rank=byteAt(prefixAt)|(byteAt(prefixAt+1u)<<8u);
 if((wi&1u)==1u){rank+=countOneBits(wordAt(block+(wi-1u)*4u));}
 rank+=countOneBits(mask&((1u<<bit)-1u));code=byteAt(block+80u+rank);
 }}else{code=byteAt(body+index*512u+local);}
 return f32(code)*(p.scales.w/255.);
}
fn sigma(q:vec3<f32>,head:u32,body:u32)->f32 {
 let xyz=q*f32(p.side)/512.-.5;let lo=vec3<i32>(floor(xyz));let f=fract(xyz);var total=0.;
 for(var z=0;z<2;z++){for(var y=0;y<2;y++){for(var x=0;x<2;x++){
 let r=vec3<i32>(x,y,z);let w=select(vec3<f32>(1.)-f,f,vec3<bool>(x==1,y==1,z==1));
 total+=voxel(lo+r,head,body)*w.x*w.y*w.z;
 }}}return total;
}
fn velocity(q:vec3<f32>)->vec3<f32>{
 let xyz=clamp(q*(32./512.)-.5,vec3<f32>(0.),vec3<f32>(31.));let lo=vec3<u32>(floor(xyz));let hi=min(lo+1u,vec3<u32>(31u));let f=fract(xyz);var v=vec3<f32>(0.);
 for(var z=0u;z<2u;z++){for(var y=0u;y<2u;y++){for(var x=0u;x<2u;x++){
 let c=select(lo,hi,vec3<bool>(x==1u,y==1u,z==1u));let bits=flow[p.flowOffset+(c.z*32u+c.y)*32u+c.x];
 let signed=vec3<i32>(i32(bits<<24u)>>24,i32(bits<<16u)>>24,i32(bits<<8u)>>24);
 let w=select(vec3<f32>(1.)-f,f,vec3<bool>(x==1u,y==1u,z==1u));
 v+=vec3<f32>(signed)/127.*p.scales.xyz*w.x*w.y*w.z;
 }}}return v;
}
fn evaluate(index:u32)->f32 {
 let n=p.side;let q=(vec3<f32>(f32(index%n),f32((index/n)%n),f32(index/(n*n)))+.5)*(512./f32(n));
 if(p.pad0==1u){let c=vec3<i32>(i32(index%n),i32((index/n)%n),i32(index/(n*n)));return mix(voxel(c,p.headA,p.bodyA),voxel(c,p.headB,p.bodyB),p.theta);}
 let v=velocity(q);let a=-p.theta*p.duration;let b=(1.-p.theta)*p.duration;
 let qa=q+a*velocity(q+.5*a*v);let qb=q+b*velocity(q+.5*b*v);
 return (1.-p.theta)*sigma(qa,p.headA,p.bodyA)+p.theta*sigma(qb,p.headB,p.bodyB);
}
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let index=id.x*2u;if(index>=p.side*p.side*p.side){return;}
 result[id.x]=pack2x16float(vec2<f32>(evaluate(index),evaluate(index+1u)));
}`;
const lightShader = `
@group(0) @binding(0) var field:texture_3d<f32>;
@group(0) @binding(1) var<storage,read_write> result:array<u32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let x=(id.x%64u)*2u;let z=id.x/64u;if(z>=128u){return;}var tau=vec2<f32>(0.);
 for(var y=127;y>=0;y--){let d=vec2<f32>(textureLoad(field,vec3<i32>(i32(x),y,i32(z)),0).x,textureLoad(field,vec3<i32>(i32(x+1u),y,i32(z)),0).x)*4.;
 let mid=tau+.5*d;let source=.75*exp(-mid*.8)+.15*exp(-mid*.16)+vec2<f32>(.13);tau+=d;
 result[(z*128u+u32(y))*64u+x/2u]=pack2x16float(source);
 }
}`;
const renderShader = `
struct Params { headA:u32,bodyA:u32,headB:u32,bodyB:u32,flowOffset:u32,side:u32,theta:f32,duration:f32,scales:vec4<f32>,mode:u32,pad0:u32,pad1:u32,pad2:u32 }
@group(0) @binding(0) var field:texture_3d<f32>;
@group(0) @binding(1) var light:texture_3d<f32>;
@group(0) @binding(2) var filtering:sampler;
@group(0) @binding(3) var<uniform> p:Params;
struct Out { @builtin(position) pos:vec4<f32>, @location(0) uv:vec2<f32> }
@vertex fn vs(@builtin(vertex_index) id:u32)->Out{
 let v=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));var out:Out;out.pos=vec4<f32>(v[id],0.,1.);out.uv=vec2<f32>((v[id].x+1.)*.5,1.-(v[id].y+1.)*.5);return out;
}
@fragment fn fs(input:Out)->@location(0) vec4<f32>{
 let xy=vec2<f32>(64.+input.uv.x*384.,304.-input.uv.y*288.);var trans=1.;var scattered=0.;
 for(var z=0u;z<128u;z++){
 var q=vec3<f32>(xy,(f32(z)+.5)*4.);if(p.mode>=2u){q=vec3<f32>(512.-q.z,q.y,q.x);}
 let s=textureSampleLevel(field,filtering,q/512.,0.).x;let l=textureSampleLevel(light,filtering,q/512.,0.).x;
 let a=1.-exp(-s*4.);scattered+=trans*a*l;trans*=1.-a;
 }
 var rgb=vec3<f32>(scattered)*vec3<f32>(1.,.98,.94)+trans*vec3<f32>(.24,.40,.64);
 if(p.mode%2u==1u){rgb=vec3<f32>(trans);}
 // Canvas sRGB conversion is explicit because the swapchain is unorm.
 rgb=clamp(rgb,vec3<f32>(0.),vec3<f32>(1.));rgb=select(12.92*rgb,1.055*pow(rgb,vec3<f32>(1./2.4))-.055,rgb>vec3<f32>(.0031308));return vec4<f32>(rgb,1.);
}`;
async function compile(code){const module=device.createShaderModule({code});const info=await module.getCompilationInfo();const errors=info.messages.filter(x=>x.type==='error');if(errors.length)throw Error(errors.map(x=>`${x.lineNum}:${x.linePos} ${x.message}`).join('\n'));return module;}
async function save(name,body){const response=await fetch('/capture',{method:'POST',headers:{'X-Capture-Name':name},body});if(!response.ok)throw Error('Capture save failed '+response.status);return response.text();}
async function digest(bytes){return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))).map(x=>x.toString(16).padStart(2,'0')).join('');}
async function load() {
 instrumentHash=await digest(await(await fetch(import.meta.url,{cache:'no-store'})).arrayBuffer());computeHash=await digest(new TextEncoder().encode(shader));
 if (!navigator.gpu) throw Error('WebGPU unavailable; offline PNG review remains available.');
 $('status').textContent='Requesting GPU adapter…';
 const adapter=await Promise.race([navigator.gpu.requestAdapter({powerPreference:'high-performance'}),new Promise((_,reject)=>setTimeout(()=>reject(Error('GPU adapter request timed out after10s')),10000))]);
 if(!adapter)throw Error('No WebGPU adapter.');
 const useStamp=adapter.features.has('timestamp-query');
 $('status').textContent='Requesting GPU device…';
 device=await adapter.requestDevice({requiredFeatures:useStamp?['timestamp-query']:[]});
 device.addEventListener('uncapturederror',event=>{$('status').textContent+='\nGPU error: '+event.error.message;playing=false;});
 $('status').textContent='Loading pinned source manifest…';
 m=await (await fetch(new URL('cloudclip.json',base))).json();
 if(m.density_side!==128||![0,32].includes(m.flow_side)||!(m.runtime_sigma_max_per_m>0))throw Error('Unsupported source dimensions/scale.');
 densityBytes=new Uint8Array(await(await fetch(new URL('density.bin',base))).arrayBuffer());flowBytes=m.reader_kind==='direct'?new Uint8Array(0):new Uint8Array(await(await fetch(new URL('flow.bin',base))).arrayBuffer());
 const hashes=await Promise.all([densityBytes,flowBytes].map(async b=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',b))).map(x=>x.toString(16).padStart(2,'0')).join('')));
 for(let i=0;i<(m.reader_kind==='direct'?1:2);i++)if(hashes[i]!==m.products[['density.bin','flow.bin'][i]].sha256)throw Error('Source product hash mismatch.');
 paddedFlow=new Uint8Array(flowBytes.length/3*4);for(let i=0,j=0;i<flowBytes.length;i+=3,j+=4)paddedFlow.set(flowBytes.subarray(i,i+3),j);
 const buffer=(bytes,usage)=>{const b=device.createBuffer({size:Math.max(4,Math.ceil(bytes.byteLength/4)*4),usage:usage|GPUBufferUsage.COPY_DST});device.queue.writeBuffer(b,0,bytes);return b;};
 const db=buffer(densityBytes,GPUBufferUsage.STORAGE),fb=buffer(paddedFlow,GPUBufferUsage.STORAGE);
 uniform=device.createBuffer({size:64,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});
 pack=device.createBuffer({size:128**3*2,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
 const texture=()=>device.createTexture({size:[128,128,128],dimension:'3d',format:'r16float',usage:GPUTextureUsage.TEXTURE_BINDING|GPUTextureUsage.COPY_DST|GPUTextureUsage.COPY_SRC});
 densityTexture=texture();lightTexture=texture();
 pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compile(shader),entryPoint:'main'}});
 computeGroup=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[db,fb,uniform,pack].map((buffer,binding)=>({binding,resource:{buffer}}))});
 lightPipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compile(lightShader),entryPoint:'main'}});
 lightGroup=device.createBindGroup({layout:lightPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:densityTexture.createView()},{binding:1,resource:{buffer:pack}}]});
 context=canvas.getContext('webgpu');const format=navigator.gpu.getPreferredCanvasFormat();context.configure({device,format,alphaMode:'opaque'});
 renderPipeline=await device.createRenderPipelineAsync({layout:'auto',vertex:{module:await compile(renderShader),entryPoint:'vs'},fragment:{module:await compile(renderShader),entryPoint:'fs',targets:[{format}]},primitive:{topology:'triangle-list'}});
 renderGroup=device.createBindGroup({layout:renderPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:densityTexture.createView()},{binding:1,resource:lightTexture.createView()},{binding:2,resource:device.createSampler({minFilter:'linear',magFilter:'linear'})},{binding:3,resource:{buffer:uniform}}]});
 if(useStamp){stamp=device.createQuerySet({type:'timestamp',count:2});queryResolve=device.createBuffer({size:16,usage:GPUBufferUsage.QUERY_RESOLVE|GPUBufferUsage.COPY_SRC});queryRead=device.createBuffer({size:16,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});}
 sampleIdentity=hashes[0];
 $('identity').textContent=`128³ ·512m ·${m.source_times_s.at(-1)} source seconds ·${m.source_times_s.length} density frames ·${m.reader_kind==='direct'?'direct, spatial only':'32³ carrier'}\nInstrument SHA256 ${instrumentHash}\nDensity SHA256 ${hashes[0]}\nFlow ${m.reader_kind==='direct'?'not used':hashes[1]}\nSource binaries ${((densityBytes.length+flowBytes.length)/1048576).toFixed(3)}MiB · GPU padded source ${((densityBytes.length+paddedFlow.length)/1048576).toFixed(3)}MiB\nAdapter ${JSON.stringify({vendor:adapter.info?.vendor,architecture:adapter.info?.architecture,device:adapter.info?.device,description:adapter.info?.description})} · timestamp query ${useStamp}`;
 $('status').textContent=`Candidate status: ${m.qualification}. ${`${m.rejection_reasons.length} failed source gates; details in cloudclip.json`}`;
 $('seek').max=m.source_times_s.at(-1);draw(age,true);requestAnimationFrame(tick);
}
function parameters(s){let i=0;while(i<m.source_times_s.length-2&&m.source_times_s[i+1]<=s)i++;const duration=m.source_times_s[i+1]-m.source_times_s[i],theta=(s-m.source_times_s[i])/duration,a=m.density_frames[i],b=m.density_frames[i+1];const data=new ArrayBuffer(64),u=new Uint32Array(data),f=new Float32Array(data);u[0]=a.offset;u[1]=a.offset+a.index_bytes+a.occupancy_bytes;u[2]=b.offset;u[3]=b.offset+b.index_bytes+b.occupancy_bytes;u[4]=m.reader_kind==='direct'?0:i*32**3;u[5]=128;f[6]=theta;f[7]=duration;if(m.reader_kind!=='direct')f.set(m.flow_intervals[i].component_scale_m_per_s,8);f[11]=m.runtime_sigma_max_per_m;u[12]=Number($('view').value);u[13]=m.reader_kind==='direct'?1:0;u[14]=m.density_encoding_version==='thrum.cloud-source-proof.mixed-bricks.v1'?1:0;return data;}
function sourceCommands(encoder,s,withStamp=false){device.queue.writeBuffer(uniform,0,parameters(s));const pass=encoder.beginComputePass(withStamp&&stamp?{timestampWrites:{querySet:stamp,beginningOfPassWriteIndex:0,endOfPassWriteIndex:1}}:{});pass.setPipeline(pipeline);pass.setBindGroup(0,computeGroup);pass.dispatchWorkgroups(16384);pass.end();encoder.copyBufferToTexture({buffer:pack,bytesPerRow:256,rowsPerImage:128},{texture:densityTexture},[128,128,128]);}
function draw(s,source=true){const encoder=device.createCommandEncoder();if(source){sourceCommands(encoder,s);const lightPass=encoder.beginComputePass();lightPass.setPipeline(lightPipeline);lightPass.setBindGroup(0,lightGroup);lightPass.dispatchWorkgroups(128);lightPass.end();encoder.copyBufferToTexture({buffer:pack,bytesPerRow:256,rowsPerImage:128},{texture:lightTexture},[128,128,128]);}else device.queue.writeBuffer(uniform,0,parameters(s));const pass=encoder.beginRenderPass({colorAttachments:[{view:context.getCurrentTexture().createView(),loadOp:'clear',storeOp:'store',clearValue:{r:0,g:0,b:0,a:1}}]});pass.setPipeline(renderPipeline);pass.setBindGroup(0,renderGroup);pass.draw(3);pass.end();device.queue.submit([encoder.finish()]);}
async function tick(now){if(!measuring&&playing){age=Math.min(m.source_times_s.at(-1),age+Math.max(0,now-prior)/1000*Number($('rate').value));draw(age);$('age').textContent=age.toFixed(2)+' s';$('seek').value=age;if(recording)recordSamples.push({wall_ms:now-recordStart.wall_ms,source_s:age});if(age===m.source_times_s.at(-1)){if(recording)stopRecording();playing=false;$('play').textContent='Play';}cadences.push(now-lastRenderTime);if(cadences.length>600)cadences.shift();lastRenderTime=now;await device.queue.onSubmittedWorkDone();}prior=now;requestAnimationFrame(tick);}
$('play').onclick=()=>{if(age>=240)age=0;playing=!playing;prior=performance.now();lastRenderTime=prior;$('play').textContent=playing?'Pause':'Play';};
$('seek').oninput=()=>{age=Number($('seek').value);$('age').textContent=age.toFixed(2)+' s';if(device)draw(age);};
$('exact').oninput=()=>{const value=Number($('exact').value);if(!Number.isFinite(value)||value<0||value>240)return;age=value;$('seek').value=age;$('age').textContent=age.toFixed(5)+' s';if(device)draw(age);};
$('view').onchange=()=>{if(device)draw(age,false);};
$('capture').onclick=()=>{draw(age,false);requestAnimationFrame(()=>{const a=document.createElement('a');a.download=`source-${age.toFixed(2)}-${$('view').value}.png`;a.href=canvas.toDataURL('image/png');a.click();});};
function stats(values){const v=[...values].sort((a,b)=>a-b),q=p=>v[Math.min(v.length-1,Math.floor(v.length*p))];return {n:v.length,p50_ms:q(.5),p95_ms:q(.95),p99_ms:q(.99),max_ms:v.at(-1)};}
$('bench').onclick=async()=>{if(measuring||recording)return;measuring=true;playing=false;$('play').textContent='Play';$('bench').disabled=true;const gpu=[],wall=[];try{for(let k=0;k<110;k++){const encoder=device.createCommandEncoder(),s=30+((k*1.13)%90);sourceCommands(encoder,s,true);if(stamp){encoder.resolveQuerySet(stamp,0,2,queryResolve,0);encoder.copyBufferToBuffer(queryResolve,0,queryRead,0,16);}const begin=performance.now();device.queue.submit([encoder.finish()]);await device.queue.onSubmittedWorkDone();const dt=performance.now()-begin;if(stamp){await queryRead.mapAsync(GPUMapMode.READ);const values=new BigUint64Array(queryRead.getMappedRange());if(k>=10)gpu.push(Number(values[1]-values[0])/1e6);queryRead.unmap();}if(k>=10)wall.push(dt);if(k%10===0)$('metrics').textContent=`Measuring source kernel ${k}/110…`;}
 const report={kind:'isolated source-only full-grid reconstruction; GPU compute excludes R16 copy; queue completion includes copy and observer wait',instrument_sha256:instrumentHash,compute_shader_sha256:computeHash,source_density_sha256:sampleIdentity,source_frames:m.source_times_s.length,side:128,workgroups:16384,samples:100,warmup:10,gpu_compute:gpu.length?stats(gpu):'timestamp unavailable',queue_completion:stats(wall),animation_callback_cadence:cadences.length?stats(cadences):'unrun',source_binary_bytes:densityBytes.length+flowBytes.length,gpu_source_bytes:densityBytes.length+paddedFlow.length,limits:'M3 Max local source instrument, not M2, complete frame,720p workload, mobile or startup qualification'};$('metrics').textContent=JSON.stringify(report,null,2);await save(`kernel-${Date.now()}.json`,JSON.stringify(report,null,2));draw(age);}
 catch(error){$('metrics').textContent=error.stack;}finally{measuring=false;$('bench').disabled=false;}};
function stopRecording(){recording=false;recorder.stop();$('record').textContent='Record playback';}
$('record').onclick=async()=>{if(recording){stopRecording();return;}if(measuring)return;recordedChunks=[];recordSamples=[];recordStart={wall_ms:performance.now(),source_s:age,rate:Number($('rate').value),view:Number($('view').value),id:Date.now()};const stream=canvas.captureStream(24);recorder=new MediaRecorder(stream,{mimeType:'video/webm;codecs=vp9',videoBitsPerSecond:2500000});recorder.ondataavailable=e=>{if(e.data.size)recordedChunks.push(e.data);};recorder.onstop=async()=>{stream.getTracks().forEach(t=>t.stop());const prefix=`source-${recordStart.id}-${recordStart.rate}x`;await save(prefix+'.webm',new Blob(recordedChunks,{type:'video/webm'}));await save(prefix+'.json',JSON.stringify({instrument_sha256:instrumentHash,compute_shader_sha256:computeHash,source_density_sha256:sampleIdentity,start:recordStart,end_source_s:age,presented_samples:recordSamples,scope:'Canvas capture; source age follows elapsed wall time; not a performance benchmark'},null,2));$('metrics').textContent=`Saved ${prefix}.webm`;};recording=true;recorder.start(500);if(!playing){playing=true;prior=performance.now();lastRenderTime=prior;$('play').textContent='Pause';}$('record').textContent='Stop recording';};
$('verify').onclick=async()=>{if(measuring||recording)return;playing=false;$('play').textContent='Play';device.pushErrorScope('validation');draw(age);await device.queue.onSubmittedWorkDone();const bytes=128**3*2;const read=device.createBuffer({size:bytes,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});const encoder=device.createCommandEncoder();encoder.copyTextureToBuffer({texture:densityTexture},{buffer:read,bytesPerRow:256,rowsPerImage:128},[128,128,128]);device.queue.submit([encoder.finish()]);await read.mapAsync(GPUMapMode.READ);const validation=await device.popErrorScope();if(validation){read.unmap();read.destroy();$('metrics').textContent=validation.message;return;}const copy=new Uint8Array(read.getMappedRange()).slice();read.unmap();read.destroy();const prefix=`field-${age.toFixed(3)}-${Date.now()}`;await save(prefix+'.f16',copy);await save(prefix+'.json',JSON.stringify({age,instrument_sha256:instrumentHash,compute_shader_sha256:computeHash,source_density_sha256:sampleIdentity,source_flow_sha256:m.products['flow.bin']?.sha256??null,side:128,encoding:'IEEE754halfLE Xfastest,Y,Z',scope:'Actual GPU reconstructed field, before diagnostic lighting'},null,2));$('metrics').textContent=`Saved ${prefix}`;};
load().catch(error=>{$('status').textContent=error.stack;});
