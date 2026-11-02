import {DeepSun} from './deep-sun.js';
import {checkScatterFit} from './fit-probes.js';
import {ScatterFit} from './scatter-fit.js';
import {PreparedCu} from './prepared-cu.js';
import {CuSourceField} from './cu-source.js';
import {FIELD_PARAMETERS_WGSL,PACKED_LOAD_WGSL} from './scalar-field.js';
import {compileShader,hash,readBuffer} from './gpu.js';
import {BEER_WGSL} from './transport.js';
import {PreviewClock} from './clock.js';

const $=id=>document.getElementById(id),canvas=document.querySelector('canvas');
const proofParameters=new URLSearchParams(location.search);
const preparedOptions={crop:proofParameters.get('source_crop')!=='0',specialize128:proofParameters.get('source_fast')!=='0'};
const norm=v=>{const length=Math.hypot(...v);return v.map(x=>x/length);};
const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
let device,context,sourceObject,field,recipe,sceneBuffer,errors,prefix,ambient,fit,fitProducts,imageLayout,deep,deepInput;
let prefixPipeline,ambientPipeline,imagePipeline,prefixInput,ambientInput,imageInput,prefixOutput,ambientOutput;
let age=650,busy=false,pending=false,exclusive=false;
const clock=new PreviewClock(1800,650);
let yaw=.48,pitch=.11,distanceScale=3.2,pan=[0,0,0],lastFrame,identity,shaderCommon,shaderCache;
let lastUniformWords;
let querySet,queryResolve,queryRead,sentinelPipeline,sentinelGroup,endSentinelPipeline,markerBuffer,lightCacheKey=null;

