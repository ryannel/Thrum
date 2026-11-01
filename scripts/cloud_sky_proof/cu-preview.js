import {createCuRecipe,cuFrame,sampleCu,CONVECTIVE_WGSL} from './convective.js';
import {ScalarField,FIELD_PARAMETERS_WGSL,FIELD_DISPATCH_WGSL,PACKED_LOAD_WGSL} from './scalar-field.js';
import {compileShader,hash,readBuffer} from './gpu.js';
import {BEER_WGSL} from './transport.js';
import {PreviewClock} from './clock.js';

const $=id=>document.getElementById(id),canvas=document.querySelector('canvas');
const norm=v=>{const length=Math.hypot(...v);return v.map(x=>x/length);};
const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
let device,context,field,recipe,nodeBuffer,sceneBuffer,errors,prefix,ambient,fieldPipeline,fieldGroup;
let prefixPipeline,ambientPipeline,imagePipeline,prefixInput,ambientInput,imageInput,prefixOutput,ambientOutput;
let age=650,busy=false,pending=false,exclusive=false;
const clock=new PreviewClock(1800,650);
let yaw=.48,pitch=.11,distanceScale=3.2,pan=[0,0,0],lastFrame,identity,shaderCommon,shaderCache;
let querySet,queryResolve,queryRead,sentinelPipeline,sentinelGroup;

