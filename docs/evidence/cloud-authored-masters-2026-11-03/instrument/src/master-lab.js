import { FrozenMaster } from './master-adapter.js';
import { Weather } from './weather.js';
const $=id=>document.getElementById(id),canvas=$('canvas'),weather=new Weather(71,{wind:0,lift:0,moist:0,dry:0});
let adapter,identity,baseline,loading=true,dirty=false,queued=false,drawing=false,lastFrame=null,serial=0,waiters=[];
const observation=crypto.randomUUID().slice(0,8);
const info=t=>$('status').textContent=t;
const options=()=>({family:1,provider:0,sun:Number($('sun').value),mode:Number($('mode').value),mirror:$('mirror').checked,main:$('main').checked,fog:false,resolved:$('full').checked,viewSteps:Number($('steps').value),extinctionScale:Number($('kappa').value)/.0045,preparedDensity:true,materialStructure:false,materialSpace:'world',materialFilter:false,thermals:false,materialCore:false,connectedFeed:false,frontMorph:false,neckMix:false,dualRest:false,emptyWork:false,cachedVelocity:false,alpha:1,sourceDiagnostic:false});
function controls(disabled){for(const el of document.querySelectorAll('#controls input,#controls select,#controls button'))el.disabled=disabled;}
function fail(error){info('Inspection unavailable · '+error.message);console.error(error);$('receipt').textContent=error.stack??String(error);for(const w of waiters.splice(0))w.reject(error);}
function requestRender(){dirty=true;if(!loading&&!queued&&!drawing){queued=true;requestAnimationFrame(draw);}}
async function draw(){queued=false;if(!adapter||loading)return;dirty=false;drawing=true;
 try {const s=adapter.scene,ui=options();if(!s.render(weather,ui,0,0))throw Error('Static frame was not submitted');await s.gpu.device.queue.onSubmittedWorkDone();if(s.gpu.errors.length)throw Error(s.gpu.errors.join('; '));
  lastFrame={asset:adapter.describe(),presentation:structuredClone(s.lastPresentation),sourceFrozen:true,worldTime:0,sourceSteps:0,logicalRendererBytes:s.gpu.bytes,logicalAllocations:structuredClone(s.gpu.allocations),cameraPreset:$('camera').value};
  info(`Master ${adapter.asset.id} · ${adapter.asset.side}³ R16 · frozen · κ ${$('kappa').value}/m · ${ui.viewSteps} steps · build ${identity.build}`);
 }catch(error){fail(error);}finally{drawing=false;if(dirty)requestRender();else for(const w of waiters.splice(0))w.resolve();}}
async function settled(){if(dirty||queued||drawing)await new Promise((resolve,reject)=>waiters.push({resolve,reject}));}
async function saveReceipt(name,data){const s=adapter.scene;const record={name,build:identity.build,identity,rendererBaseline:baseline.baseline_build,observation,host:{adapter:s.gpu.adapter.info??null,devicePixelRatio,screen:[screen.width,screen.height]},sourceFrozen:true,worldTime:0,...data};
 $('receipt').textContent=JSON.stringify(record,null,2);const r=await fetch('/receipt',{method:'POST',body:JSON.stringify(record)});if(!r.ok)throw Error(`Receipt save${r.status}`);return record;}
async function capture(){await settled();if(!lastFrame)throw Error('No completed static frame');const frame=structuredClone(lastFrame),name=`master-${frame.asset.id}-${frame.asset.side}-${frame.cameraPreset}-mode${frame.presentation.controls.mode}-${identity.build}-${observation}-${++serial}`;
 const blob=await new Promise(resolve=>canvas.toBlob(resolve,'image/png'));if(!blob)throw Error('Capture encoding failed');const r=await fetch('/capture',{method:'POST',headers:{'X-Capture-Name':name},body:blob});if(!r.ok)throw Error(`Capture save${r.status}`);await saveReceipt(name,{...frame,capture:{file:name+'.png',bytes:blob.size}});info('Saved '+name);}
$('capture').onclick=()=>capture().catch(fail);
$('probe').onclick=async()=>{controls(true);try{await settled();const frame=structuredClone(lastFrame),probe=await adapter.probe();await saveReceipt(`upload-${adapter.asset.id}-${identity.build}-${observation}-${++serial}`,{...frame,probe});info(probe.pass?'Aliased scalar upload and sampled phase defaults match':'Upload verification failed');}catch(error){fail(error);}finally{controls(false);}};
async function changeSource(){controls(true);try{await settled();loading=true;await adapter.select($('master').value,Number($('quality').value));}catch(error){fail(error);}finally{loading=false;controls(false);requestRender();}}
$('master').onchange=changeSource;$('quality').onchange=changeSource;
function preset(){const c=adapter.scene.camera,v=$('camera').value;Object.assign(c,{yaw:v==='front'?0:v==='side'?Math.PI/2:.3,pitch:v==='high'?.55:-.05,distance:Number($('distance').value)||c.distance,panX:0,panY:0});requestRender();}
$('camera').onchange=preset;$('distance').onchange=()=>{const value=Number($('distance').value);if(value>0){adapter.scene.camera.distance=value;requestRender();}};
for(const id of ['sun','kappa','mode','steps','full','mirror','main'])$(id).onchange=requestRender;
let drag=null;canvas.onpointerdown=e=>{if(loading)return;canvas.setPointerCapture(e.pointerId);drag={x:e.clientX,y:e.clientY,pan:e.shiftKey||e.button===2};};canvas.onpointerup=()=>drag=null;canvas.onpointercancel=()=>drag=null;
canvas.onpointermove=e=>{if(!drag||loading)return;const c=adapter.scene.camera,dx=e.clientX-drag.x,dy=e.clientY-drag.y;if(drag.pan){c.panX-=dx*c.distance*.001;c.panY+=dy*c.distance*.001;}else{c.yaw-=dx*.004;c.pitch=Math.max(-.7,Math.min(.8,c.pitch+dy*.004));}drag.x=e.clientX;drag.y=e.clientY;requestRender();};
canvas.oncontextmenu=e=>e.preventDefault();canvas.addEventListener('wheel',e=>{e.preventDefault();if(loading)return;const c=adapter.scene.camera,dx=Math.max(-120,Math.min(120,e.deltaX)),dy=Math.max(-120,Math.min(120,e.deltaY));if(e.ctrlKey||e.metaKey){c.distance=Math.max(1500,Math.min(20000,c.distance*Math.exp(dy*.006)));$('distance').value='custom';}else if(e.shiftKey){c.panX-=dx*c.distance*.0005;c.panY+=dy*c.distance*.0005;}else{c.yaw-=dx*.003;c.pitch=Math.max(-.7,Math.min(.8,c.pitch+dy*.003));}requestRender();},{passive:false});
controls(true);
try {identity=await fetch('identity.json').then(r=>r.json());baseline=await fetch('assets/renderer-baseline.json').then(r=>r.json());adapter=await FrozenMaster.create(canvas,info,'03',256);loading=false;controls(false);requestRender();}catch(error){loading=false;fail(error);}
