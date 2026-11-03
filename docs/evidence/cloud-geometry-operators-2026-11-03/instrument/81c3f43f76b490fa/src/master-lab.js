import { GeometryMaster } from './geometry-adapter.js';
import { Weather } from './weather.js';
import { ProofWorldClock, motionOutcome } from './proof-clock.js';
import { recordCanvas, MAX_CLIP_BYTES } from './motion.js';
import { FrameSettlement, assertGpuHealthy, frameFence } from './frame-lifecycle.js';
const $=id=>document.getElementById(id),canvas=$('canvas');
const makeWeather=()=>new Weather(71,{wind:0,lift:0,moist:0,dry:0});
let weather=makeWeather(),adapter,identity,baseline,loading=true,fatal=false;
let dirty=false,queued=false,drawing=false,lastFrame=null,lastSubmitted=null,serial=0;
const settlement=new FrameSettlement();
let animationHandle=null,fixtureMode=false,revision=0,recording=null;
const proofWorldClock=new ProofWorldClock(),observation=crypto.randomUUID().slice(0,8);
const info=t=>$('status').textContent=t;
function options(){const c=proofWorldClock.snapshot();return {family:1,provider:0,sun:Number($('sun').value),mode:Number($('mode').value),mirror:$('mirror').checked,main:$('main').checked,fog:false,resolved:$('full').checked,viewSteps:Number($('steps').value),extinctionScale:Number($('kappa').value)/.0045,preparedDensity:true,materialStructure:false,materialSpace:'world',materialFilter:false,thermals:false,materialCore:false,connectedFeed:false,frontMorph:false,neckMix:false,dualRest:false,emptyWork:false,cachedVelocity:false,alpha:1,sourceDiagnostic:false,...adapter.parameters(Number($('operator').value),fixtureMode?c.exposureWorldM:Number($('exposure').value),$('channels').value,fixtureMode)};}
function controls(disabled){for(const el of document.querySelectorAll('#controls input,#controls select,#controls button'))el.disabled=disabled;}
function fail(error){if(fatal)return;fatal=true;dirty=false;queued=false;settlement.fail(error);pause(false);recording?.controller.abort('faulted');controls(true);info('Inspection unavailable · '+error.message);console.error(error);$('receipt').textContent=error.stack??String(error);}
function clockLabel(){const c=proofWorldClock.snapshot();$('clock').textContent=`${c.playing?'Playing':'Paused'} · source ${c.worldTime.toFixed(2)} s · phase ${c.trajectoryPhaseSeconds.toFixed(2)}/120 s${c.trajectoryClamped?' (held endpoint)':''} · ${c.rate}×`;}
function animation(now){animationHandle=null;if(!proofWorldClock.playing||fatal)return;proofWorldClock.advance(performance.now());clockLabel();requestRender();animationHandle=requestAnimationFrame(animation);}
function play(){if(loading||fatal)return;if(!fixtureMode)reset(false);proofWorldClock.play(performance.now());clockLabel();if(animationHandle===null)animationHandle=requestAnimationFrame(animation);requestRender();}
function pause(abort=true){if(abort&&recording)recording.controller.abort('paused-by-user');proofWorldClock.pause(performance.now());if(animationHandle!==null)cancelAnimationFrame(animationHandle);animationHandle=null;clockLabel();if(adapter&&!fatal)requestRender();}
function reset(abort=true){if(abort&&recording)recording.controller.abort('reset-by-user');pause(false);proofWorldClock.reset(performance.now());weather=makeWeather();fixtureMode=true;clockLabel();if(adapter)requestRender();}
function changed(){revision++;if(recording)recording.controller.abort('fixture-changed');pause(false);}
function requestRender(){if(fatal)return;dirty=true;if(!loading&&!queued&&!drawing){queued=true;requestAnimationFrame(draw);}}
function advanceWeather(t){while(weather.time+1<=t+1e-8)weather.advanceTo(t,128);}
async function draw(){queued=false;if(!adapter||loading||fatal)return;dirty=false;drawing=true;
 try{
  // Capture the complete tuple synchronously at actual submission, before any await.
  proofWorldClock.advance(performance.now());const clock=proofWorldClock.snapshot(),s=adapter.scene,ui=options();advanceWeather(clock.worldTime);s.time=clock.worldTime;s.steps=0;
  assertGpuHealthy(s.gpu);
  $('matched').textContent=`Actual E ${ui.geometryExposure.toFixed(2)} m · ρ ${ui.geometryRhoThreshold.toPrecision(4)} · α ${ui.geometryAlpha.toFixed(4)}`;
  if(!s.render(weather,ui,0,clock.worldTime)){assertGpuHealthy(s.gpu);dirty=true;return;}
  const submitted={asset:structuredClone(adapter.describe()),presentation:structuredClone(s.lastPresentation),underlyingMasterFrozen:true,effectiveSourceFrozen:!clock.playing,sourceMode:fixtureMode?'analytic parameter fixture':'manual held geometry',effectiveSourceTime:clock.worldTime,worldTime:clock.worldTime,sourceSteps:0,proofClock:clock,trajectory:fixtureMode?{exposureStartWorldM:0,exposureEndWorldM:100,durationSourceSeconds:120,phaseSeconds:clock.trajectoryPhaseSeconds,clamped:clock.trajectoryClamped,fullExtinctionClaim:false}:null,fixtureRevision:revision,submittedPerformanceMs:performance.now(),logicalRendererBytes:s.gpu.bytes,logicalAllocations:structuredClone(s.gpu.allocations),cameraPreset:$('camera').value};
  lastSubmitted=submitted;
  await frameFence(s.gpu);
  lastFrame={...submitted,gpuQueueCompletedPerformanceMs:performance.now(),completionScope:'Queue fence completed; browser display/presentation time not measured'};
  info(`Master ${adapter.asset.id} · ${adapter.asset.side}³ · op ${ui.geometryOperator} · E ${ui.geometryExposure.toFixed(2)}m · source ${clock.worldTime.toFixed(2)}s · ${ui.viewSteps} steps · build ${identity.build}`);
 }catch(error){fail(error);}finally{drawing=false;if(fatal){dirty=false;queued=false;}else if(dirty)requestRender();else settlement.complete();}}