async function save(name,body) {
  const response=await fetch('/capture',{method:'POST',headers:{'X-Capture-Name':name},body});
  if(!response.ok)throw Error('Capture '+response.status);
}
function resetCamera(){
  yaw=.48;distanceScale=3.2;pan=[0,0,0];
  if(recipe){const radius=Math.max(recipe.bounds.extent[0],recipe.bounds.extent[2])*distanceScale;
    pitch=Math.asin(Math.max(-.95,Math.min(.95,(120-(900+recipe.bounds.extent[1]*.5)*.38)/radius)));}
  else pitch=-.3;
}
function currentAge(){return clock.sample(performance.now());}
function pause(){age=clock.pause(performance.now());$('play').textContent='Play';}
function requestFrame(){pending=true;if(!busy&&!exclusive)void drain();}
async function drain(){
  if(busy||exclusive)return;
  busy=true;
  try {
    do {pending=false;await renderFrame(currentAge());} while(pending&&!clock.playing);
  } catch(error){pause();$('status').textContent=error.stack;}
  finally{busy=false;if(clock.playing)requestAnimationFrame(()=>requestFrame());}
}
async function idle(){while(busy)await new Promise(resolve=>setTimeout(resolve,5));}
function scene(time){
  const shift=[3*(time-650),900,time-650];
  const lo=recipe.bounds.min.map((v,i)=>v+shift[i]),extent=recipe.bounds.extent;
  const centre=lo.map((v,i)=>v+extent[i]*.5);
  const width=Math.max(extent[0],extent[2]),radius=width*distanceScale;
  const target=centre.map((v,i)=>(i===1?v*.38:v)+pan[i]);
  const camera=[target[0]+radius*Math.sin(yaw)*Math.cos(pitch),target[1]+radius*Math.sin(pitch),target[2]+radius*Math.cos(yaw)*Math.cos(pitch)];
  camera[1]=Math.max(2,camera[1]);
  const elevation=Number($('sun').value),angle=elevation*Math.PI/180;
  const sun=norm([-.9*Math.cos(angle),Math.sin(angle),.4358899*Math.cos(angle)]);
  const W=sun.map(x=>-x),U=norm(cross([0,1,0],sun)),V=cross(W,U);
  const projected=basis=>extent.reduce((sum,v,i)=>sum+Math.abs(basis[i])*v*.5,0)+Math.max(...extent)/64;
  const data=new ArrayBuffer(192),f=new Float32Array(data),u=new Uint32Array(data);
  const put=(i,v)=>f.set(v,i*4);
  put(0,[...lo,time]);put(1,[...extent,recipe.duration_s]);put(2,[...camera,canvas.width/canvas.height]);put(3,[...target,.47]);
  put(4,[...sun,elevation>0?1:0]);put(5,[...U,projected(U)]);put(6,[...V,projected(V)]);put(7,[...W,projected(W)]);
  put(8,[90000,85000,75000,.00006]);put(9,(elevation>0?[6000,9000,15000]:[0,0,0]).concat([1]));
  u.set([...field.levels[0].dimensions,0],40);u.set([$('main').checked?1:0,$('reflection').checked?1:0,$('transmittance').checked?1:0,0],44);
  return {source_time_s:time,variant:recipe.variant,seed:recipe.seed,shift_m:shift,lo_m:lo,extent_m:extent,camera_m:camera,target_m:target,sun_direction:sun,sun_elevation_deg:elevation,main_cloud:$('main').checked,mirror:$('reflection').checked,data};
}
async function renderFrame(time){
  age=time;$('exact').value=time.toFixed(2);$('seek').value=time;
  if(time>=recipe.duration_s)pause();
  const start=performance.now(),state=scene(time);
  device.queue.writeBuffer(nodeBuffer,0,cuFrame(recipe,time));
  field.setParameters(recipe.bounds.min,recipe.bounds.extent,time);
  device.queue.writeBuffer(sceneBuffer,0,state.data);device.queue.writeBuffer(errors,0,new Uint32Array(4));
  const encoder=device.createCommandEncoder();
  if(querySet){const marker=encoder.beginComputePass({timestampWrites:{querySet,beginningOfPassWriteIndex:0}});marker.setPipeline(sentinelPipeline);marker.setBindGroup(0,sentinelGroup);marker.dispatchWorkgroups(1);marker.end();}
  const sourcePass=encoder.beginComputePass();field.encode(sourcePass,fieldPipeline,fieldGroup);sourcePass.end();
  const caches=encoder.beginComputePass();
  caches.setPipeline(prefixPipeline);caches.setBindGroup(0,prefixInput);caches.setBindGroup(1,prefixOutput);caches.dispatchWorkgroups(9,9);
  caches.setPipeline(ambientPipeline);caches.setBindGroup(0,ambientInput);caches.setBindGroup(1,ambientOutput);caches.dispatchWorkgroups(2,2,2);caches.end();
  const draw=encoder.beginRenderPass({colorAttachments:[{view:context.getCurrentTexture().createView(),loadOp:'clear',storeOp:'store',clearValue:{r:0,g:0,b:0,a:1}}]});
  draw.setPipeline(imagePipeline);draw.setBindGroup(0,imageInput);draw.draw(3);draw.end();
  if(querySet){const marker=encoder.beginComputePass({timestampWrites:{querySet,endOfPassWriteIndex:1}});marker.setPipeline(sentinelPipeline);marker.setBindGroup(0,sentinelGroup);marker.dispatchWorkgroups(1);marker.end();encoder.resolveQuerySet(querySet,0,2,queryResolve,0);encoder.copyBufferToBuffer(queryResolve,0,queryRead,0,16);}
  const encoded=performance.now();device.queue.submit([encoder.finish()]);
  let gpu=null;
  if(querySet){await queryRead.mapAsync(GPUMapMode.READ);const stamps=new BigUint64Array(queryRead.getMappedRange());if(stamps[1]>stamps[0]&&stamps[0]>0n)gpu=Number(stamps[1]-stamps[0])/1e6;queryRead.unmap();}
  await device.queue.onSubmittedWorkDone();
  const counters=new Uint32Array((await readBuffer(device,errors,16)).buffer);
  const end=performance.now();
  lastFrame={...state,data:undefined,cpu_encode_ms:encoded-start,queue_complete_ms:end-encoded,gpu_bracket_ms:gpu,invalid_view_paths:counters[0],source_field_bytes:field.levels.reduce((n,l)=>n+l.bytes,0),scope:'fresh source+mips+64prefix+8cube ambient+native main/mirror/display; local vacuum/constant sky fixture, not full SKY-G or Thrum frame'};
  $('status').textContent=`Candidate ${recipe.name} · ${time.toFixed(2)} s · ${gpu===null?'GPU query unavailable':gpu.toFixed(2)+' ms GPU bracket'} · ${(end-encoded).toFixed(2)} ms queue · not accepted`;
}
async function chooseRecipe(){
  $('variant').disabled=true;pause();await idle();exclusive=true;
  try {
    await device.queue.onSubmittedWorkDone();field?.destroy();nodeBuffer?.destroy();
    recipe=createCuRecipe(Number($('variant').value),1729);field=await ScalarField.create(device,[128,128,128]);
    const frame=cuFrame(recipe,age);nodeBuffer=device.createBuffer({size:frame.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
    fieldGroup=device.createBindGroup({layout:fieldPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:field.levels[0].uniform}},{binding:1,resource:field.levels[0].texture.createView()},{binding:2,resource:{buffer:nodeBuffer}}]});
    const commonEntries=()=>[{binding:0,resource:field.levels[0].texture.createView()},{binding:1,resource:device.createSampler({minFilter:'linear',magFilter:'linear'})},{binding:2,resource:prefix.createView()},{binding:3,resource:ambient.createView()},{binding:4,resource:{buffer:sceneBuffer}},{binding:5,resource:{buffer:errors}}];
    const cacheEntries=()=>commonEntries().filter(row=>[0,1,4].includes(row.binding));
    prefixInput=device.createBindGroup({layout:prefixPipeline.getBindGroupLayout(0),entries:cacheEntries()});
    ambientInput=device.createBindGroup({layout:ambientPipeline.getBindGroupLayout(0),entries:cacheEntries()});
    imageInput=device.createBindGroup({layout:imagePipeline.getBindGroupLayout(0),entries:commonEntries()});
    $('identity').textContent=JSON.stringify({identity,recipe,known_limits:['128cube candidate field','64cube point-tau interpolation, not qualified deep curves','8cube/five-axis external ambient','constant diffuse receiver term, not full clouded irradiance','vacuum and external sky, not SKY-G air/horizon','1280x720 native, no history, one GPU frame in flight']},null,2);
    resetCamera();
  } finally{exclusive=false;$('variant').disabled=false;}
  requestFrame();
}
async function checkSource(){
  pause();await idle();exclusive=true;$('checks').textContent='Checking canonical source against CPU…';
  try {
    await renderFrame(age);
    const points=[];let random=1093;
    for(let i=0;i<192;i++)points.push(recipe.bounds.min.map((v,k)=>{random=(Math.imul(random,1664525)+1013904223)>>>0;return v+random/4294967296*recipe.bounds.extent[k];}));
    for(const node of recipe.cohorts)points.push(node.end_m,node.start_m);
    const code=CONVECTIVE_WGSL+PACKED_LOAD_WGSL+`
@group(0) @binding(6) var<storage,read> inputs:array<vec4f>;
@group(0) @binding(7) var<storage,read_write> outputs:array<vec4f>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3u) {
 if(id.x<arrayLength(&inputs)){let q=inputs[id.x];outputs[id.x]=vec4f(canonicalSigma(q.xyz,q.w),0.,0.,0.);}
}`;
    const input=device.createBuffer({size:points.length*16,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST}),output=device.createBuffer({size:points.length*16,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
    device.queue.writeBuffer(input,0,new Float32Array(points.flatMap(p=>[...p,age])));
    const pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,code),entryPoint:'main'}});
    const group=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:2,resource:{buffer:nodeBuffer}},{binding:6,resource:{buffer:input}},{binding:7,resource:{buffer:output}}]});
    const encoder=device.createCommandEncoder(),pass=encoder.beginComputePass();pass.setPipeline(pipeline);pass.setBindGroup(0,group);pass.dispatchWorkgroups(Math.ceil(points.length/64));pass.end();device.queue.submit([encoder.finish()]);
    const values=new Float32Array((await readBuffer(device,output,points.length*16)).buffer);input.destroy();output.destroy();
    const rows=points.map((point,index)=>{const expected=sampleCu(recipe,point,age),actual=values[index*4];return {point,expected,actual,error:Math.abs(expected-actual)};});
    const max=Math.max(...rows.map(x=>x.error));const report={identity,scene:lastFrame,source_recipe:recipe,source_shader_sha256:await hash(new TextEncoder().encode(code)),scope:'canonical CPU double vs actual WGSL coefficient; no field/ray/appearance qualification',max_absolute_sigma_error_per_m:max,pass:max<2e-6,rows};
    await save(`cu-source-${Date.now()}.json`,JSON.stringify(report,null,2));$('checks').textContent=JSON.stringify({pass:report.pass,samples:rows.length,max_absolute_sigma_error_per_m:max,scope:report.scope},null,2);
  } catch(error){$('checks').textContent=error.stack;}finally{exclusive=false;}
}
async function load(){
  identity=await(await fetch('/identity')).json();if(identity.build_id!==document.querySelector('meta[name="proof-build"]').content)throw Error('Build changed; reload');
  const adapter=await navigator.gpu?.requestAdapter({powerPreference:'high-performance'});if(!adapter)throw Error('WebGPU unavailable');
  device=await adapter.requestDevice({requiredFeatures:adapter.features.has('timestamp-query')?['timestamp-query']:[]});
  identity.adapter={vendor:adapter.info.vendor,architecture:adapter.info.architecture,device:adapter.info.device};
  device.addEventListener('uncapturederror',event=>{pause();$('status').textContent='GPU validation: '+event.error.message;});
  device.lost.then(info=>{pause();$('status').textContent='Device lost: '+info.message;});
  context=canvas.getContext('webgpu');const format=navigator.gpu.getPreferredCanvasFormat();context.configure({device,format,alphaMode:'opaque'});
  fieldPipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,FIELD_PARAMETERS_WGSL+CONVECTIVE_WGSL+FIELD_DISPATCH_WGSL),entryPoint:'fillField'}});
  shaderCommon=PACKED_LOAD_WGSL+BEER_WGSL+await(await fetch('cu-preview.wgsl')).text();shaderCache=await(await fetch('cu-caches.wgsl')).text();
  const code=await compileShader(device,shaderCommon),cache=await compileShader(device,shaderCommon+shaderCache);
  prefixPipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:cache,entryPoint:'buildPrefix'}});
  ambientPipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:cache,entryPoint:'buildAmbient'}});
  imagePipeline=await device.createRenderPipelineAsync({layout:'auto',vertex:{module:code,entryPoint:'triangle'},fragment:{module:code,entryPoint:'image',targets:[{format}]},primitive:{topology:'triangle-list'}});
  sceneBuffer=device.createBuffer({size:192,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});errors=device.createBuffer({size:16,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST|GPUBufferUsage.COPY_SRC});
  const usage=GPUTextureUsage.TEXTURE_BINDING|GPUTextureUsage.STORAGE_BINDING|GPUTextureUsage.COPY_SRC;
  prefix=device.createTexture({size:[66,66,66],dimension:'3d',format:'rgba16float',usage});ambient=device.createTexture({size:[8,8,8],dimension:'3d',format:'rgba16float',usage});
  prefixOutput=device.createBindGroup({layout:prefixPipeline.getBindGroupLayout(1),entries:[{binding:0,resource:prefix.createView()}]});
  ambientOutput=device.createBindGroup({layout:ambientPipeline.getBindGroupLayout(1),entries:[{binding:1,resource:ambient.createView()}]});
  if(device.features.has('timestamp-query')) {
    querySet=device.createQuerySet({type:'timestamp',count:2});queryResolve=device.createBuffer({size:256,usage:GPUBufferUsage.QUERY_RESOLVE|GPUBufferUsage.COPY_SRC});queryRead=device.createBuffer({size:16,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
    const marker=device.createBuffer({size:4,usage:GPUBufferUsage.STORAGE});
    sentinelPipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,'@group(0) @binding(0) var<storage,read_write> marker:atomic<u32>; @compute @workgroup_size(1) fn main(){atomicAdd(&marker,1u);}'),entryPoint:'main'}});
    sentinelGroup=device.createBindGroup({layout:sentinelPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:marker}}]});
  }
  await chooseRecipe();
}
$('play').onclick=()=>{if(clock.playing){pause();}else{clock.play(performance.now());$('play').textContent='Pause';requestFrame();}};
$('rate').onchange=()=>{age=clock.setRate(Number($('rate').value),performance.now());requestFrame();};
for(const id of ['exact','seek'])$(id).oninput=()=>{pause();age=clock.seek(Number($(id).value),performance.now());requestFrame();};
for(const id of ['sun','main','reflection','transmittance'])$(id).onchange=requestFrame;
$('variant').onchange=()=>void chooseRecipe().catch(error=>{$('status').textContent=error.stack;});
$('reset').onclick=()=>{resetCamera();requestFrame();};$('probes').onclick=()=>void checkSource();
$('capture').onclick=async()=>{pause();await idle();exclusive=true;try{await renderFrame(age);const name=`cu-${Date.now()}`;const blob=await new Promise(resolve=>canvas.toBlob(resolve,'image/png'));await save(name+'.png',blob);await save(name+'.json',JSON.stringify({identity,recipe,frame:lastFrame},null,2));$('checks').textContent='Saved '+name;}catch(error){$('checks').textContent=error.stack;}finally{exclusive=false;}};
canvas.addEventListener('wheel',event=>{event.preventDefault();const scale=event.deltaMode===1?16:1,dx=Math.max(-300,Math.min(300,event.deltaX*scale)),dy=Math.max(-300,Math.min(300,event.deltaY*scale));if(event.ctrlKey){distanceScale=Math.max(.35,Math.min(12,distanceScale*Math.exp(dy*.005)));}else if(event.shiftKey){pan[0]+=dx*2;pan[1]-=dy*2;}else{yaw+=dx*.004;pitch=Math.max(-1.3,Math.min(1.3,pitch+dy*.004));}requestFrame();},{passive:false});
let drag=null;canvas.addEventListener('contextmenu',event=>event.preventDefault());canvas.addEventListener('pointerdown',event=>{drag={x:event.clientX,y:event.clientY,pan:event.button===2||event.shiftKey};canvas.setPointerCapture(event.pointerId);});canvas.addEventListener('pointerup',()=>drag=null);canvas.addEventListener('pointercancel',()=>drag=null);canvas.addEventListener('pointermove',event=>{if(!drag)return;const dx=event.clientX-drag.x,dy=event.clientY-drag.y;drag.x=event.clientX;drag.y=event.clientY;if(drag.pan){pan[0]-=dx*2;pan[1]+=dy*2;}else{yaw-=dx*.006;pitch=Math.max(-1.3,Math.min(1.3,pitch+dy*.006));}requestFrame();});
load().catch(error=>{$('status').textContent=error.stack;});
