// Requested-only profile4 differential. It owns and retires all mutable GPU fixtures.
export async function compareFrontMask(scene) {
  if (scene.family !== 1 || !scene.sourcePipeline || !scene.mixPipeline ||
      !scene.thermalDriver?.frontMorph || !scene.thermalDriver.connectedFeed)
    throw Error("Cb mask comparison requires the profile4 connected-feed volume");
  const g = scene.gpu, d = g.device, dims = [...scene.dims];
  const epoch = scene.epoch, sourceTime = scene.time, current = scene.current;
  const heads = scene.thermalDriver.data.slice(), bands = scene.thermalDriver.bands.slice();
  const weather = { ...scene.lastSourceInput }, bounds = scene.bounds().map(v => [...v]);
  const allocations = [], cpuBytes = heads.byteLength + bands.byteLength;
  let logicalBytes = 0;
  const texture = label => {
    const t = d.createTexture({ label: `front mask/${label}`, size: dims, dimension: "3d", format: "rgba16float",
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.STORAGE_BINDING | GPUTextureUsage.COPY_SRC | GPUTextureUsage.COPY_DST });
    allocations.push(t); logicalBytes += dims.reduce((a,b) => a*b,1)*8; return t;
  };
  const buffer = (label, size, usage) => {
    const b = d.createBuffer({label:`front mask/${label}`,size,usage});
    allocations.push(b);logicalBytes += size;return b;
  };
  const unchanged = () => {
    if (scene.epoch !== epoch || scene.time !== sourceTime || scene.current !== current || g.lost)
      throw Error("Profile4 mask fixture changed or device became unavailable");
  };
  const compute = (e,pipeline,group,size=dims,local=4) => {
    const p=e.beginComputePass();p.setPipeline(pipeline);p.setBindGroup(0,group);
    p.dispatchWorkgroups(...size.map(v=>Math.ceil(v/local)));p.end();
  };
  const submit = async e => { g.submit(e);await d.queue.onSubmittedWorkDone();unchanged(); };
  let result;
  try {
    await d.queue.onSubmittedWorkDone(); unchanged();
    const old=texture("common input"),dense=texture("dense"),masked=texture("masked"),fwd=texture("forward"),rev=texture("reverse/normal scratch");
    const bricks=dims.map(v=>Math.ceil(v/8)), brickCount=bricks.reduce((a,b)=>a*b,1);
    const storage=GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST;
    const support=buffer("prospective support",brickCount*4,storage),active=buffer("guarded mask",brickCount*4,storage);
    const headBuffer=buffer("frozen head snapshot",heads.byteLength,storage),bandBuffer=buffer("frozen band snapshot",bands.byteLength,storage);
    d.queue.writeBuffer(headBuffer,0,heads);d.queue.writeBuffer(bandBuffer,0,bands);
    const emit=buffer("finite emitter",1024,storage);
    const uniforms=Array.from({length:6},(_,i)=>buffer(`immutable step slot ${i}`,176,GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST));
    const counters=buffer("differential counters",48,storage|GPUBufferUsage.COPY_SRC);
    const read=buffer("readback",48,GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST);
    const init=await g.compute("profile4 mask clear/brick-edge seed",`
      @group(0) @binding(0) var output:texture_storage_3d<rgba16float,write>;
      @group(0) @binding(1) var<uniform> params:vec4u;
      @compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){
        let n=textureDimensions(output);if(any(id>=n)){return;}var q=vec4f(0);
        if(params.x==1u&&(id.x==7u||id.x==8u)&&(id.y==4u||id.y==5u)&&(id.z==n.z/2u||id.z+1u==n.z/2u)){q=vec4f(.2,.05,17.,0.);}
        textureStore(output,id,q);
      }`);
    const initUniform=buffer("initial fixture selector",16,GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST);
    const initGroup=g.group(init,[[0,old.createView()],[1,initUniform]]);
    const compare=await g.compute("profile4 all-channel whole-field differential",`
      @group(0) @binding(0) var a:texture_3d<f32>;
      @group(0) @binding(1) var b:texture_3d<f32>;
      @group(0) @binding(2) var<storage,read_write> counts:array<atomic<u32>,12>;
      @group(0) @binding(3) var<storage,read> mask:array<u32>;
      @group(0) @binding(4) var previous:texture_3d<f32>;
      @compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){
        let n=textureDimensions(a);if(any(id>=n)){return;}
        let av=textureLoad(a,vec3i(id),0);let bv=textureLoad(b,vec3i(id),0);
        if(any(av!=bv)){atomicAdd(&counts[0],1u);}
        if(any(av!=av)||any(bv!=bv)||any(abs(av)>vec4f(65504))||any(abs(bv)>vec4f(65504))||any(av.xy<vec2f(0))||any(bv.xy<vec2f(0))||any(av.xy>vec2f(1))||any(bv.xy>vec2f(1))||av.z<0.||bv.z<0.||av.w<0.||bv.w<0.){atomicAdd(&counts[1],1u);}
        let delta=abs(av-bv);atomicMax(&counts[2],bitcast<u32>(max(max(delta.x,delta.y),max(delta.z,delta.w))));
        let flat=(id.z*n.y+id.y)*n.x+id.x;if(flat<arrayLength(&mask)){atomicAdd(&counts[3],mask[flat]);}
        if(av.x+av.y>0.){atomicAdd(&counts[4],1u);if(id.x==0u||id.y==0u||id.z==0u||id.x+1u==n.x||id.y+1u==n.y||id.z+1u==n.z){atomicAdd(&counts[6],1u);}}
        if(av.w>0.){atomicAdd(&counts[5],1u);}
        let prev=textureLoad(previous,vec3i(id),0);if(prev.x+prev.y>0.){atomicAdd(&counts[7],1u);}
        for(var channel=0u;channel<4u;channel++){if(av[channel]!=bv[channel]){atomicAdd(&counts[8u+channel],1u);}}
      }`);
    const compareGroup=g.group(compare,[[0,masked.createView()],[1,dense.createView()],[2,counters],[3,active],[4,old.createView()]]);
    const rest=scene.rest[scene.currentRest];
    const sourceGroups=uniforms.map((u,i)=>{
      const stage=i%3+1,out=stage===1?fwd:stage===2?rev:i<3?dense:masked;
      return g.group(scene.sourcePipeline,[[0,u],[1,old.createView()],[2,(stage===1?old:fwd).createView()],
        [3,(stage===3?rev:old).createView()],[4,out.createView()],[5,scene.noise.createView()],[6,scene.clamp],
        [7,rest.createView()],[8,emit],[9,scene.curl.createView()],[10,scene.repeat],[11,active],[12,scene.velocityField],
        [13,headBuffer],[14,bandBuffer]]);
    });
    const supportGroup=g.group(scene.supportPipeline,[[0,uniforms[3]],[1,old.createView()],[2,emit],[3,support]]);
    const dilationGroup=g.group(scene.dilatePipeline,[[0,old.createView()],[1,support],[2,active]]);
    const mixGroups=[dense,masked].map((input,i)=>g.group(scene.mixPipeline,[[0,uniforms[i*3+2]],[1,input.createView()],
      [2,rev.createView()],[3,scene.noise.createView()],[4,scene.repeat],[5,headBuffer],[6,bandBuffer]]));
    const records=[],forcing=[];
    for(const fixture of ["clear-birth","existing-q","brick-edge-and-chart-boundary-birth"]){
      const setup=d.createCommandEncoder();
      if(fixture==="existing-q")setup.copyTextureToTexture({texture:scene.q[current]},{texture:old},dims);
      else {d.queue.writeBuffer(initUniform,0,new Uint32Array([fixture.startsWith("brick")?1:0,0,0,0]));compute(setup,init,initGroup);}
      await submit(setup);
      let windIntegral=weather.windIntegral;
      for(let step=0;step<12;step++){
        unchanged();
        const wind=step<4?3:-12,lift=step<4?1:step<8?2:0,moist=step<8?.85:0,dry=step<8?.08:.7;
        const time=sourceTime+step*.25;
        const params=new Float32Array(44);
        params.set([...bounds[0],.0045],12);params.set([...bounds[1],.0018],16);
        params.set([time,.25,wind,windIntegral],20);params.set([moist,dry,lift,1],24);
        params.set([step<4?1e6:windIntegral+300,1,4,1],28);
        params.set([dims[0],.25,0,1],36);params.set([scene.steps+step+1,1,256,0],40);
        const pulse=new Float32Array(256);
        const edge=fixture.startsWith("brick"), desiredX=edge?bounds[0][0]+600:windIntegral;
        // Deliberate boundary stress uses a finite kernel outside the ordinary authoring catchment.
        pulse.set([(desiredX-windIntegral)/2,0,600,2,sourceTime-8,4,8,10]);d.queue.writeBuffer(emit,0,pulse);
        for(let variant=0;variant<2;variant++)for(let stage=1;stage<=3;stage++){
          params[3]=0;params[38]=variant;params[39]=stage;
          d.queue.writeBuffer(uniforms[variant*3+stage-1],0,params);
        }
        const e=d.createCommandEncoder();e.clearBuffer(counters);
        compute(e,scene.supportPipeline,supportGroup,bricks,1);
        compute(e,scene.dilatePipeline,dilationGroup,bricks);
        for(let variant=0;variant<2;variant++){
          for(let stage=0;stage<3;stage++)compute(e,scene.sourcePipeline,sourceGroups[variant*3+stage]);
          compute(e,scene.mixPipeline,mixGroups[variant]);
          e.copyTextureToTexture({texture:rev},{texture:variant===0?dense:masked},dims);
        }
        compute(e,compare,compareGroup);e.copyBufferToBuffer(counters,0,read,0,48);
        e.copyTextureToTexture({texture:masked},{texture:old},dims);
        await submit(e);await read.mapAsync(GPUMapMode.READ);
        const a=new Uint32Array(read.getMappedRange().slice(0));read.unmap();
        records.push({fixture,step,sourceTime:time,differentVoxels:a[0],invalidVoxels:a[1],
          maxAbsDifference:new Float32Array(new Uint32Array([a[2]]).buffer)[0],activeBricks:a[3],totalBricks:brickCount,
          occupiedVoxels:a[4],sourceTaggedVoxels:a[5],boundaryOccupiedVoxels:a[6],inputOccupiedVoxels:a[7],
          differentChannelsRGBA:Array.from(a.slice(8,12))});
        if(fixture==="clear-birth")forcing.push({step,wind,lift,moist,dry,windIntegral});
        windIntegral+=wind*.25;
      }
    }
    const first=fixture=>records.find(r=>r.fixture===fixture&&r.step===0);
    const witnesses={clearBirth:first("clear-birth").inputOccupiedVoxels===0&&first("clear-birth").sourceTaggedVoxels>0,
      existingInput:first("existing-q").inputOccupiedVoxels>0,
      boundaryBirth:records.some(r=>r.fixture.startsWith("brick")&&r.boundaryOccupiedVoxels>0&&r.sourceTaggedVoxels>0),
      genuineSkipping:records.every(r=>r.activeBricks>0&&r.activeBricks<r.totalBricks)};
    result={epoch,sourceTime,profile:4,pass:Object.values(witnesses).every(Boolean)&&records.every(r=>r.differentVoxels===0&&r.invalidVoxels===0)&&g.errors.length===0,
      scope:"36 sequential full-resolution .25s q ticks: clear birth, current-q copy and brick-edge/chart-boundary finite-kernel birth; frozen actual head/band snapshot and prepared basis; both routes direct flow, source stages plus actual profile4 normal/mix pass; exact decoded RGBA16F all4channels; independent mask resources; changed wind/lift then shutdown. No carrier update/equivalence, head-motion replay or cache equivalence.",
      witnesses,records,forcing,errors:[...g.errors],oracleLogicalBytes:logicalBytes,oracleCpuSnapshotBytes:cpuBytes,
      resourcesRetired:false,frozenHeadCount:scene.thermalDriver.heads.length,headSnapshotBytes:heads.byteLength,bandSnapshotBytes:bands.byteLength};
  } finally {
    // Error paths drain before retirement too; the caller's scene resources remain untouched.
    try { await d.queue.onSubmittedWorkDone(); } catch (_) { /* Device loss still permits destroy. */ }
    for(const resource of allocations)resource.destroy();
    if(result)result.resourcesRetired=true;
  }
  return result;
}
