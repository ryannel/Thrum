import assert from 'node:assert/strict';
import {FrameSettlement,assertGpuHealthy,frameFence} from '../src/frame-lifecycle.js';
import {recordCanvas,MAX_CLIP_BYTES} from '../src/motion.js';
const settlement=new FrameSettlement(),pending=settlement.wait(true),error=Error('injected GPU loss');
const rejected=assert.rejects(pending,e=>e===error);settlement.fail(error);await rejected;
await assert.rejects(settlement.wait(true),e=>e===error);await assert.rejects(settlement.wait(false),e=>e===error);settlement.complete();await assert.rejects(settlement.wait(false),e=>e===error);
assert.throws(()=>assertGpuHealthy({lost:true,errors:[]}),/device lost/);assert.throws(()=>assertGpuHealthy({lost:false,errors:['injected validation failure']}),/validation failure/);
let lose,finishFence,lossListeners=0,fenceCalls=0;
const lostPromise=new Promise(r=>lose=r),device={lost:{then(fn){lossListeners++;return lostPromise.then(fn);}},queue:{onSubmittedWorkDone(){fenceCalls++;return new Promise(r=>finishFence=r);}}};
const gpu={device,lost:false,errors:[]};let complete=false;
const late=frameFence(gpu).then(()=>complete=true);await Promise.resolve();assert.equal(complete,false);finishFence();await late;assert.equal(complete,true,'late healthy fence completes normally');
for(let i=0;i<5;i++){const f=frameFence(gpu);finishFence();await f;}assert.equal(lossListeners,1,'one listener per device, not one retained listener per frame');
const neverFence=frameFence(gpu),lossRejected=assert.rejects(neverFence,/injected lost message/);lose({message:'injected lost message'});await lossRejected;await assert.rejects(frameFence(gpu),/injected lost message/);assert.equal(fenceCalls,7,'lost watch prevents further queue fences');
// Real recorder retention logic, injected encoder chunks. No 90/120 s wall-time wait.
const nativeRecorder=globalThis.MediaRecorder,nativeSet=setTimeout,nativeClear=clearTimeout;let stopTracks=0,deadline,last,stopCalls=0;
globalThis.setTimeout=fn=>{deadline=fn;return 1;};globalThis.clearTimeout=()=>{};
class Recorder{static isTypeSupported(){return true;}constructor(){last=this;this.state='inactive';}start(){this.state='recording';}emit(bytes){this.ondataavailable({data:new Blob([new Uint8Array(bytes)])});}stop(){stopCalls++;this.state='inactive';this.emit(17);queueMicrotask(()=>this.onstop());}}
globalThis.MediaRecorder=Recorder;const canvas={captureStream:()=>({getTracks:()=>[{stop(){stopTracks++;}}]})};
try{
 let deadlines=0;const partial=recordCanvas(canvas,120000,{onDeadline:()=>deadlines++});last.emit(1024**2);last.emit(13*1024**2);const out=await partial;
 assert.equal(out.abortReason,'encoded-size-limit');assert.equal(out.aborted,true);assert.equal(out.blob.size,1024**2);assert.equal(out.retainedEncodedByteLimit,MAX_CLIP_BYTES);assert.equal(out.droppedEncodedBytes,13*1024**2+17);assert.equal(deadlines,1);assert.equal(stopCalls,1);
 const empty=recordCanvas(canvas,90000),capacityRejected=assert.rejects(empty,e=>e.code==='encoded-size-limit'&&/before any media/.test(e.message));last.emit(13*1024**2);await capacityRejected;assert.equal(stopTracks,2,'first-chunk capacity failure stops tracks');
 const cumulative=recordCanvas(canvas,12000);last.emit(8*1024**2);last.emit(5*1024**2);const sum=await cumulative;assert.equal(sum.abortReason,'encoded-size-limit');assert.equal(sum.blob.size,8*1024**2,'cumulative cap rejects overflow chunk');
 const exact=recordCanvas(canvas,12000);last.emit(MAX_CLIP_BYTES-17);deadline();const full=await exact;assert.equal(full.blob.size,MAX_CLIP_BYTES);assert.equal(full.aborted,false,'exact limit retained');assert.equal(stopTracks,4);
}finally{globalThis.MediaRecorder=nativeRecorder;globalThis.setTimeout=nativeSet;globalThis.clearTimeout=nativeClear;}
console.log('Terminal settled rejection, pre-render GPU faults, healthy late fence/lost-device interruption, bounded device listener, 13 MiB first/partial/cumulative overflow, exact cap and track cleanup passed (mocked GPU/encoder).');
