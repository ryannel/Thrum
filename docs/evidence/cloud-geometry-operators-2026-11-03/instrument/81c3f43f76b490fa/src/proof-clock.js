// One analytic source clock. GPU completion never advances or arrests it.
export class ProofWorldClock {
 constructor(){this.time=0;this.rate=1;this.playing=false;this.epoch=0;this.lastWall=null;}
 advance(now){if(!Number.isFinite(now)||(this.lastWall!==null&&now<this.lastWall))throw Error('Nonmonotone proof wall clock');if(this.playing&&this.lastWall!==null)this.time+=(now-this.lastWall)*.001*this.rate;this.lastWall=now;return this.snapshot();}
 play(now){this.advance(now);this.playing=true;return this.snapshot();}
 pause(now){this.advance(now);this.playing=false;return this.snapshot();}
 setRate(rate,now){if(![1,16].includes(rate))throw Error('Unsupported proof rate');this.advance(now);this.rate=rate;return this.snapshot();}
 reset(now){this.time=0;this.playing=false;this.lastWall=now;this.epoch++;return this.snapshot();}
 snapshot(){const phase=Math.min(120,this.time);return {worldTime:this.time,trajectoryPhaseSeconds:phase,exposureWorldM:100*phase/120,trajectoryClamped:this.time>120,rate:this.rate,playing:this.playing,epoch:this.epoch};}
}
export function motionOutcome(start,end){
 if(start.revision!==end.revision)return 'fixture-changed';
 if(start.epoch!==end.epoch)return 'epoch-changed';
 if(end.fault||end.gpuErrors)return 'faulted';
 if(!(end.sourceTime>start.sourceTime&&end.imageFrame>start.imageFrame))return 'no-source-or-image-progress';
 return 'fixed-fixture';
}
