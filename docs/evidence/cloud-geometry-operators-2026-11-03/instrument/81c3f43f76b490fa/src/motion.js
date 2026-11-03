// This bounds our retained encoded chunks, not the browser's internal encoder buffer.
export const MAX_CLIP_BYTES=12*1024**2;
// Actual rendered canvas recording. Capture request FPS is not a delivered-FPS claim.
export async function recordCanvas(canvas,durationMs=12000,{signal,onStart=()=>{},onDeadline=()=>{}}={}){
 if(!canvas.captureStream||!globalThis.MediaRecorder)throw Error('Canvas recording unavailable');
 const mimeType=['video/webm;codecs=vp9','video/webm;codecs=vp8','video/webm'].find(t=>MediaRecorder.isTypeSupported(t));if(!mimeType)throw Error('WebM recording unavailable');
 const stream=canvas.captureStream(15),chunks=[];let started,abortReason=null,stopRequestedPerformanceMs=null,retainedBytes=0,droppedEncodedBytes=0,capacityExceeded=false;
 try{return await new Promise((resolve,reject)=>{
  const recorder=new MediaRecorder(stream,{mimeType,videoBitsPerSecond:durationMs>60000?750000:1500000});let timer;
  const cleanup=()=>{clearTimeout(timer);signal?.removeEventListener('abort',abort);};
  const stop=()=>{if(stopRequestedPerformanceMs!==null)return;stopRequestedPerformanceMs=performance.now();clearTimeout(timer);onDeadline();if(recorder.state!=='inactive')recorder.stop();};
  const abort=()=>{if(!capacityExceeded)abortReason=String(signal.reason??'aborted');stop();};
  recorder.ondataavailable=e=>{
   if(!e.data.size)return;
   if(capacityExceeded||retainedBytes+e.data.size>MAX_CLIP_BYTES){capacityExceeded=true;abortReason='encoded-size-limit';droppedEncodedBytes+=e.data.size;stop();return;}
   retainedBytes+=e.data.size;chunks.push(e.data);
  };
  recorder.onerror=e=>{cleanup();reject(Error(e.error?.message??'Motion recorder failed'));};
  recorder.onstop=()=>{
   cleanup();
   if(retainedBytes>MAX_CLIP_BYTES){chunks.length=0;reject(Error('Encoded clip exceeds retained 12 MiB limit'));return;}
   if(!retainedBytes){const error=Error(capacityExceeded?'Encoded clip capacity exceeded before any media could be retained':'Motion recorder produced no frames');if(capacityExceeded)error.code='encoded-size-limit';reject(error);return;}
   const blob=new Blob(chunks,{type:mimeType});chunks.length=0;
   if(blob.size>MAX_CLIP_BYTES){reject(Error('Encoded clip exceeds retained 12 MiB limit'));return;}
   resolve({blob,mimeType,elapsedMs:performance.now()-started,stopRequestedPerformanceMs,stoppedPerformanceMs:performance.now(),requestedFps:15,aborted:abortReason!==null,abortReason,retainedEncodedByteLimit:MAX_CLIP_BYTES,droppedEncodedBytes});
  };
  recorder.start(1000);started=performance.now();onStart();signal?.addEventListener('abort',abort,{once:true});timer=setTimeout(stop,durationMs);if(signal?.aborted)abort();
 });}finally{for(const track of stream.getTracks())track.stop();}
}
