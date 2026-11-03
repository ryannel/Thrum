export class FeedWitness {
  static async create(scene){
    const w=new FeedWitness(),g=scene.gpu;
    w.running=true;w.time=0;
    w.pipeline=await g.compute('requested supplied-material trajectory witness',scene.codes.source.replace('fn main(','fn sourceStep(')+scene.codes.witness);
    w.state=g.buffer('witness state',32*48,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC|GPUBufferUsage.COPY_DST);
    w.paths=g.buffer('witness paths',32*512*32,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC|GPUBufferUsage.COPY_DST);
    w.totals=g.buffer('witness totals',32,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC|GPUBufferUsage.COPY_DST);return w;
  }
  reset(encoder,time){this.running=true;this.time=time;encoder.clearBuffer(this.state);encoder.clearBuffer(this.paths);encoder.clearBuffer(this.totals);}
  step(scene,encoder,uniform,q,emit){
    if(!this.running)return;this.time=scene.time+.25;
    const thermal=scene.thermalBindings.get(uniform)??{heads:scene.zeroHeads,bands:scene.zeroBands};
    const bg=scene.cachedGroup('witness/'+uniform.label+'/'+q.label+'/'+thermal.heads.label,this.pipeline,()=>[
      [0,uniform],[5,scene.noise.createView()],[1,q.createView()],[6,scene.clamp],[8,emit],[9,scene.curl.createView()],[10,scene.repeat],[13,thermal.heads],[14,thermal.bands],[15,this.state],[16,this.paths],[17,this.totals],
    ]);
    const pass=encoder.beginComputePass();pass.setPipeline(this.pipeline);pass.setBindGroup(0,bg);pass.dispatchWorkgroups(1);pass.end();
  }
  async observe(scene){
    const g=scene.gpu,stateBytes=32*48,pathBytes=32*512*32,totalBytes=stateBytes+pathBytes+32;
    this.read??=g.buffer('witness requested readback',totalBytes,GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST);
    const e=g.device.createCommandEncoder();e.copyBufferToBuffer(this.state,0,this.read,0,stateBytes);e.copyBufferToBuffer(this.paths,0,this.read,stateBytes,pathBytes);e.copyBufferToBuffer(this.totals,0,this.read,stateBytes+pathBytes,32);g.submit(e);
    await this.read.mapAsync(GPUMapMode.READ);const bytes=this.read.getMappedRange().slice(0);this.read.unmap();
    const state=new Float32Array(bytes,0,stateBytes/4),paths=new Float32Array(bytes,stateBytes,pathBytes/4),totals=Array.from(new Uint32Array(bytes,stateBytes+pathBytes,8));
    const tracks=[];
    for(let i=0;i<32;i++){
      const s=state.subarray(i*12,i*12+12);if(s[5]===0)continue;
      const count=Math.min(512,Math.round(s[6])),samples=[];
      for(let j=0;j<count;j++)samples.push(Array.from(paths.subarray((i*512+j)*8,(i*512+j+1)*8)));
      const head=scene.thermalDriver.heads.find(h=>Math.abs(h.birth-s[4])<1e-4);
      tracks.push({slot:i,sourceBirthTime:s[4],sourceHeadId:head?.id??null,seedTime:s[3],state:['unused','active','terminated','no-birth-at-seed'][Math.round(s[5])],position:Array.from(s.subarray(0,3)),minQ:s[8],maxQ:s[9],maxHeightWithQAbovePoint02:s[10],maxInternalSpeed:s[11],speedCapSteps:s[7],samples});
    }
    return {scope:'Requested proof-only RK2 paths through frozen per-tick composed velocity, sampled against updated raw q; admission at head-cohort age8s needs positive reaction-birth proxy and q>.01. Not exact parcel lineage or optical visibility.32tracks,2/cohort,512samples/track,1Hz plus endpoints; old cohorts overwritten after16new cohorts.',sourceTime:this.time,running:this.running,sampleLayout:['x','y','z','qLiquid+qIce','sourceTime','iceFraction','state(1active/2terminated/3no-birth)','speedCapSteps'],totals:{seedAttempts:totals[0],noBirthAtSeed:totals[1],capArrivalsWithQAbovePoint02:totals[2],boundaryTerminations:totals[3],lowQTerminations:totals[4],ageTerminations:totals[5],speedCapSteps:totals[6],maxInternalSpeed:totals[7]/100},tracks};
  }
}