async function save(name,body) {
  const response=await fetch('/capture',{method:'POST',headers:{'X-Capture-Name':name},body});
  if(!response.ok)throw Error('Capture '+response.status);
}
function resetCamera(){
  const route=$('route').value;
  if(route==='local'){yaw=.48;pitch=Math.asin(100/(Math.max(...recipe.bounds.extent)*1.5));distanceScale=1.5;pan=[0,0,0];return;}
  if(route!=='coupled'){yaw=route==='side'?Math.PI*.5:0;pitch=route==='top'?1.15:0;distanceScale=1.1;pan=[0,0,0];return;}
  yaw=.48;distanceScale=3.2;pan=[0,0,0];
  if(recipe){const radius=Math.max(recipe.bounds.extent[0],recipe.bounds.extent[2])*distanceScale;
    pitch=Math.asin(Math.max(-.95,Math.min(.95,120/radius)));}
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
  const originTime=recipe.reference_time_s??650;
  const shift=[3*(time-originTime),$('route').value==='local'?0:900,time-originTime];
  const fieldBounds=sourceObject.fieldBounds??recipe.bounds;
  const lo=fieldBounds.min.map((v,i)=>v+shift[i]),extent=fieldBounds.extent;
  const masterLo=recipe.bounds.min.map((v,i)=>v+shift[i]),masterExtent=recipe.bounds.extent;
  const centre=masterLo.map((v,i)=>v+masterExtent[i]*.5);
  const width=Math.max(masterExtent[0],masterExtent[2]),radius=$('route').value==='coupled'?Math.max(3500,width*distanceScale):width*distanceScale;
  const target=centre.map((v,i)=>($('route').value==='coupled'?(i===1?0:v):$('route').value==='local'?(i===1?80:v):(recipe.inspection_centre_m?recipe.inspection_centre_m[i]+shift[i]:v))+pan[i]);
  let camera=[target[0]+radius*Math.sin(yaw)*Math.cos(pitch),target[1]+radius*Math.sin(pitch),target[2]+radius*Math.cos(yaw)*Math.cos(pitch)];
  camera[1]=Math.max(2,camera[1]);
  const lightMode=$('sun').value,elevation=lightMode==='sky'?30:Number(lightMode),angle=elevation*Math.PI/180;
  const sun=norm([-.9*Math.cos(angle),Math.sin(angle),.4358899*Math.cos(angle)]);
  if(['backlit','frontlit'].includes($('route').value)){const sign=$('route').value==='backlit'?-1:1;camera=target.map((v,i)=>v+sign*sun[i]*radius);camera[1]=Math.max(2,camera[1]);}
  const W=sun.map(x=>-x),U=norm(cross([0,1,0],sun)),V=cross(W,U);
  const projected=basis=>extent.reduce((sum,v,i)=>sum+Math.abs(basis[i])*v*.5,0)+Math.max(...extent)/64;
  const data=new ArrayBuffer(208),f=new Float32Array(data),u=new Uint32Array(data);
  const put=(i,v)=>f.set(v,i*4);
  put(0,[...lo,time]);put(1,[...extent,recipe.duration_s]);put(2,[...camera,canvas.width/canvas.height]);put(3,[...target,.47]);
  put(4,[...sun,lightMode!=='sky'&&elevation>0?1:0]);put(5,[...U,projected(U)]);put(6,[...V,projected(V)]);put(7,[...W,projected(W)]);
  put(8,[90000,85000,75000,.00006]);put(9,(lightMode!=='0'?[6000,9000,15000]:[0,0,0]).concat([sourceObject.sigmaMax]));
  const referencePoint=masterLo.map((v,i)=>v+masterExtent[i]*[.3467439115,.2991746366,.4495654702][i]);put(12,[...referencePoint,1]);
  u.set([...field.levels[0].dimensions,(Number($('light-model').value)|($('depth-model').value==='deep'?16:0)|($('reference-light').checked?32:0))],40);u.set([$('main').checked?1:0,$('reflection').checked?($('route').value==='local'?2:1):0,$('transmittance').checked?1:0,Number($('view').value)],44);
  return {source_time_s:time,source_model:sourceObject.kind,max_level:sourceObject.maxLevel,inspection_view:$('view').value,camera_route:$('route').value,variant:recipe.variant,seed:recipe.seed,shift_m:shift,lo_m:lo,extent_m:extent,master_lo_m:masterLo,master_extent_m:masterExtent,reference_point_m:referencePoint,allocation_profile:sourceObject.allocationProfile,camera_m:camera,target_m:target,sun_direction:sun,sun_elevation_deg:elevation,light_model:$('light-model').value,depth_model:$('depth-model').value,sun_reference_patches:$('reference-light').checked,deep_identity:deep.identity,fit_identity:fit.identity,main_cloud:$('main').checked,mirror:$('reflection').checked,data};
}
// Dense per-frame query slots never read an inactive/stale component timestamp.
// Whole bracket retains its actual-render consumer; component intervals are
// additive only when valid, ordered, nonoverlapping and inside that bracket.
function decodeGpuComponentTiming(stamps,records,wholeIndices){
  const available=stamps!==null,components={},intervals=[];
  const wholeStart=available?stamps[wholeIndices[0]]:null,wholeEnd=available?stamps[wholeIndices[1]]:null;
  const wholeValid=typeof wholeStart==='bigint'&&typeof wholeEnd==='bigint'&&wholeStart>0n&&wholeEnd>wholeStart;
  let complete=available;
  for(const [name,record] of Object.entries(records)){
    const active=record.status==='updated'||record.status==='rendered',pair=active?record.query_indices:null;
    const start=available&&pair?stamps[pair[0]]:null,end=available&&pair?stamps[pair[1]]:null;
    const valid=typeof start==='bigint'&&typeof end==='bigint'&&start>0n&&end>=start;
    components[name]={...record,query_indices:pair,duration_ms:valid?Number(end-start)/1e6:null,
      raw_timestamps_ns:typeof start==='bigint'&&typeof end==='bigint'?[String(start),String(end)]:null,
      timestamp_valid:active&&available?valid:null};
    if(active){if(valid)intervals.push({start,end,index:pair[0]});else complete=false;}
  }
  intervals.sort((a,b)=>a.index-b.index);
  const nonoverlap=complete?intervals.every((x,i)=>i===0||x.start>=intervals[i-1].end):null;
  const contained=complete&&wholeValid?intervals.every(x=>x.start>=wholeStart&&x.end<=wholeEnd):null;
  const admitted=complete&&wholeValid&&nonoverlap&&contained;
  const sum=admitted?intervals.reduce((n,x)=>n+x.end-x.start,0n):null;
  return {available,components,whole_bracket_query_indices:wholeIndices,
    raw_query_timestamps_ns:available?Array.from(stamps,String):null,
    timestamp_intervals_complete:available?complete:null,non_overlapping:nonoverlap,contained_in_whole:contained,
    additive_sum_ms:admitted?Number(sum)/1e6:null,
    unattributed_gpu_gap_ms:admitted?Number(wholeEnd-wholeStart-sum)/1e6:null,
    scope:'Pass begin/end intervals; timed cache dispatches are split into separate passes, adding instrumentation/pass-boundary overhead. Whole bracket still ends after compute reads the actual rendered canvas. Gap includes transitions/markers/idle/quantization; no CPU or presentation attribution.',
    observer_storage:{query_set_count:available?12:0,query_readback_bytes:available?96:0,query_resolve_bytes:available?256:0,used_query_count:available?stamps.length:0}};
}
async function renderFrame(time){
  age=time;$('exact').value=time.toFixed(2);$('seek').value=time;
  if(time>=recipe.duration_s)pause();
  const start=performance.now(),sourceFrame=sourceObject.prepare(time),state=scene(sourceFrame.sourceParams[0]);
  lastUniformWords=new Uint32Array(state.data).slice(0,48);
  device.queue.writeBuffer(sceneBuffer,0,state.data);device.queue.writeBuffer(errors,0,new Uint32Array(4));
  const sourceUpdated=sourceObject.needsBuild;
  const wantedLightKey=JSON.stringify([sourceObject.kind,recipe.variant,recipe.seed,sourceObject.maxLevel,field.levels[0].dimensions,state.source_time_s,state.lo_m,$('sun').value,$('light-model').value,$('depth-model').value,fit.identity,deep.identity]);
  const lightUpdated=$('view').value!=='1'&&lightCacheKey!==wantedLightKey;
  const encoder=device.createCommandEncoder();
  const sunEnabled=new Float32Array(state.data)[19]>0,geometryView=state.inspection_view==='1';
  const fitUpdated=lightUpdated&&state.light_model==='2'&&sunEnabled,sunUpdated=lightUpdated&&sunEnabled;
  const componentRecords={
    source_field_mips:{status:sourceUpdated?'updated':'reused',query_indices:null},
    fit_products:{status:geometryView?'disabled-geometry-view':state.light_model!=='2'?'disabled-model':!sunEnabled?'disabled-sun':fitUpdated?'updated':'reused',query_indices:null},
    sun_depth:{status:geometryView?'disabled-geometry-view':!sunEnabled?'disabled-sun':sunUpdated?'updated':'reused',representation:state.depth_model,query_indices:null},
    ambient:{status:geometryView?'disabled-geometry-view':lightUpdated?'updated':'reused',query_indices:null},
    final_views:{status:'rendered',main_cloud_enabled:state.main_cloud,mirror_enabled:state.mirror,inspection_view:state.inspection_view,query_indices:null},
  };
  let nextQuery=0,wholeEndIndex=null;
  const componentStamp=name=>{
    if(!querySet)return {};
    const pair=[nextQuery++,nextQuery++];componentRecords[name].query_indices=pair;
    return {timestampWrites:{querySet,beginningOfPassWriteIndex:pair[0],endOfPassWriteIndex:pair[1]}};
  };
  if(querySet&&!sourceUpdated&&!lightUpdated){
    const marker=encoder.beginComputePass({timestampWrites:{querySet,beginningOfPassWriteIndex:nextQuery++}});
    marker.setPipeline(sentinelPipeline);marker.setBindGroup(0,sentinelGroup);marker.dispatchWorkgroups(1);marker.end();
  }
  if(sourceUpdated){const sourcePass=encoder.beginComputePass(componentStamp('source_field_mips'));sourceObject.encode(sourcePass);sourcePass.end();}
  const encodeSun=pass=>{
    if(state.depth_model==='deep')deep.encode(pass,deepInput);
    else{pass.setPipeline(prefixPipeline);pass.setBindGroup(0,prefixInput);pass.setBindGroup(1,prefixOutput);pass.dispatchWorkgroups(9,9);}
  };
  const encodeAmbient=pass=>{pass.setPipeline(ambientPipeline);pass.setBindGroup(0,ambientInput);pass.setBindGroup(1,ambientOutput);pass.dispatchWorkgroups(2,2,2);};
  if(lightUpdated){
    if(querySet){
      // Core WebGPU timestamps are pass boundaries. Preserve dispatch order and
      // kernels, while disclosing that these timed passes perturb scheduling.
      if(fitUpdated){const pass=encoder.beginComputePass(componentStamp('fit_products'));fitProducts.encode(pass);pass.end();}
      if(sunUpdated){const pass=encoder.beginComputePass(componentStamp('sun_depth'));encodeSun(pass);pass.end();}
      const pass=encoder.beginComputePass(componentStamp('ambient'));encodeAmbient(pass);pass.end();
    }else{
      // Preserve the original combined cache pass when timestamps unavailable.
      const caches=encoder.beginComputePass();if(fitUpdated)fitProducts.encode(caches);if(sunUpdated)encodeSun(caches);encodeAmbient(caches);caches.end();
    }
  }
  const renderedTexture=context.getCurrentTexture();
  const draw=encoder.beginRenderPass({...componentStamp('final_views'),colorAttachments:[{view:renderedTexture.createView(),loadOp:'clear',storeOp:'store',clearValue:{r:0,g:0,b:0,a:1}}]});
  draw.setPipeline(imagePipeline);draw.setBindGroup(0,imageInput);draw.draw(3);draw.end();
  if(querySet){
    wholeEndIndex=nextQuery++;
    const marker=encoder.beginComputePass({timestampWrites:{querySet,endOfPassWriteIndex:wholeEndIndex}});marker.setPipeline(endSentinelPipeline);marker.setBindGroup(0,device.createBindGroup({layout:endSentinelPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:markerBuffer}},{binding:1,resource:renderedTexture.createView()}]}));marker.dispatchWorkgroups(1);marker.end();
    encoder.resolveQuerySet(querySet,0,nextQuery,queryResolve,0);encoder.copyBufferToBuffer(queryResolve,0,queryRead,0,nextQuery*8);
  }
  const encoded=performance.now();device.queue.submit([encoder.finish()]);sourceObject.markSubmitted();if(lightUpdated)lightCacheKey=wantedLightKey;
  let gpu=null,rawStamps=null,componentTiming;
  if(querySet){
    let mapped=false;
    try{
      await queryRead.mapAsync(GPUMapMode.READ);mapped=true;
      const stamps=new BigUint64Array(queryRead.getMappedRange()).slice(0,nextQuery);
      rawStamps=[String(stamps[0]),String(stamps[wholeEndIndex])];
      if(stamps[wholeEndIndex]>stamps[0]&&stamps[0]>0n)gpu=Number(stamps[wholeEndIndex]-stamps[0])/1e6;
      componentTiming=decodeGpuComponentTiming(stamps,componentRecords,[0,wholeEndIndex]);
    }finally{if(mapped)queryRead.unmap();}
  }else{componentTiming=decodeGpuComponentTiming(null,componentRecords,[null,null]);}
  await device.queue.onSubmittedWorkDone();
  const counters=new Uint32Array((await readBuffer(device,errors,16)).buffer);
  const end=performance.now();
  lastFrame={...state,data:undefined,requested_source_time_s:time,source_updated:sourceUpdated,light_updated:lightUpdated,source_stats:sourceFrame.stats,cpu_encode_ms:encoded-start,queue_complete_ms:end-encoded,gpu_bracket_ms:gpu,gpu_raw_timestamps_ns:rawStamps,gpu_component_timing:componentTiming,gpu_timing_scope:'first actual compute pass (or reuse marker) through compute consuming actual rendered canvas; no present latency',invalid_view_paths:counters[0],source_field_bytes:field.levels.reduce((n,l)=>n+l.bytes,0),sun_depth_fallbacks:counters[1],sun_depth_invalid_footprints:counters[2],sun_depth_unsupported:counters[3],deep_bytes:deep.bytes,scatter_fit_bytes:{...fit.bytes,products:fitProducts.bytes},scope:'requested graph with explicit source/light reuse flags; geometry view omits light products; local fixture, not full SKY-G or Thrum frame'};
  if(counters[0]||counters[3]){clock.seek(time,performance.now());pause();throw Error(`Stopped invalid graph: view step caps ${counters[0]}, Sun-depth unsupported ${counters[3]}, fallback requests ${counters[1]}`);}
  $('status').textContent=`Candidate ${recipe.name} · ${time.toFixed(2)} s · ${gpu===null?'GPU query unavailable':gpu.toFixed(2)+' ms GPU bracket'} · ${(end-encoded).toFixed(2)} ms queue · not accepted`;
}
async function chooseRecipe(){
  const selectors=['variant','model','levels','resolution'];selectors.forEach(id=>$(id).disabled=true);
  pause();await idle();exclusive=true;
  let next,nextProducts;
  try {
    const kind=$('model').value,side=Number($('resolution').value),maxLevel=Number($('levels').value);
    next=kind==='prepared'?await PreparedCu.create(device,side,preparedOptions):await CuSourceField.create(device,kind,Number($('variant').value),side,maxLevel);
    const nextAge=Math.min(next.recipe.duration_s,kind==='prepared'&&recipe?.id!==next.recipe.id?90:age);
    next.prepare(nextAge);
    const newDeepInput=deep.createInputGroup(next.field.levels[0].texture,sceneBuffer);
    nextProducts=fit.createProducts(next.field.levels[0].dimensions);nextProducts.configure(next.field.levels[0].texture,sceneBuffer);
    const commonEntries=()=>[{binding:0,resource:next.field.levels[0].texture.createView()},{binding:1,resource:device.createSampler({minFilter:'linear',magFilter:'linear'})},{binding:2,resource:prefix.createView()},{binding:3,resource:ambient.createView()},{binding:4,resource:{buffer:sceneBuffer}},{binding:5,resource:{buffer:errors}},{binding:7,resource:nextProducts.bankView},...deep.bindings()];
    const cacheEntries=()=>commonEntries().filter(row=>[0,1,4].includes(row.binding));
    const newPrefix=device.createBindGroup({layout:prefixPipeline.getBindGroupLayout(0),entries:cacheEntries()});
    const newAmbient=device.createBindGroup({layout:ambientPipeline.getBindGroupLayout(0),entries:cacheEntries()});
    const newImage=device.createBindGroup({layout:imagePipeline.getBindGroupLayout(0),entries:commonEntries()});
    await device.queue.onSubmittedWorkDone();const previous=sourceObject,previousProducts=fitProducts;
    sourceObject=next;next=null;deepInput=newDeepInput;fitProducts=nextProducts;nextProducts=null;field=sourceObject.field;recipe=sourceObject.recipe;
    clock.duration=recipe.duration_s;age=clock.seek(nextAge,performance.now());$('exact').max=clock.duration;$('seek').max=clock.duration;
    prefixInput=newPrefix;ambientInput=newAmbient;imageInput=newImage;lightCacheKey=null;previous?.destroy();previousProducts?.destroy();
    const names=kind==='prepared'?['Retained Q1 master (one geometry)','Unavailable','Unavailable','Unavailable']:kind==='hierarchy'?['Broad uneven heads','Compact double crown','Long shallow bank','Offset rising crown']:['Broad shallow','Clustered turrets','Narrow rising','Wide asymmetric banks'];
    [...$('variant').options].forEach((option,i)=>option.textContent=names[i]);
    $('identity').textContent=JSON.stringify({identity,recipe,known_limits:['retained Q1 source is one static geometry with finite authored lifecycle; hierarchy source rejected','shape view is geometry-only','legacy point-tau or bounded64x64 deep curves; filtered visibility, point-quality admission failed','8cube/five-axis external ambient','constant receiver diffuse fixture','vacuum/external sky, not SKY-G air','native1280x720; no history; one frame in flight']},null,2);
    resetCamera();
  } catch(error){if(sourceObject){$('model').value=sourceObject.kind;$('variant').value=sourceObject.recipe.variant;$('levels').value=sourceObject.maxLevel;$('resolution').value=sourceObject.nominalSide??sourceObject.field.levels[0].dimensions[0];}throw error;} finally {next?.destroy();nextProducts?.destroy();exclusive=false;selectors.forEach(id=>$(id).disabled=false);if(sourceObject?.kind==='prepared'){$('variant').disabled=true;$('levels').disabled=true;$('variant').value='0';$('levels').value='0';}}
  requestFrame();
}
async function checkSource(){
  pause();await idle();exclusive=true;$('checks').textContent='Checking canonical source against CPU…';
  try {
    await renderFrame(age);
    const points=[];let random=1093;
    for(let i=0;i<192;i++)points.push(recipe.bounds.min.map((v,k)=>{random=(Math.imul(random,1664525)+1013904223)>>>0;return v+random/4294967296*recipe.bounds.extent[k];}));
    if(sourceObject.kind==='prepared')points.push([256,120,256],[256,180,256]);
    else if(sourceObject.kind==='coarse')for(const node of recipe.cohorts)points.push(node.end_m,node.start_m);
    else for(let index=0;index<sourceObject.frame.nodes.length;index+=16){const n=sourceObject.frame.nodes;const c=Array.from(n.slice(index,index+3));points.push(c,[c[0]+n[index+3]*.9,c[1],c[2]]);}
    const validity=sourceObject.kind==='hierarchy'?'hierarchyInputsValid(q.w)':'true';
    const code=FIELD_PARAMETERS_WGSL+sourceObject.shader+PACKED_LOAD_WGSL+`
@group(0) @binding(10) var<storage,read> inputs:array<vec4f>;
@group(0) @binding(11) var<storage,read_write> outputs:array<vec4f>;
@group(1) @binding(0) var actualField:texture_3d<f32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3u) {
 if(id.x<arrayLength(&inputs)){
   let q=inputs[id.x];let valid=${validity};
   let cell=vec3i(floor((q.xyz-field.lo.xyz)/field.extent.xyz*vec3f(field.grid.xyz)));
   let centre=field.lo.xyz+(vec3f(cell)+.5)*(field.extent.xyz/vec3f(field.grid.xyz));
   let stored=loadPackedCell(actualField,cell,field.grid.xyz);
   outputs[id.x]=vec4f(canonicalSigma(q.xyz,q.w),select(0.,1.,valid),stored,roundHalf(canonicalSigma(centre,q.w)));
 }
}`;
    const input=device.createBuffer({size:points.length*16,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST}),output=device.createBuffer({size:points.length*16,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
    device.queue.writeBuffer(input,0,new Float32Array(points.flatMap(p=>[...p,age])));
    const pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,code),entryPoint:'main'}});
    const group=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[...sourceObject.probeEntries(),{binding:10,resource:{buffer:input}},{binding:11,resource:{buffer:output}}]});
    const fieldGroup=device.createBindGroup({layout:pipeline.getBindGroupLayout(1),entries:[{binding:0,resource:field.levels[0].texture.createView()}]});
    const encoder=device.createCommandEncoder(),pass=encoder.beginComputePass();pass.setPipeline(pipeline);pass.setBindGroup(0,group);pass.setBindGroup(1,fieldGroup);pass.dispatchWorkgroups(Math.ceil(points.length/64));pass.end();device.queue.submit([encoder.finish()]);
    const values=new Float32Array((await readBuffer(device,output,points.length*16)).buffer);input.destroy();output.destroy();
    const rows=points.map((point,index)=>{const expected=sourceObject.sample(point,lastFrame.source_time_s),actual=values[index*4];return {point,expected,actual,valid:values[index*4+1],field_stored:values[index*4+2],field_expected:values[index*4+3],field_store_error:Math.abs(values[index*4+2]-values[index*4+3]),error:Math.abs(expected-actual)};});
    const max=Math.max(...rows.map(x=>x.error));const report={identity,scene:lastFrame,source_recipe:recipe,source_shader_sha256:await hash(new TextEncoder().encode(code)),scope:'bounded canonical CPU/WGSL/index and selected actual field-store comparisons; no ray/appearance qualification',max_absolute_sigma_error_per_m:max,pass:max<2e-6&&rows.every(row=>row.valid===1&&row.field_store_error<=1.6e-5),rows};
    await save(`cu-source-${Date.now()}.json`,JSON.stringify(report,null,2));$('checks').textContent=JSON.stringify({pass:report.pass,samples:rows.length,max_absolute_sigma_error_per_m:max,scope:report.scope},null,2);
  } catch(error){$('checks').textContent=error.stack;}finally{exclusive=false;}
}
async function load(){
  identity=await(await fetch('/identity')).json();if(identity.build_id!==document.querySelector('meta[name="proof-build"]').content)throw Error('Build changed; reload');
  const adapter=await navigator.gpu?.requestAdapter({powerPreference:'high-performance'});if(!adapter)throw Error('WebGPU unavailable');
  device=await adapter.requestDevice({requiredFeatures:adapter.features.has('timestamp-query')?['timestamp-query']:[]});
  identity.adapter={vendor:adapter.info.vendor,architecture:adapter.info.architecture,device:adapter.info.device,description:adapter.info.description};
  identity.host={user_agent:navigator.userAgent,dpr:devicePixelRatio,canvas:[canvas.width,canvas.height],url:location.href,prepared_options:preparedOptions,visibility:document.visibilityState};
  device.addEventListener('uncapturederror',event=>{pause();$('status').textContent='GPU validation: '+event.error.message;});
  device.lost.then(info=>{pause();$('status').textContent='Device lost: '+info.message;});
  context=canvas.getContext('webgpu');const format=navigator.gpu.getPreferredCanvasFormat();context.configure({device,format,alphaMode:'opaque',usage:GPUTextureUsage.RENDER_ATTACHMENT|GPUTextureUsage.TEXTURE_BINDING});
  deep=await DeepSun.create(device,compileShader);
  shaderCommon=PACKED_LOAD_WGSL+BEER_WGSL+await(await fetch('cu-preview.wgsl')).text()+deep.queryWGSL;shaderCache=await(await fetch('cu-caches.wgsl')).text();
  const code=await compileShader(device,shaderCommon),cache=await compileShader(device,shaderCommon+shaderCache);
  prefixPipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:cache,entryPoint:'buildPrefix'}});
  ambientPipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:cache,entryPoint:'buildAmbient'}});
  imageLayout=device.createBindGroupLayout({entries:[
    ...[0,2,3].map(binding=>({binding,visibility:GPUShaderStage.FRAGMENT,texture:{sampleType:'float',viewDimension:'3d'}})),
    {binding:1,visibility:GPUShaderStage.FRAGMENT,sampler:{type:'filtering'}},
    {binding:4,visibility:GPUShaderStage.FRAGMENT,buffer:{type:'uniform',minBindingSize:208}},
    {binding:5,visibility:GPUShaderStage.FRAGMENT,buffer:{type:'storage'}},
    {binding:7,visibility:GPUShaderStage.FRAGMENT,texture:{sampleType:'unfilterable-float',viewDimension:'2d'}},
    ...deep.layoutEntries(GPUShaderStage.FRAGMENT)
  ]});
  imagePipeline=await device.createRenderPipelineAsync({layout:device.createPipelineLayout({bindGroupLayouts:[imageLayout]}),vertex:{module:code,entryPoint:'triangle'},fragment:{module:code,entryPoint:'image',targets:[{format}]},primitive:{topology:'triangle-list'}});
  sceneBuffer=device.createBuffer({size:208,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});errors=device.createBuffer({size:16,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST|GPUBufferUsage.COPY_SRC});
  const usage=GPUTextureUsage.TEXTURE_BINDING|GPUTextureUsage.STORAGE_BINDING|GPUTextureUsage.COPY_SRC;
  prefix=device.createTexture({size:[66,66,66],dimension:'3d',format:'rgba16float',usage});ambient=device.createTexture({size:[8,8,8],dimension:'3d',format:'rgba16float',usage});
  prefixOutput=device.createBindGroup({layout:prefixPipeline.getBindGroupLayout(1),entries:[{binding:0,resource:prefix.createView()}]});
  ambientOutput=device.createBindGroup({layout:ambientPipeline.getBindGroupLayout(1),entries:[{binding:1,resource:ambient.createView()}]});
  if(device.features.has('timestamp-query')) {
    querySet=device.createQuerySet({type:'timestamp',count:12});queryResolve=device.createBuffer({size:256,usage:GPUBufferUsage.QUERY_RESOLVE|GPUBufferUsage.COPY_SRC});queryRead=device.createBuffer({size:96,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
    const marker=markerBuffer=device.createBuffer({size:4,usage:GPUBufferUsage.STORAGE});
    sentinelPipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,'@group(0) @binding(0) var<storage,read_write> marker:atomic<u32>; @compute @workgroup_size(1) fn main(){atomicAdd(&marker,1u);}'),entryPoint:'main'}});
    endSentinelPipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,'@group(0) @binding(0) var<storage,read_write> marker:atomic<u32>; @group(0) @binding(1) var rendered:texture_2d<f32>; @compute @workgroup_size(1) fn main(){atomicAdd(&marker,bitcast<u32>(textureLoad(rendered,vec2i(textureDimensions(rendered)/2u),0).x));}'),entryPoint:'main'}});
    sentinelGroup=device.createBindGroup({layout:sentinelPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:marker}}]});
  }
  fit=await ScatterFit.create(device,compileShader);
  await chooseRecipe();
}
$('play').onclick=()=>{if($('reference-light').checked){$('status').textContent='Direct-light patches are single-frame inspection only';return;}if(clock.playing){pause();}else{clock.play(performance.now());$('play').textContent='Pause';requestFrame();}};
$('rate').onchange=()=>{age=clock.setRate(Number($('rate').value),performance.now());requestFrame();};
for(const id of ['exact','seek'])$(id).oninput=()=>{pause();age=clock.seek(Number($(id).value),performance.now());requestFrame();};
for(const id of ['sun','main','reflection','transmittance','view'])$(id).onchange=requestFrame;
for(const id of ['variant','model','levels','resolution'])$(id).onchange=()=>void chooseRecipe().catch(error=>{$('status').textContent=error.stack;});
$('route').onchange=()=>{resetCamera();requestFrame();};
$('reset').onclick=()=>{resetCamera();requestFrame();};$('probes').onclick=()=>void checkSource();
$('capture').onclick=async()=>{pause();await idle();exclusive=true;try{await renderFrame(age);const name=`cu-${Date.now()}`;const blob=await new Promise(resolve=>canvas.toBlob(resolve,'image/png'));await save(name+'.png',blob);await save(name+'.json',JSON.stringify({identity,recipe,frame:lastFrame},null,2));$('checks').textContent='Saved '+name;}catch(error){$('checks').textContent=error.stack;}finally{exclusive=false;}};
canvas.addEventListener('wheel',event=>{event.preventDefault();const scale=event.deltaMode===1?16:1,dx=Math.max(-300,Math.min(300,event.deltaX*scale)),dy=Math.max(-300,Math.min(300,event.deltaY*scale));if(event.ctrlKey){distanceScale=Math.max(.35,Math.min(12,distanceScale*Math.exp(dy*.005)));}else if(event.shiftKey){pan[0]+=dx*2;pan[1]-=dy*2;}else{yaw+=dx*.004;pitch=Math.max(-1.3,Math.min(1.3,pitch+dy*.004));}requestFrame();},{passive:false});
let drag=null;canvas.addEventListener('contextmenu',event=>event.preventDefault());canvas.addEventListener('pointerdown',event=>{drag={x:event.clientX,y:event.clientY,pan:event.button===2||event.shiftKey};canvas.setPointerCapture(event.pointerId);});canvas.addEventListener('pointerup',()=>drag=null);canvas.addEventListener('pointercancel',()=>drag=null);canvas.addEventListener('pointermove',event=>{if(!drag)return;const dx=event.clientX-drag.x,dy=event.clientY-drag.y;drag.x=event.clientX;drag.y=event.clientY;if(drag.pan){pan[0]-=dx*2;pan[1]+=dy*2;}else{yaw-=dx*.006;pitch=Math.max(-1.3,Math.min(1.3,pitch+dy*.006));}requestFrame();});
load().catch(error=>{$('status').textContent=error.stack;});

