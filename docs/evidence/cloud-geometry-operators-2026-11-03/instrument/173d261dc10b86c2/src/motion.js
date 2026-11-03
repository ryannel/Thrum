// Actual rendered canvas recording. Capture request FPS is not a delivered-FPS claim.
export async function recordCanvas(canvas,durationMs=12000,{signal,onStart=()=>{},onDeadline=()=>{}}={}){
 if(!canvas.captureStream||!globalThis.MediaRecorder)throw Error('Canvas recording unavailable');
 const mimeType=['video/webm;codecs=vp9','video/webm;codecs=vp8','video/webm'].find(t=>MediaRecorder.isTypeSupported(t));if(!mimeType)throw Error('WebM recording unavailable');
 const stream=canvas.captureStream(15),chunks=[];let started,abortReason=null,stopRequestedPerformanceMs=null;
 try{return await new Promise((resolve,reject)=>{
  const recorder=new MediaRecorder(stream,{mimeType,videoBitsPerSecond:durationMs>60000?750000:1500000});let timer;
  const stop=()=>{if(stopRequestedPerformanceMs!==null)return;stopRequestedPerformanceMs=performance.now();clearTimeout(timer);onDeadline();if(recorder.state!=='inactive')recorder.stop();};
  const abort=()=>{abortReason=String(signal.reason??'aborted');stop();};
  const cleanup=()=>{clearTimeout(timer);signal?.removeEventListener('abort',abort);};
  recorder.ondataavailable=e=>{if(e.data.size)chunks.push(e.data);};
  recorder.onerror=e=>{cleanup();reject(Error(e.error?.message??'Motion recorder failed'));};
  recorder.onstop=()=>{cleanup();const blob=new Blob(chunks,{type:mimeType});if(!blob.size)reject(Error('Motion recorder produced no frames'));else resolve({blob,mimeType,elapsedMs:performance.now()-started,stopRequestedPerformanceMs,stoppedPerformanceMs:performance.now(),requestedFps:15,aborted:abortReason!==null,abortReason});};
  recorder.start(1000);started=performance.now();onStart();signal?.addEventListener('abort',abort,{once:true});timer=setTimeout(stop,durationMs);if(signal?.aborted)abort();
 });}finally{for(const track of stream.getTracks())track.stop();}
}