function settled(){return settlement.wait(dirty||queued||drawing);}
async function saveReceipt(name,data){const s=adapter.scene;const record={name,build:identity.build,identity,rendererBaseline:baseline.baseline_build,observation,host:{adapter:s.gpu.adapter.info?{vendor:s.gpu.adapter.info.vendor,architecture:s.gpu.adapter.info.architecture,device:s.gpu.adapter.info.device,description:s.gpu.adapter.info.description}:null,devicePixelRatio,screen:[screen.width,screen.height]},...data};$('receipt').textContent=JSON.stringify(record,null,2);const r=await fetch('/receipt',{method:'POST',body:JSON.stringify(record)});if(!r.ok)throw Error(`Receipt save ${r.status}`);return record;}
async function capture(){pause();controls(true);try{await settled();if(!lastFrame)throw Error('No completed frame');const frame=structuredClone(lastFrame),name=`geometry-${frame.asset.id}-${frame.asset.side}-op${frame.presentation.controls.geometryOperator}-e${Math.round(frame.presentation.controls.geometryExposure*1000)}mm-${frame.cameraPreset}-${identity.build}-${observation}-${++serial}`;const blob=await new Promise(resolve=>canvas.toBlob(resolve,'image/png'));if(!blob)throw Error('Capture encoding failed');const r=await fetch('/capture',{method:'POST',headers:{'X-Capture-Name':name},body:blob});if(!r.ok)throw Error(`Capture save ${r.status}`);await saveReceipt(name,{...frame,capture:{file:name+'.png',bytes:blob.size}});info('Saved '+name);}finally{if(!fatal)controls(false);}}
$('capture').onclick=()=>capture().catch(fail);
$('probe').onclick=async()=>{pause();controls(true);try{await settled();loading=true;const frame=structuredClone(lastFrame),ui=frame.presentation.controls,probe=await adapter.probe(ui,weather.sample(frame.worldTime));await saveReceipt(`geometry-probe-${adapter.asset.side}-${identity.build}-${observation}-${++serial}`,{...frame,probe});info(probe.pass?'Upload and bounded shared coefficient query passed':'Probe reported a failed invariant');}catch(error){fail(error);}finally{loading=false;if(!fatal)controls(false);if(dirty)requestRender();}};
function recordState(){const clock=proofWorldClock.snapshot();return {revision,epoch:clock.epoch,sourceTime:clock.worldTime,clock,playing:clock.playing,fault:fatal,gpuErrors:adapter.scene.gpu.errors.length,imageFrame:lastFrame?.presentation.frame??-1,lastCompletedFrame:lastFrame?structuredClone(lastFrame):null,lastSubmittedFrame:lastSubmitted?structuredClone(lastSubmitted):null,wallPerformanceMs:performance.now()};}
async function record(seconds){if(recording||loading||fatal)return;reset(false);controls(true);await settled();controls(false);const controller=new AbortController(),state={controller,start:null,end:null};recording=state;$('record-status').textContent=`Recording ${seconds} actual wall seconds…`;
 try{
  const result=await recordCanvas(canvas,seconds*1000,{signal:controller.signal,onStart:()=>{play();state.start=recordState();},onDeadline:()=>{pause(false);state.end=recordState();}});
  if(!state.end){pause(false);state.end=recordState();}
  await settled();const outcome=result.aborted?String(result.abortReason??'aborted'):motionOutcome(state.start,state.end),name=`geometry-motion-op${state.start.lastCompletedFrame.presentation.controls.geometryOperator}-${state.start.clock.rate}x-${seconds}s-${identity.build}-${observation}-${++serial}`;
  if(result.blob.size>MAX_CLIP_BYTES)throw Error('Encoded clip exceeds retained 12 MiB limit');
  const r=await fetch('/motion',{method:'POST',headers:{'X-Capture-Name':name},body:result.blob});if(!r.ok)throw Error(`Motion save ${r.status}`);
  await saveReceipt(name,{underlyingMasterFrozen:true,effectiveSourceFrozen:false,sourceMode:'analytic parameter fixture',motion:{file:name+'.webm',mimeType:result.mimeType,bytes:result.blob.size,retainedEncodedByteLimit:MAX_CLIP_BYTES,droppedEncodedBytes:result.droppedEncodedBytes,encoderInternalBufferMeasured:false,partialClipPlaybackValidated:false,requestedWallSeconds:seconds,actualRecorderWallMs:result.elapsedMs,actualSourceRunningWallMs:state.end.wallPerformanceMs-state.start.wallPerformanceMs,recorderStopRequestPerformanceMs:result.stopRequestedPerformanceMs,recorderStoppedPerformanceMs:result.stoppedPerformanceMs,requestedCaptureFps:result.requestedFps,deliveredFpsClaim:false,outcome,aborted:result.aborted,start:state.start,endAtStopRequest:state.end,afterStopSettledFrame:structuredClone(lastFrame),sourceWindow:[state.start.sourceTime,state.end.sourceTime],sourceWindowScope:'Clock drive interval until stop request; encoded frame endpoints unmeasured, particularly after a dropped overflow chunk',trajectoryWindow:[state.start.clock.trajectoryPhaseSeconds,state.end.clock.trajectoryPhaseSeconds],scope:'Actual canvas stream; submitted/completed frames and stop clock explicit, display latency unmeasured. Effective exposure holds E100 after phase120 while proof clock continues. Frozen master, no fragment translation, no PDE/full source lifecycle claim.'}});
  $('record-status').textContent=`Saved ${outcome} · ${result.elapsedMs.toFixed(0)} ms actual wall · source end ${state.end.sourceTime.toFixed(2)} s`;info('Saved '+name);
 }finally{recording=null;if(!fatal)controls(false);}
}
$('play').onclick=play;$('pause').onclick=()=>pause();$('reset').onclick=()=>{changed();reset();};
$('rate').onchange=()=>{changed();proofWorldClock.setRate(Number($('rate').value),performance.now());clockLabel();};
$('record12').onclick=()=>record(12).catch(fail);$('record90').onclick=()=>record(90).catch(fail);$('record120').onclick=()=>record(120).catch(fail);
function updateExposure(value){if(!adapter)return;const max=adapter.maxExposure($('channels').value),e=Math.max(0,Math.min(max,Number(value)||0));$('exposure').max=max;$('exposure-value').max=max;$('exposure').value=e;$('exposure-value').value=e;requestRender();}
async function changeSource(){changed();controls(true);try{await settled();loading=true;await adapter.selectGeometry(Number($('quality').value),Number($('iso').value));}catch(error){fail(error);}finally{loading=false;if(!fatal){controls(false);updateExposure($('exposure').value);}}}
$('quality').onchange=changeSource;$('iso').onchange=changeSource;
function manualExposure(value){changed();fixtureMode=false;updateExposure(value);}
$('exposure').oninput=()=>manualExposure($('exposure').value);$('exposure-value').onchange=()=>manualExposure($('exposure-value').value);
$('channels').onchange=()=>{changed();updateExposure($('exposure').value);};$('operator').onchange=()=>{changed();requestRender();};
function preset(){changed();const c=adapter.scene.camera,v=$('camera').value;Object.assign(c,{yaw:v==='front'?0:v==='side'?Math.PI/2:.3,pitch:v==='high'?.55:-.05,distance:Number($('distance').value)||c.distance,panX:0,panY:0});requestRender();}
$('camera').onchange=preset;$('distance').onchange=()=>{changed();const value=Number($('distance').value);if(value>0){adapter.scene.camera.distance=value;requestRender();}};
for(const id of ['sun','kappa','mode','steps','full','mirror','main'])$(id).onchange=()=>{changed();requestRender();};
let drag=null;canvas.onpointerdown=e=>{if(loading||fatal)return;canvas.setPointerCapture(e.pointerId);drag={x:e.clientX,y:e.clientY,pan:e.shiftKey||e.button===2};};canvas.onpointerup=()=>drag=null;canvas.onpointercancel=()=>drag=null;
canvas.onpointermove=e=>{if(!drag||loading||fatal)return;changed();const c=adapter.scene.camera,dx=e.clientX-drag.x,dy=e.clientY-drag.y;if(drag.pan){c.panX-=dx*c.distance*.001;c.panY+=dy*c.distance*.001;}else{c.yaw-=dx*.004;c.pitch=Math.max(-.7,Math.min(.8,c.pitch+dy*.004));}drag.x=e.clientX;drag.y=e.clientY;requestRender();};
canvas.oncontextmenu=e=>e.preventDefault();canvas.addEventListener('wheel',e=>{e.preventDefault();if(loading||fatal)return;changed();const c=adapter.scene.camera,dx=Math.max(-120,Math.min(120,e.deltaX)),dy=Math.max(-120,Math.min(120,e.deltaY));if(e.ctrlKey||e.metaKey){c.distance=Math.max(1500,Math.min(20000,c.distance*Math.exp(dy*.006)));$('distance').value='custom';}else if(e.shiftKey){c.panX-=dx*c.distance*.0005;c.panY+=dy*c.distance*.0005;}else{c.yaw-=dx*.003;c.pitch=Math.max(-.7,Math.min(.8,c.pitch+dy*.003));}requestRender();},{passive:false});
controls(true);clockLabel();
try{identity=await fetch('identity.json').then(r=>r.json());baseline=await fetch('assets/renderer-baseline.json').then(r=>r.json());adapter=await GeometryMaster.create(canvas,info,256,.001);loading=false;controls(false);updateExposure(0);}catch(error){loading=false;fail(error);}