// Bounded actual-canvas records; never advance source time by a fixed amount per callback.
let sequenceCancel=false;
async function runSequence(record=true){
  if($('reference-light').checked){$('checks').textContent='Disable direct-light reference patches before motion or measurement';return;}
  if(exclusive)return;pause();await idle();exclusive=true;sequenceCancel=false;
  const controls=[...document.querySelectorAll('button,input,select')],disabledBefore=controls.map(control=>control.disabled);controls.forEach(control=>control.disabled=true);$('stop').disabled=false;
  let recorder,stream,chunks=[],samples=[],snapshots=[];
  const preset=$('motion').value,prepared=sourceObject.kind==='prepared';
  const settings=record?(preset==='growth'?[prepared?40:210,prepared?60:230,1]:preset==='breakup'?[prepared?200:1470,prepared?220:1490,1]:[0,recipe.duration_s,preset==='full16'?16:64]):[prepared?90:650,Infinity,1];
  const [start,end,rate]=settings,name=`${record?'motion':'cost'}-${Date.now()}-${record?preset:'fresh-source-1x'}`,identityStart={identity,recipe,scene:lastFrame,source_kind:sourceObject.kind,rate,start_source_s:start,end_source_s:Number.isFinite(end)?end:null};
  try{
    await renderFrame(start);identityStart.scene={...lastFrame};let snapshotIndex=0;
    if(record){stream=canvas.captureStream(30);recorder=new MediaRecorder(stream,{mimeType:'video/webm;codecs=vp9',videoBitsPerSecond:5000000});recorder.ondataavailable=event=>{if(event.data.size)chunks.push(event.data);};recorder.start(500);}
    const began=performance.now();let index=0;
    while(!sequenceCancel){
      const wall=(performance.now()-began)/1000,target=Math.min(end,start+wall*rate);
      await renderFrame(target);samples.push({...lastFrame,wall_source_request_s:wall,wall_complete_s:(performance.now()-began)/1000,warmup:!record&&index<30});index++;
      if(record&&snapshots.length<9&&(performance.now()-began)/1000>=snapshotIndex*(end-start)/rate/8){
        const filename=`${name}-${snapshotIndex}.png`;await save(filename,await new Promise(resolve=>canvas.toBlob(resolve,'image/png')));snapshots.push({file:filename,frame:lastFrame});snapshotIndex++;
      }
      $('checks').textContent=`${record?'Recording actual canvas':'Measuring fresh-source fixture'}: ${samples.length} frames, source ${target.toFixed(2)} s`;
      if(target>=end||(!record&&index>=330))break;
      await new Promise(resolve=>requestAnimationFrame(resolve));
    }
    if(recorder){await new Promise(resolve=>{recorder.onstop=resolve;recorder.stop();});await save(name+'.webm',new Blob(chunks,{type:'video/webm'}));}
    await save(name+'.json',JSON.stringify({...identityStart,cancelled:sequenceCancel,samples,snapshots,scope:record?'Actual canvas with sampled native PNGs; recorder/readback observer cost included, not a quiet benchmark':'330 actual sequential rendered frames, first30 warmup; source time follows wall; paused camera; no recorder/PNGs; standalone fixture only'},null,2));
    $('checks').textContent='Saved '+name+` (${samples.length} frames)`;
  }catch(error){$('checks').textContent=error.stack;}
  finally{stream?.getTracks().forEach(track=>track.stop());clock.seek(age,performance.now());controls.forEach((control,index)=>control.disabled=disabledBefore[index]);$('stop').disabled=true;exclusive=false;}
}
$('record').onclick=()=>void runSequence(true);$('measure').onclick=()=>void runSequence(false);$('stop').onclick=()=>{sequenceCancel=true;};

