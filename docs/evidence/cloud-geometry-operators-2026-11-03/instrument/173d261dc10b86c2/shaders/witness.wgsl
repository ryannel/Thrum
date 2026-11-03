// Requested proof only. Approximate trajectories through each frozen source-step velocity,
// annotated by actual updated q. They are diagnostics, not a second material authority.
struct Witness {position:vec4f,record:vec4f,stats:vec4f};
struct PathSample {positionQ:vec4f,timeIce:vec4f};
@group(0) @binding(15) var<storage,read_write> witnesses:array<Witness,32>;
@group(0) @binding(16) var<storage,read_write> paths:array<PathSample>;
@group(0) @binding(17) var<storage,read_write> totals:array<atomic<u32>,8>;
@compute @workgroup_size(32) fn main(@builtin(local_invocation_id) id:vec3u){
  if(id.x==0u){
    for(var j=0u;j<32u;j++){
      let emitter=emitters[j];let age=p.weather.x-emitter.timing.x;
      if(emitter.location.w<1.5||age<8.||age>=8.+p.weather.y){continue;}
      var pair=0u;var oldest=1e20;
      for(var k=0u;k<16u;k++){if(witnesses[k*2u].record.y==0.){pair=k;break;}
        if(witnesses[k*2u].record.x<oldest){oldest=witnesses[k*2u].record.x;pair=k;}}
      for(var side=0u;side<2u;side++){
        let slot=pair*2u+side;let point=vec3f(p.weather.w+select(-80.,80.,side==1u),1320.,p.weather.x*1.1);
        let q=qAt(oldQ,point);let total=q.x+q.y;let admitted=total>.01&&q.w>1e-8;
        witnesses[slot]=Witness(vec4f(point,p.weather.x),vec4f(emitter.timing.x,select(3.,1.,admitted),0,0),vec4f(total,total,select(0.,point.y,total>.02),0));
        atomicAdd(&totals[0],1u);if(!admitted){atomicAdd(&totals[1],1u);}
      }
    }
  }
  storageBarrier();
  let slot=id.x;var w=witnesses[slot];if(w.record.y==0.){return;}
  var justFinished=false;
  if(w.record.y==1.&&p.weather.x>w.position.w+1e-5){
    let first=velocity(w.position.xyz);let middle=w.position.xyz+first*p.weather.y*.5;
    let speed=velocity(middle);let internal=length(speed-sharedWind(middle.y,1u));
    w.position=vec4f(w.position.xyz+speed*p.weather.y,w.position.w);
    let q=qAt(oldQ,w.position.xyz);let total=q.x+q.y;
    let wasCap=w.stats.z>=9000.;w.stats.x=min(w.stats.x,total);w.stats.y=max(w.stats.y,total);
    if(total>.02){w.stats.z=max(w.stats.z,w.position.y);}
    w.stats.w=max(w.stats.w,internal);atomicMax(&totals[7],u32(round(internal*100.)));
    if(!wasCap&&w.stats.z>=9000.){atomicAdd(&totals[2],1u);}
    if(internal>=99.99){w.record.w+=1.;atomicAdd(&totals[6],1u);}
    let outside=any(coords(w.position.xyz)<vec3f(0))||any(coords(w.position.xyz)>vec3f(1));
    if(outside||total<.001||p.weather.x-w.position.w>=420.){
      w.record.y=2.;justFinished=true;
      if(outside){atomicAdd(&totals[3],1u);}else if(total<.001){atomicAdd(&totals[4],1u);}else{atomicAdd(&totals[5],1u);}
    }
  }
  if((w.record.y==1.||w.record.z==0.||justFinished)&&w.record.z<512.&&(u32(p.lifecycle.x)%4u==0u||w.record.z==0.||justFinished)){
    let q=qAt(oldQ,w.position.xyz);let total=q.x+q.y;
    paths[slot*512u+u32(w.record.z)]=PathSample(vec4f(w.position.xyz,total),vec4f(p.weather.x,q.y/max(total,1e-12),w.record.y,w.record.w));w.record.z+=1.;
  }
  witnesses[slot]=w;
}
