// Failure stays terminal for this one-shot lab; late callers cannot wait forever.
export class FrameSettlement {
 constructor(){this.error=null;this.waiters=[];}
 wait(pending){if(this.error)return Promise.reject(this.error);if(!pending)return Promise.resolve();return new Promise((resolve,reject)=>this.waiters.push({resolve,reject}));}
 fail(error){this.error??=error;for(const w of this.waiters.splice(0))w.reject(this.error);}
 complete(){for(const w of this.waiters.splice(0))this.error?w.reject(this.error):w.resolve();}
}
export function assertGpuHealthy(gpu){if(gpu.lost)throw Error('Geometry GPU device lost');if(gpu.errors.length)throw Error(gpu.errors.join('; '));}
const deviceWatches=new WeakMap();
function lossWatch(device){
 let watch=deviceWatches.get(device);if(watch)return watch;
 watch={error:null,waiters:new Set()};deviceWatches.set(device,watch);
 // One device listener, with only current fences subscribed (no per-frame pending promise leak).
 device.lost.then(info=>{watch.error=Error('Geometry GPU device lost: '+(info?.message??'unknown reason'));for(const reject of watch.waiters)reject(watch.error);watch.waiters.clear();});return watch;
}
export async function frameFence(gpu){
 assertGpuHealthy(gpu);const watch=lossWatch(gpu.device);if(watch.error)throw watch.error;
 // A late fence is ordinary backpressure. Device loss rejects even if that fence never settles.
 await new Promise((resolve,reject)=>{const lost=error=>reject(error);watch.waiters.add(lost);gpu.device.queue.onSubmittedWorkDone().then(()=>{watch.waiters.delete(lost);resolve();},error=>{watch.waiters.delete(lost);reject(error);});});
 assertGpuHealthy(gpu);if(watch.error)throw watch.error;
}