$('fit-probes').onclick=async()=>{
  pause();await idle();exclusive=true;
  try{
    if($('light-model').value!=='2'||$('view').value!=='0'||new Set(['0','sky']).has($('sun').value))throw Error('Use lit fitted model with Sun enabled for matching fit products');
    await renderFrame(age);const receipt=await checkScatterFit(device,fit,fitProducts,sourceObject,lastFrame);
    const name=`fit-probe-${Date.now()}`;await save(name+'.json',JSON.stringify({identity,scene:lastFrame,...receipt},null,2));
    $('checks').textContent=JSON.stringify({saved:name,implementation_checks_pass:receipt.implementation_checks_pass,LUT:{...receipt.LUT,rows:undefined},storage:{...receipt.centre_bank_storage,rows:undefined},body_quality:{...receipt.body_quality,rows:undefined},bank_interpolation:{...receipt.A_map_nonlinearity,rows:undefined}},null,2);
  }catch(error){$('checks').textContent=error.stack;}finally{exclusive=false;}
};

$('depth-model').onchange=()=>{if($('depth-model').value==='legacy')$('reference-light').checked=false;if($('depth-model').value==='deep'&&$('light-model').value==='0')$('light-model').value='2';requestFrame();};
$('light-model').onchange=()=>{if($('light-model').value==='0'){$('depth-model').value='legacy';$('reference-light').checked=false;}requestFrame();};

