/** One source-time authority for paused, skipped and accelerated preview frames. */
export class PreviewClock {
  constructor(duration, time=0) {
    if (!(duration>0) || !Number.isFinite(duration) || !Number.isFinite(time)) throw new RangeError('Finite positive duration required');
    this.duration=duration;this.time=Math.max(0,Math.min(duration,time));
    this.rate=1;this.playing=false;this.anchorTime=0;this.anchorSource=this.time;
  }
  sample(now) {
    if (!Number.isFinite(now)) throw new TypeError('Finite wall time required');
    return this.playing?Math.min(this.duration,this.anchorSource+Math.max(0,now-this.anchorTime)*this.rate/1000):this.time;
  }
  play(now) {
    this.anchorSource=this.sample(now);this.anchorTime=now;this.playing=true;
    return this.anchorSource;
  }
  pause(now) {this.time=this.sample(now);this.playing=false;return this.time;}
  seek(time,now) {
    if(!Number.isFinite(time)||!Number.isFinite(now))throw new TypeError('Finite source and wall time required');
    this.time=Math.max(0,Math.min(this.duration,time));this.anchorSource=this.time;this.anchorTime=now;
    return this.time;
  }
  setRate(rate,now) {
    if(!(rate>0)||!Number.isFinite(rate))throw new RangeError('Finite positive rate required');
    const time=this.sample(now);this.time=time;this.anchorSource=time;this.anchorTime=now;this.rate=rate;
    return time;
  }
}