$('deep-probes').onclick=async()=>{
  if(exclusive)return;pause();await idle();exclusive=true;
  const controls=[...document.querySelectorAll('button,input,select')],disabled=controls.map(control=>control.disabled);controls.forEach(control=>control.disabled=true);
  try{
    if(lastFrame?.depth_model!=='deep'||$('depth-model').value!=='deep')throw Error('Render the bounded depth-curve mode first');
    const {captureDeepSun}=await import('./deep-probes.js');
    const result=await captureDeepSun(device,deep,field,sceneBuffer,lastFrame,lastUniformWords);
    const name=`deep-probe-${Date.now()}`;
    for(const [key,bytes] of Object.entries(result.binaries)){
      const filename=name+'-'+result.receipt.binaries[key].filename;
      result.receipt.binaries[key].filename=filename;await save(filename,bytes);
    }
    await save(name+'.json',JSON.stringify({...result.receipt,build_identity:identity},null,2));
    $('checks').textContent='Saved '+name+' (actual GPU buffers, input paths and point queries; quality unqualified)';
  }catch(error){$('checks').textContent=error.stack;}
  finally{controls.forEach((control,index)=>control.disabled=disabled[index]);exclusive=false;}
};

$('reference-light').onchange=()=>{
  if($('reference-light').checked){pause();$('depth-model').value='deep';if($('light-model').value==='0')$('light-model').value='2';}
  requestFrame();
};

$('prepared-probes').onclick=async()=>{
  if(exclusive)return;pause();await idle();exclusive=true;
  const controls=[...document.querySelectorAll('button,input,select')],disabled=controls.map(control=>control.disabled);controls.forEach(control=>control.disabled=true);
  try{
    if(sourceObject.kind!=='prepared')throw Error('Use the retained prepared source for this comparison');
    const {capturePreparedFieldComparison}=await import('./prepared-field-probes.js');
    const receipt=await capturePreparedFieldComparison(device,sourceObject.nominalSide,Math.fround(age));
    const name=`prepared-field-${Date.now()}`;await save(name+'.json',JSON.stringify({identity,...receipt},null,2));
    $('checks').textContent='Saved '+name+' (actual field/mip/producer comparisons; inspect receipt for result)';
  }catch(error){$('checks').textContent=error.stack;}
  finally{controls.forEach((control,index)=>control.disabled=disabled[index]);exclusive=false;}
};
