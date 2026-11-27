#!/usr/bin/env node
// The documented scene command, hardware Chrome, and the actual submitted field/output taps.
import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn, execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import zlib from 'node:zlib';
import {installWasmReceipt,readWasmReceipt} from './wasm_receipt.mjs';

const mode = process.argv[process.argv.indexOf('--case') + 1];
if (!['first-frame','frames','fixed','realtime','island','bodies','wake','foam','surf','channel','river','river-calm','rain-puddles','rain-rings','rain-footfall','rain-ground','under'].includes(mode)) throw Error('use --case first-frame|frames|fixed|realtime|island|bodies|wake|foam|surf|channel|river|river-calm|rain-puddles|rain-rings|rain-footfall|rain-ground|under');
const opticsRoute = mode === 'fixed' ? process.env.THRUM_WATER_BROWSER_OPTICS : undefined;
const diveRoute = ['fixed','realtime'].includes(mode) ? process.env.THRUM_WATER_BROWSER_DIVE : undefined;
const surfInspection = process.env.THRUM_WATER_INSPECT_SURF === '1' && ['fixed','realtime'].includes(mode);
const handoffCut = surfInspection && mode === 'realtime' && process.env.THRUM_WATER_ROUTE === 'handoff-cut';
const [width,height] = mode === 'under' ? [48,27]
  : mode === 'realtime' && diveRoute ? [1280,720]
  : ['rain-rings','rain-footfall','rain-ground'].includes(mode) ? [960,540]
  : [2560,1440];
const output = path.resolve(process.env.THRUM_WATER_OUTPUT ?? `tmp/waters-M3/${mode}-${Date.now()}`);
await fs.mkdir(output,{recursive:true});
const delay = ms => new Promise(r=>setTimeout(r,ms));
const assert = (ok,text) => {if(!ok) throw Error(text);};
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const lensRoute = (fixture, arm) => `?lens=${fixture}&arm=${arm}`;
const lensObserver = `
  globalThis.__lensPasses={gather:0,cut:0,composition:0};
  let lensGame;
  Object.defineProperty(window,'game',{
    configurable:true,
    get:()=>lensGame,
    set(game){
      lensGame=game;
      const setVisible=game.set_visible.bind(game);
      game.set_visible=(visible,timestamp)=>{
        globalThis.__lensClockOrigin??=timestamp;
        return setVisible(visible,timestamp);
      };
    },
  });
  const watchLensPipeline=(prototype, label, kind)=>{
    const setPipeline=prototype.setPipeline;
    prototype.setPipeline=function(pipeline){
      if(pipeline.label===label)__lensPasses[kind]++;
      return setPipeline.call(this,pipeline);
    };
  };
  watchLensPipeline(GPUComputePassEncoder.prototype,'water gather','gather');
  watchLensPipeline(GPURenderPassEncoder.prototype,'water through lens','composition');
  watchLensPipeline(GPUComputePassEncoder.prototype,'water lens cut','cut');
`;
function riverLook(rgba, station) {
  // Windows stay within the submitted water for the three fixed river lenses.
  const windows={
    riffle:[[0.53,0.64,0.50,0.55]],
    pool:[[0.56,0.64,0.66,0.78]],
    ledge:[[0.24,0.40,0.47,0.60],[0.62,0.75,0.66,0.76]],
  };
  return windows[station].map(([x0,x1,y0,y1])=>{
    let edge=0,bright=0,count=0;
    for(let y=Math.floor(y0*height);y<y1*height;y+=2){
      for(let x=Math.floor(x0*width);x<x1*width;x+=2){
        const at=(y*width+x)*4;
        const [r,g,b]=rgba.subarray(at,at+3);
        const delta=(r+g+b-rgba[at+8]-rgba[at+9]-rgba[at+10])/3;
        edge+=delta*delta;
        if(Math.min(r,g,b)>145&&Math.max(r,g,b)-Math.min(r,g,b)<55)bright++;
        count++;
      }
    }
    return {edge_rms:Math.sqrt(edge/count),bright_cover:bright/count};
  });
}
const frameInfo = text => Object.fromEntries(
  text.split(' ').map(pair => pair.split('=')).map(([key, value]) => [key, Number(value)]),
);
async function until(fn,label,timeout=180000) {
  const end=Date.now()+timeout;
  while(Date.now()<end) {const v=await fn(); if(v)return v; await delay(50);}
  throw Error(`Timed out: ${label}`);
}
class Cdp {
  constructor(socket) {
    this.socket=socket; this.next=0; this.pending=new Map(); this.faults=[];
    socket.onmessage=({data})=>{
      const p=JSON.parse(data), pending=this.pending.get(p.id);
      if(pending) {this.pending.delete(p.id);clearTimeout(pending.timer);p.error?pending.reject(Error(JSON.stringify(p.error))):pending.resolve(p.result);}
      if(p.method==='Runtime.exceptionThrown')this.faults.push(p.params);
      if(p.method==='Runtime.consoleAPICalled'&&p.params.type==='error')this.faults.push(p.params.args.map(a=>a.value??a.description).join(' '));
    };
    socket.onclose=event=>{for(const p of this.pending.values()){clearTimeout(p.timer);p.reject(Error(`CDP closed ${event.code}: ${event.reason}`));}this.pending.clear();};
  }
  call(method,params={},sessionId) {
    return new Promise((resolve,reject)=>{
      const id=++this.next, timer=setTimeout(()=>reject(Error(`CDP timeout ${method}`)),120000);
      this.pending.set(id,{resolve,reject,timer});this.socket.send(JSON.stringify({id,method,params,...(sessionId?{sessionId}:{})}));
    });
  }
}
function png(rgba,width,height) {
  const chunk=(name,data)=>{
    const body=Buffer.concat([Buffer.from(name),data]);let crc=~0;
    for(const b of body){crc^=b;for(let k=0;k<8;k++)crc=crc&1?(crc>>>1)^0xedb88320:crc>>>1;}
    const out=Buffer.alloc(body.length+8);out.writeUInt32BE(data.length);body.copy(out,4);out.writeUInt32BE(~crc>>>0,body.length+4);return out;
  };
  const header=Buffer.alloc(13);header.writeUInt32BE(width);header.writeUInt32BE(height,4);header[8]=8;header[9]=6;
  const rows=Buffer.concat(Array.from({length:height},(_,y)=>Buffer.concat([Buffer.from([0]),rgba.subarray(y*width*4,(y+1)*width*4)])));
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),chunk('IDAT',zlib.deflateSync(rows)),chunk('IEND',Buffer.alloc(0))]);
}
// This observer only appends copies to the ordinary frame encoder. No field shader is replaced.
const observer = `
  globalThis.__devices=[];globalThis.__buffers={};globalThis.__faults=[];
  // The inspection supplies the page's frame opportunities from one RAF loop.
  globalThis.__requestCaptureFrame=window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame=()=>-1;
  const request=GPUAdapter.prototype.requestDevice;
  GPUAdapter.prototype.requestDevice=async function(...args){
    const d=await request.apply(this,args);__devices.push(d);
    d.addEventListener('uncapturederror',e=>__faults.push(String(e.error.message)));
    return d;
  };
  const create=GPUDevice.prototype.createBuffer;
  GPUDevice.prototype.createBuffer=function(desc){
    if(['sea parameters','interaction sample settings','foam settings','surf field settings'].includes(desc.label))desc={...desc,usage:desc.usage|GPUBufferUsage.COPY_SRC};
    const b=create.call(this,desc);
    if(desc.label?.startsWith('sea ')||desc.label?.startsWith('interaction ')||desc.label?.startsWith('foam ')||['surf field cache','surf field settings','water surface vertices'].includes(desc.label))__buffers[desc.label]=b;
    return b;
  };
  globalThis.__readFoam=async(points=[])=>{
    const device=__devices.at(-1), commands=device.createCommandEncoder();
    const stages=['foam settings','foam endpoints',...(points.length?['foam transport']:[])].map(name=>{
      const source=__buffers[name];
      if(!source)throw Error('missing submitted '+name);
      const stage=device.createBuffer({size:source.size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
      commands.copyBufferToBuffer(source,0,stage,0,source.size);
      return stage;
    });
    device.queue.submit([commands.finish()]);
    const [settings,state,transport]=await Promise.all(stages.map(async stage=>{
      await stage.mapAsync(GPUMapMode.READ);
      const bytes=stage.getMappedRange().slice(0);
      stage.unmap();stage.destroy();
      return bytes;
    }));
    const bank=new Uint32Array(settings,64,4)[1];
    const cells=new Float32Array(state);
    const count=cells.length/16;
    let cover=0,peak=0,filmCells=0;
    for(let i=0;i<count;i++){
      const at=4*(2*bank*count+i);
      const film=cells[at+1]+cells[at+2];
      cover+=film;
      peak=Math.max(peak,film);
      filmCells+=Number(film>0.01);
    }
    const dt=new Float32Array(settings,80,1)[0];
    const parcels = dt>0 ? points.map(([x,z]) => {
      const [ox,oz] = new Int32Array(settings,0,2);
      const ix=Math.floor(x/0.25)-ox, iz=Math.floor(z/0.25)-oz;
      if(ix<0||ix>=256||iz<0||iz>=256)throw Error('point outside submitted fine foam window');
      const state=new Float32Array(transport,(iz*256+ix)*32,8);
      return {point:[x,z],speed:[state[4]*0.25/dt,state[5]*0.25/dt],film:state[1]+state[2]};
    }) : [];
    return {mean:cover/count,peak,filmCells,parcels};
  };
  const wakeWitnesses=new Map();
  globalThis.__readWake=async(tick,points=[],hull=null,footfall=null)=>{
    const device=__devices.at(-1), commands=device.createCommandEncoder();
    const stages=['interaction sample settings','interaction wave endpoints'].map(name=>{
      const source=__buffers[name];
      if(!source)throw Error('missing submitted '+name);
      const buffer=device.createBuffer({size:source.size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
      commands.copyBufferToBuffer(source,0,buffer,0,source.size);return buffer;
    });
    device.queue.submit([commands.finish()]);
    const [settings,state]=await Promise.all(stages.map(async buffer=>{
      await buffer.mapAsync(GPUMapMode.READ);
      const bytes=buffer.getMappedRange().slice(0);buffer.unmap();buffer.destroy();return bytes;
    }));
    const rows=new Float32Array(settings),bases=new Uint32Array(settings,80,4);
    const cells=new Float32Array(state);
    const interpolate=([x,z])=>[2,3].reduce((sum,plane)=>{
      const cx=(x-rows[plane*4])/rows[plane*4+2],cz=(z-rows[plane*4+1])/rows[plane*4+2];
      if(cx<0||cz<0||cx>=255||cz>=255)return sum;
      const i=2*(bases[plane]+Math.floor(cz)*256+Math.floor(cx));
      const fx=cx%1,fz=cz%1;
      const a=cells[i]+fx*(cells[i+2]-cells[i]);
      const b=cells[i+512]+fx*(cells[i+514]-cells[i+512]);
      return sum+a+fz*(b-a);
    },0);
    wakeWitnesses.set(tick,interpolate);
    while(wakeWitnesses.size>4)wakeWitnesses.delete(wakeWitnesses.keys().next().value);
    let radialMoment=0,nearEnergy=0,outsideHull=0;
    const peak=[2,3].map(plane=>{
      let value=0;
      for(let cell=0;cell<256*256;cell++){
        const height=cells[2*(bases[plane]+cell)];
        value=Math.max(value,Math.abs(height));
        if(plane===2&&hull){
          const x=rows[8]+(cell%256)*rows[10]-hull[0];
          const z=rows[9]+Math.floor(cell/256)*rows[10]-hull[1];
          if(x*x+z*z>36)outsideHull=Math.max(outsideHull,Math.abs(height));
        }
        if(plane===2&&points.length){
          const x=rows[8]+(cell%256)*rows[10]-points[0][0];
          const z=rows[9]+Math.floor(cell/256)*rows[10]-points[0][1];
          radialMoment+=(x*x+z*z)*height*height;
          nearEnergy+=height*height;
        }
      }
      return value;
    });
    let fine=null;
    if(footfall){
      const [x,z,sill]=footfall;
      const [ox,oz,spacing]=new Float32Array(settings,112,4);
      const base=new Uint32Array(settings,144,4)[1];
      const at=([x,z])=>[Math.round((x-ox)/spacing),Math.round((z-oz)/spacing)];
      const [right]=at([149.35,66.35]);
      if(!(right>0&&right<255))throw Error('receiving puddle is outside the fine witness window');
      if(new Float32Array(settings,128,4)[2]!==1)throw Error('no submitted fine level');
      let source=0,front=0,beyond=0,beyondPotential=0;
      for(let index=0;index<256*256;index++){
        const px=ox+(index%256)*spacing,pz=oz+Math.floor(index/256)*spacing;
        const height=Math.abs(cells[2*(base+index)]),potential=Math.abs(cells[2*(base+index)+1]);
        const radius=Math.hypot(px-x,pz-z);
        if(radius<0.12)source=Math.max(source,height);
        if(radius>0.22&&radius<0.43)front=Math.max(front,height);
        if(px>sill){
          beyond=Math.max(beyond,height);
          beyondPotential=Math.max(beyondPotential,potential);
        }
      }
      fine={source,front,beyond,beyondPotential,origin:[ox,oz],spacing,right};
    }
    return {tick,peak,heights:points.map(interpolate),outsideHull,radius:nearEnergy>0?Math.sqrt(radialMoment/nearEnergy):0,fine};
  };
  globalThis.__checkWakeSample=([tick,used,requested,x,z,height])=>{
    const sample=wakeWitnesses.get(tick);
    if(!sample||used!==requested)return null;
    return {tick,x,z,height,expected:sample([x,z])};
  };
  globalThis.__rainPasses=0;
  const begin=GPUCommandEncoder.prototype.beginComputePass;
  GPUCommandEncoder.prototype.beginComputePass=function(desc){
    if(desc?.label?.includes('sea long waves'))this.__sea=true;
    if(desc?.label?.endsWith('rain bands'))globalThis.__rainPasses++;
    return begin.call(this,desc);
  };
  const finish=GPUCommandEncoder.prototype.finish;
  GPUCommandEncoder.prototype.finish=function(...args){
    if(globalThis.__probeNext&&this.__sea){
      __probeNext=false;globalThis.__readbacks=[];
      for(const name of ['sea parameters','sea long field','sea cascades']){
        const src=__buffers[name],d=__devices.at(-1);
        const b=d.createBuffer({size:src.size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
        this.copyBufferToBuffer(src,0,b,0,src.size);__readbacks.push(b);
      }
    }
    return finish.apply(this,args);
  };
  globalThis.__readFields=async()=>{
    const result=[];
    for(const [i,b] of __readbacks.entries()){
      await b.mapAsync(GPUMapMode.READ);const a=new Float32Array(b.getMappedRange());
      if(i===0)result.push(Array.from(a));
      else if(i===1)result.push(Array.from({length:64},(_,j)=>{
        const level=j%8,pixel=(j*4051+137)%16384;
        return {level,pixel,value:Array.from(a.slice((4096+level*16384+pixel)*12,(4096+level*16384+pixel)*12+12))};
      }));
      else result.push(Array.from({length:3},(_,band)=>Array.from({length:32},(_,j)=>{
        const pixel=(j*4051+137)%65536;return Array.from(a.slice((band*65536+pixel)*12,(band*65536+pixel)*12+12));
      })));
      b.unmap();b.destroy();
    }
    return result;
  };
`;
const islandObserver = `
  globalThis.__island = {draws:[], layouts:new WeakMap(), pipelines:new WeakMap()};
  function wrap(proto,name,observe){
    const original=proto[name];
    proto[name]=function(...args){
      const result=original.apply(this,args);
      observe.call(this,result,...args);
      return result;
    };
  }
  wrap(GPUDevice.prototype,'createShaderModule',(shader,d)=>{
    if(d.label==='water surface')__island.source=d.code;
  });
  __island.surfTextures=[];
  __island.rainTextures=[];
  wrap(GPUDevice.prototype,'createTexture',(_,d)=>{
    if(['surf lags','surf exposures','breaker section','surf bed pyramid'].includes(d.label))__island.surfTextures.push(d);
    if(['sea filtered fields','rain canopy and depth response','rain bands'].includes(d.label))__island.rainTextures.push(d);
  });
  wrap(GPUDevice.prototype,'createComputePipeline',(_,d)=>{
    if(d.label==='surf field cache')__island.surfEntries=__island.pipelines.get(d.layout).flatMap(layout=>__island.layouts.get(layout)??[]);
  });
  wrap(GPUDevice.prototype,'createBindGroupLayout',(layout,d)=>{
    __island.layouts.set(layout,d.entries);
    if(d.label==='water field')__island.fieldLayout=d.entries;
  });
  wrap(GPUDevice.prototype,'createPipelineLayout',(layout,d)=>__island.pipelines.set(layout,d.bindGroupLayouts));
  wrap(GPUDevice.prototype,'createBindGroup',(group,d)=>{
    if(d.label==='water field')__island.bindings=d.entries;
  });
  wrap(GPUDevice.prototype,'createRenderPipeline',(pipeline,d)=>{
    const entries=()=>__island.pipelines.get(d.layout)?.flatMap(layout=>__island.layouts.get(layout)??[])??[];
    if(d.label==='prepared opaque synthetic role')__island.forwardEntries=entries();
    if(d.label==='shore film')__island.filmEntries=entries();
    if(d.label!=='water surface')return;
    __island.surface=pipeline;
    __island.surfaceEntries=__island.pipelines.get(d.layout).flatMap(layout=>__island.layouts.get(layout)??[]);
  });
  wrap(GPURenderPassEncoder.prototype,'setPipeline',function(_,pipeline){this.__water=pipeline===__island.surface;});
  wrap(GPURenderPassEncoder.prototype,'draw',function(_,count,instances,first){
    if(this.__water)__island.draws.push([first,count]);
  });
  __island.yardDraws=[];
  wrap(GPURenderPassEncoder.prototype,'setIndexBuffer',function(_,buffer){
    this.__yardIndices=buffer.label==='water hollows tile indices';
  });
  wrap(GPURenderPassEncoder.prototype,'drawIndexed',function(_,count,instances,first){
    if(this.__water)__island.draws.push([first,count]);
    if(this.__water&&this.__yardIndices)__island.yardDraws.push(count);
  });
`;

async function inspectChannelRipples(evaluate) {
  // Isolate a marker wave with the actual departure functions and map used
  // by the WebGPU surface. The Metal probe fits it to submitted depth.
  const windows=[0,0.4,0.8,1,1.8,1.95,2.8,3.8,3.95,5,7.95,59.95,279.95];
  const samples=await evaluate(`(async()=>{
    const d=__devices.at(-1), source=__island.source;
    const start=source.indexOf('fn cascade_departure');
    const end=source.indexOf('// Two readings',start);
    const departure=source.slice(start,end);
    if (start<0 || end<0 || !source.includes('cascade_motion(input.rest, row.current') ||
        !source.includes('cascade_motion(point, row.current, row.track, water.carriage.w, water.carriage.y)')) {
      throw Error('submitted vertex and fragment carriage functions unavailable');
    }
    const map=('const BODY_LIMIT'+source.split('const BODY_LIMIT')[1].split('fn surface_body')[0])
      .replace('WATER_MAP_GROUP = 3u','WATER_MAP_GROUP = 0u');
    const starts=${JSON.stringify(windows)};
    const count=starts.length*5;
    const size=count*16;
    const code=departure+map+\`
      @group(0) @binding(0) var<storage, read_write> drift: array<vec4<f32>>;

      fn chart_motion(place: vec2<f32>, seconds: f32) -> CascadeMotion {
        let row = map_sample(place);
        let current = row.row.water.yz;
        return cascade_motion(place, current, map_wave_track(place, row.body, current), seconds, 4.0);
      }

      @compute @workgroup_size(1)
      fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
        let index = id.x;
        let windows = array<f32, \${starts.length}>(\${starts.map(t=>t.toFixed(2)).join(',')});
        let reach = index % 5u;
        let start = windows[index / 5u];
        let end = start + 0.2;
        let k = 0.7853981634;
        let omega = sqrt(9.81 * k);
        var before: f32;
        var after: f32;
        var gradient = 1.0;
        var track: f32;
        var speed: f32;
        if reach < 3u {
            speed = array<f32, 3>(0.0, 0.5, 1.5)[reach];
            let point = vec2(11.0, -3.0);
            let current = vec2(speed, 0.0);
            before = cascade_motion(point, current, vec4(0.0), start, 4.0).departure.x;
            after = cascade_motion(point, current, vec4(0.0), end, 4.0).departure.x;
        } else {
            let point = vec2(select(228.0, 256.0, reach == 4u), 5.0);
            let row = map_sample(point);
            speed = row.row.water.y;
            track = map_wave_track(point, row.body, row.row.water.yz).w;
            before = chart_motion(point, start).departure.x;
            after = chart_motion(point, end).departure.x;
            let left = chart_motion(point - vec2(0.1, 0.0), 0.0).departure.x;
            let right = chart_motion(point + vec2(0.1, 0.0), 0.0).departure.x;
            gradient = (right - left) / 0.2;
        }
        let delta = k * (after - before) - omega * (end - start);
        drift[index] = vec4(atan2(sin(delta), cos(delta)), gradient, speed, track);
      }
    \`;
    const module=d.createShaderModule({code});
    const layout=d.createBindGroupLayout({entries:[
      {binding:0,visibility:GPUShaderStage.COMPUTE,buffer:{type:'storage'}},
      {binding:5,visibility:GPUShaderStage.COMPUTE,texture:{sampleType:'unfilterable-float',viewDimension:'2d-array'}},
      {binding:6,visibility:GPUShaderStage.COMPUTE,buffer:{type:'uniform'}}
    ]});
    const pipe=await d.createComputePipelineAsync({layout:d.createPipelineLayout({bindGroupLayouts:[layout]}),
      compute:{module,entryPoint:'probe'}});
    const storage=d.createBuffer({size,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
    const staging=d.createBuffer({size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
    const group=d.createBindGroup({layout,entries:[{binding:0,resource:{buffer:storage}},
      ...__island.bindings.filter(e=>e.binding===5||e.binding===6)]});
    const enc=d.createCommandEncoder(), pass=enc.beginComputePass();
    pass.setPipeline(pipe);pass.setBindGroup(0,group);pass.dispatchWorkgroups(count);pass.end();
    enc.copyBufferToBuffer(storage,0,staging,0,size);d.queue.submit([enc.finish()]);
    await staging.mapAsync(GPUMapMode.READ);
    const phases=Array.from(new Float32Array(staging.getMappedRange()));
    staging.unmap();staging.destroy();storage.destroy();
    return {phases,departure,map};
  })()`);
  const k=2*Math.PI/8, duration=0.2;
  const rows=windows.map((start,i)=>{
    const still=samples.phases[i*20];
    assert(Math.abs(still+Math.sqrt(9.81*k)*duration)<0.001,'incorrect still marker phase');
    const reaches=Array.from({length:5},(_,j)=>{
      const [phase,gradient,expected,track]=samples.phases.slice(i*20+j*4,i*20+j*4+4);
      const speed=(still-phase)/(k*duration*gradient);
      if(j>=3)assert(track===1&&gradient>0&&expected>0,'live chart phase track unavailable');
      assert(Math.abs(speed-expected)<(j>=3?0.065:0.02),'browser ripple reversed or drifted: '+JSON.stringify({start,j,speed,expected}));
      return {speed,expected,phase,gradient,track};
    });
    assert(reaches[4].speed>reaches[3].speed+0.6,'live chart narrows did not carry the marker faster');
    return {start,duration,reaches};
  });
  return {source_sha256:hash(samples.departure+samples.map),rows};
}

async function inspectIsland(evaluate, result) {
  const rustc=execFileSync('rustup',['which','--toolchain','stable','rustc'],{encoding:'utf8'}).trim();
  const oracle=JSON.parse(execFileSync('rustup',['run','stable','cargo','run','--quiet','--release',
    '--locked','--offline','-p','water-scene','--example','map_oracle'],{
    maxBuffer:8<<20,env:{...process.env,PATH:path.dirname(rustc)+path.delimiter+process.env.PATH,RUSTC_WRAPPER:''}
  }));
  const probe=await fs.readFile('crates/thrum-water/tests/map_probe.wgsl','utf8');
  const report=await evaluate(`(async()=>{
    const d=__devices.at(-1);
    const entries=__island.surfaceEntries;
    const limits={};
    for(const name of ['maxSampledTexturesPerShaderStage','maxStorageTexturesPerShaderStage',
      'maxStorageBuffersPerShaderStage','maxUniformBuffersPerShaderStage','maxTextureArrayLayers',
      'maxTextureDimension2D'])limits[name]=d.limits[name];
    const stages=[GPUShaderStage.VERTEX,GPUShaderStage.FRAGMENT].map(stage=>{
      const used=entries.filter(e=>e.visibility&stage);
      return {sampled:used.filter(e=>e.texture).length,
        storageFormats:used.filter(e=>e.storageTexture).map(e=>e.storageTexture.format),
        storageBuffers:used.filter(e=>e.buffer&&e.buffer.type?.includes('storage')).length,
        uniforms:used.filter(e=>e.buffer&&(!e.buffer.type||e.buffer.type==='uniform')).length};
    });
    const points=${JSON.stringify(oracle.points)};
    const code=__island.source.split('const BODY_LIMIT')[1].split('fn surface_body')[0];
    const shader=d.createShaderModule({code:('const BODY_LIMIT'+code).replace('WATER_MAP_GROUP = 3u','WATER_MAP_GROUP = 0u')+
      ${JSON.stringify(probe)}});
    const values=new Float32Array(points.length*8);
    points.forEach(([x,z],i)=>values.set([x,z],i*8));
    const input=d.createBuffer({size:values.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
    d.queue.writeBuffer(input,0,values);
    const size=points.length*48;
    const output=d.createBuffer({size,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
    const staging=d.createBuffer({size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
    const binding=GPUShaderStage.COMPUTE;
    const layout=d.createBindGroupLayout({entries:[
      {binding:0,visibility:binding,buffer:{type:'read-only-storage'}},
      {binding:1,visibility:binding,buffer:{type:'storage'}},
      {binding:5,visibility:binding,texture:{sampleType:'unfilterable-float',viewDimension:'2d-array'}},
      {binding:6,visibility:binding,buffer:{type:'uniform'}}
    ]});
    const pipe=await d.createComputePipelineAsync({layout:d.createPipelineLayout({bindGroupLayouts:[layout]}),
      compute:{module:shader,entryPoint:'probe'}});
    const group=d.createBindGroup({layout,entries:[
      {binding:0,resource:{buffer:input}},{binding:1,resource:{buffer:output}},
      ...__island.bindings.filter(e=>e.binding===5||e.binding===6)
    ]});
    const enc=d.createCommandEncoder(), pass=enc.beginComputePass();
    pass.setPipeline(pipe);pass.setBindGroup(0,group);pass.dispatchWorkgroups(Math.ceil(points.length/64));pass.end();
    enc.copyBufferToBuffer(output,0,staging,0,size);d.queue.submit([enc.finish()]);
    await staging.mapAsync(GPUMapMode.READ);
    const answers=Array.from(new Float32Array(staging.getMappedRange()));
    staging.unmap();
    input.destroy();output.destroy();staging.destroy();
    return {answers,stages,limits,draws:__island.draws.slice(-3)};
  })()`);
  const seen=[0,0,0,0], err={bed:0,depth:0,level:0,current:0};
  oracle.expected.forEach(([body,level,bed,depth,current],i)=>{
    const [id,h,bottom,d,cx,cz,hasBed]=report.answers.slice(12*i,12*i+7);
    assert(id===(body??-1),`island map owner at ${oracle.points[i]}: ${id}, expected ${body}`);
    assert(Boolean(hasBed)===(bed!==null),'bed extent differs');
    if(bed!==null)err.bed=Math.max(err.bed,Math.abs(bottom-bed));
    if(body!==null){
      err.level=Math.max(err.level,Math.abs(h-level));
      err.current=Math.max(err.current,Math.abs(cx-current[0]),Math.abs(cz-current[1]));
      if(depth!==null)err.depth=Math.max(err.depth,Math.abs(d-depth));
      else assert(d>1e29,'unbounded water reports finite depth');
    }
    seen[body??3]++;
  });
  assert(seen.every(n=>n>=30)&&oracle.shore_crossings.every(n=>n>=20),'every island shore must be covered');
  assert(Math.max(err.bed,err.depth,err.level)<0.00004&&err.current<0.000004,JSON.stringify(err));
  assert(report.draws.length===3&&report.draws.every((d,i)=>d[1]>0&&d[0]===(i===0?0:report.draws[i-1][0]+report.draws[i-1][1])),'island body draws are absent');
  for(const stage of report.stages) {
    assert(stage.sampled<=Math.min(16,report.limits.maxSampledTexturesPerShaderStage),'sampled texture budget');
    assert(stage.storageFormats.length===0&&stage.storageBuffers===0,'surface must use sampled map data');
    assert(stage.uniforms<=report.limits.maxUniformBuffersPerShaderStage,'uniform budget');
  }
  result.map={scattered:4096,points:oracle.points.length,shore_crossings:oracle.shore_crossings,
    classifications:seen,max_absolute_error:err,...{stages:report.stages,limits:report.limits,draws:report.draws}};
}
async function inspectSurf(evaluate,result,frame) {
  const witness=await evaluate(`(async()=>{
    const device=__devices.at(-1);
    const names=['surf field settings','water surface vertices','surf field cache'];
    const encoder=device.createCommandEncoder();
    const staging=names.map(name=>{
      const source=__buffers[name];
      if(!source)throw Error('missing surf product: '+name);
      const buffer=device.createBuffer({size:source.size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
      encoder.copyBufferToBuffer(source,0,buffer,0,buffer.size);
      return buffer;
    });
    device.queue.submit([encoder.finish()]);
    const [settings,mesh,cache]=await Promise.all(staging.map(async buffer=>{
      await buffer.mapAsync(GPUMapMode.READ);
      const values=new Float32Array(buffer.getMappedRange().slice(0));
      buffer.unmap();buffer.destroy();return values;
    }));
    const nodes=[],samples=[];
    for(let i=0;i<mesh.length/4;i++){
      const [x,z,spacing,body]=mesh.subarray(i*4,i*4+4);
      const halfWidth=spacing<=0.5?64:spacing===1?80:spacing===2?48:spacing===8?40:32;
      const cacheOffset=i*12;
      if(cache[cacheOffset+3]===0 || (body===0&&Math.max(Math.abs(x),Math.abs(z))>=halfWidth*spacing))continue;
      if(i%6!==0)continue;
      nodes.push(body===0?[x+Math.floor(settings[0]),z+Math.floor(settings[1]),body]:[x,z,body]);
      samples.push(Array.from(cache.subarray(cacheOffset,cacheOffset+3)));
    }
    const entries=__island.surfEntries;
    const limits={};
    for(const name of ['maxSampledTexturesPerShaderStage','maxStorageBuffersPerShaderStage',
      'maxStorageBufferBindingSize','maxTextureArrayLayers','maxTextureDimension2D'])limits[name]=device.limits[name];
    return {nodes,samples,limits,textures:__island.surfTextures,
      layout:{sampled:entries.filter(e=>e.texture).length,storage:entries.filter(e=>e.buffer?.type?.includes('storage')).length},
      storageBytes:__buffers['surf field cache'].size,draws:__island.draws.slice(-3)};
  })()`);
  const {limits,layout,textures,storageBytes,nodes,samples,draws}=witness;
  assert(textures.length===4&&draws.length===3,'surf products or body draws missing');
  assert(layout.sampled<=limits.maxSampledTexturesPerShaderStage&&layout.storage<=limits.maxStorageBuffersPerShaderStage,'surf bindings exceed portable limits');
  assert(storageBytes<=limits.maxStorageBufferBindingSize,'surf field cache exceeds storage limit');
  for(const {size} of textures) {
    const [width,height,layers]=Array.isArray(size)?size:[size.width,size.height,size.depthOrArrayLayers];
    assert(Math.max(width,height)<=limits.maxTextureDimension2D&&layers<=limits.maxTextureArrayLayers,'surf texture exceeds portable limits');
  }
  const rustc=execFileSync('rustup',['which','--toolchain','stable','rustc'],{encoding:'utf8'}).trim();
  const time=(frame.tick-1+frame.alpha)/60;
  const oracle=JSON.parse(execFileSync('rustup',['run','stable','cargo','run','--quiet','--release',
    '--locked','--offline','-p','water-scene','--example','surf_oracle'],{
    input:JSON.stringify({time,nodes}),maxBuffer:16<<20,
    env:{...process.env,PATH:path.dirname(rustc)+path.delimiter+process.env.PATH,RUSTC_WRAPPER:''}
  }));
  let checked=0,maxError=0,worst=null;
  oracle.answers.forEach((expected,i)=>{
    if(expected===null)return;
    checked++;
    expected.forEach((value,j)=>{
      const error=Math.abs(samples[i][j]-value);
      if(error>maxError){maxError=error;worst={node:nodes[i],component:j,actual:samples[i],expected};}
    });
  });
  result.surf={time,checked,maxError,worst,limits,layout,textures,storageBytes,draws};
  assert(checked>200&&maxError<0.015,`surf field comparison: ${checked} samples, ${maxError} m maximum error`);
}

const log=await fs.open(path.join(output,'command.log'),'w');
const command=spawn('./dev',['scene','examples/water','--web'],{env:{...process.env,THRUM_SCENE_OUTPUT:output,THRUM_SCENE_WEB_CAPTURE:'1'},stdio:['ignore',log.fd,log.fd],detached:true});
let chrome,cdp,sid,failure;
const result={mode,command:'./dev scene examples/water --web',commit:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),target:'chrome-webgpu',output,frames:[]};
result.dirty_diff_sha256=hash(execFileSync('git',['diff','HEAD']));
result.source_sha256={};
for(const file of ['crates/thrum-app/src/sky.rs','crates/thrum-app/src/sky/browser.rs','crates/thrum-app/web/scene.js','crates/thrum-render/src/runtime.rs','crates/thrum-render/src/runtime/device.rs','examples/water/src/lib.rs','examples/water/src/main.rs','examples/water/examples/field_oracle.rs','scripts/check_water_browser.mjs'])result.source_sha256[file]=hash(await fs.readFile(file));
if(mode==='under'){
  for(const file of ['crates/thrum-water-model/src/cut.rs','crates/thrum-water/src/underwater.rs','crates/thrum-water/src/underwater.wgsl','crates/thrum-water/src/underwater/cut_gpu.wgsl','crates/thrum-water/src/surface_vertex_geometry.wgsl','examples/water/src/lens_probe.rs','examples/water/examples/lens_oracle.rs','examples/water/assets/meshes/lens-panel.gltf']){
    result.source_sha256[file]=hash(await fs.readFile(file));
  }
  const shortAdmission='crates/thrum-water/src/underwater/short.rs';
  result.source_sha256[shortAdmission]=hash(await fs.readFile(shortAdmission));
}
try {
  const url=await until(async()=>{
    const transcript=await fs.readFile(path.join(output,'command.log'),'utf8');
    if(command.exitCode!==null)throw Error(`scene command exited ${command.exitCode}: ${transcript.slice(-2500)}`);
    return /SCENE_URL=(\S+)/.exec(transcript)?.[1];
  },'scene URL',600000);
  const wasm=await fs.readFile(path.join(output,'web/scene_bg.wasm'));result.wasm=hash(wasm);
  const profile=path.join(output,'chrome-profile');await fs.mkdir(profile);
  const chromeLog=await fs.open(path.join(output,'chrome.log'),'w');
  chrome=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',[`--user-data-dir=${profile}`,'--headless=new','--remote-debugging-port=0','--no-first-run','--no-default-browser-check','--window-size=2560,1600','about:blank'],{stdio:['ignore',chromeLog.fd,chromeLog.fd]});
  const endpoint=await until(async()=>{try{return(await fs.readFile(path.join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n');}catch{return null;}},'Chrome port');
  const socket=new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);await new Promise((r,j)=>{socket.onopen=r;socket.onerror=j;});cdp=new Cdp(socket);
  result.gpu=(await cdp.call('SystemInfo.getInfo')).gpu;result.browser=await cdp.call('Browser.getVersion');
  assert(/Apple M3 Max/.test(JSON.stringify(result.gpu.devices))&&!/swiftshader|software/i.test(JSON.stringify(result.gpu.devices))&&result.gpu.featureStatus.webgpu==='enabled','hardware WebGPU required');
  const {targetId}=await cdp.call('Target.createTarget',{url:'about:blank'});sid=(await cdp.call('Target.attachToTarget',{targetId,flatten:true})).sessionId;
  const evaluate=async expression=>{const r=await cdp.call('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true},sid);if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value;};
  const pixels=async expression=>{
    const size=await evaluate(`${expression}.length`),chunks=[];
    for(let offset=0;offset<size;offset+=1024*1024){
      const data=await evaluate(`(()=>{const p=${expression}.subarray(${offset},${offset+1024*1024});let bytes='';for(let i=0;i<p.length;i+=8192)bytes+=String.fromCharCode(...p.subarray(i,i+8192));return btoa(bytes);})()`);
      chunks.push(Buffer.from(data,'base64'));
    }
    return Buffer.concat(chunks);
  };
  await cdp.call('Page.enable',{},sid);await cdp.call('Runtime.enable',{},sid);
  await installWasmReceipt(cdp,sid);
  const surfRoute=mode==='surf'||surfInspection;
  await cdp.call('Page.addScriptToEvaluateOnNewDocument',{source:observer+(mode==='under'?lensObserver:'')+(['island','surf','channel','river','river-calm','rain-puddles','rain-rings','rain-footfall','rain-ground'].includes(mode)?islandObserver:'')+`\ndocument.addEventListener('DOMContentLoaded',()=>{document.querySelector('canvas').width=${width};document.querySelector('canvas').height=${height};});`},sid);
  const route=diveRoute??opticsRoute??(surfRoute?(handoffCut?'?surf=gale':'?surf=trade'):mode==='foam'?'?foam=calm&speed=6&view=overhead&wake-bodies=1':mode==='wake'?'?wake=calm&speed=6&view=overhead&wake-bodies=1':mode==='bodies'?'?bodies=trade&speed=5':mode==='island'?'?island=trade':mode==='channel'?'?channel=trade':mode==='river-calm'?'?river=calm':mode==='river'?'?river=trade':['rain-puddles','rain-rings','rain-footfall','rain-ground'].includes(mode)?'?rain=calm':mode==='under'?lensRoute('sea','baseline'):'');
  result.route=route;
  const navigate=async search=>{
    await cdp.call('Page.navigate',{url:url+search},sid);
    await until(async()=>{
      const state=await evaluate('({search:location.search,ready:!!window.game,status:document.querySelector("#status")?.textContent})');
      if(state.search!==search)return false;
      if(state.status?.includes('could not start'))throw Error(state.status);
      return state.ready;
    },'scene constructed');
    return readWasmReceipt(cdp,sid,url+'scene_bg.wasm',result.wasm,wasm.length);
  };
  result.instantiated_wasm=await navigate(route);
  if(['rain-puddles','rain-rings','rain-footfall','rain-ground'].includes(mode))await evaluate("game.edit_control('rain.rate',0)");
  const fixedSurf=surfRoute&&mode!=='realtime';
  const lensClockOrigin=async()=>{
    const {origin}=await until(async()=>{
      const value=await evaluate('__lensClockOrigin');
      return Number.isFinite(value)?{origin:value}:null;
    },'the lens page clock origin');
    return origin;
  };
  let now=mode==='under'?await lensClockOrigin():(fixedSurf||mode==='rain-ground')?await evaluate('performance.now()'):0;
  const drive=async delta=>{now+=delta;await evaluate(`game.frame(${now})`);};
  const capture=async(name,delta=1000/120,probe=false)=>{
    await until(()=>evaluate('game.frame_ready()'),'free frame slot');
    await evaluate('game.begin_capture()');if(probe)await evaluate('__probeNext=true');
    now+=delta;
    const submitted=frameInfo(await evaluate(`game.frame(${now});game.frame_info()`));
    const image=await until(()=>evaluate('(()=>{const v=game.capture();if(!v)return null;globalThis.__capturePixels=v.rgba;return {identity:v.identity,frame:v.frame};})()'),'submitted image');
    await evaluate('game.end_capture()');
    const rgba=await pixels('__capturePixels');
    assert(image.identity.includes('presented=true'),'checkpoint was not the submitted surface frame');
    assert(rgba.length===width*height*4,'capture extent differs');
    const file=name+'.png';await fs.writeFile(path.join(output,file),png(rgba,width,height));
    const info=frameInfo(image.frame);
    for(const field of ['tick','alpha','submission']){
      assert(submitted[field]===info[field],`${name}: immediate frame report ${field} differs from submitted output`);
    }
    assert(submitted.generation===Number(/device=(\d+)/.exec(image.identity)[1]),`${name}: frame report generation differs`);
    const record={name,file,...image,...info,frame_report:submitted,sha256_rgba:hash(rgba)};
    if(mode==='river-calm')record.water=riverLook(rgba,name);
    if(probe){
      const [params,long,cascades]=await evaluate('__readFields()');
      const anchor=await evaluate('JSON.parse(game.controls_json()).controls.filter(c=>c.key.startsWith("camera.")).map(c=>[c.key,c.value])');
      const controls=Object.fromEntries(anchor),h=controls['camera.height'],bearing=controls['camera.bearing']*Math.PI/180;
      const radius=24+(h-2)*16/38,eye=[Math.fround(132-Math.sin(bearing)*radius),Math.fround(32+Math.cos(bearing)*radius)];
      const points=long.map(({level,pixel})=>{const p=params.slice(12+level*4,16+level*4);return [eye[0]+p[0]+(pixel%128+0.5)*p[2],eye[1]+p[1]+(Math.floor(pixel/128)+0.5)*p[2],p[3]];});
      const time=Math.max(0,(info.tick-1+info.alpha)*(1/60));
      const oracle=JSON.parse(execFileSync('target/release/examples/field_oracle',[],{input:JSON.stringify({time,long:points}),maxBuffer:8<<20}));
      let height=0,slope=0;const bands=[0,0,0];
      long.forEach(({value},i)=>{height=Math.max(height,Math.abs(value[1]-oracle.long[i].displacement[1]));for(let a=0;a<2;a++)slope=Math.max(slope,Math.abs(value[4+a]-oracle.long[i].slope[a]));});
      cascades.forEach((samples,b)=>samples.forEach((value,i)=>{for(let a=0;a<3;a++)for(const [offset,key]of [[0,'displacement'],[8,'velocity']])bands[b]=Math.max(bands[b],Math.abs(value[offset+a]-oracle.cascades[b].samples[i][key][a])/oracle.cascades[b].rms);}));
      record.fields={time,height,slope,cascade_error_per_rms:bands,long_points:points.length,cascade_points_per_band:32,clock_word:params[0]};
      assert(height<0.002&&slope<0.002,`long field mismatch ${JSON.stringify(record.fields)}`);
      assert(bands.every(v=>v<0.0004),`cascade mismatch ${JSON.stringify(record.fields)}`);
    }
    result.frames.push(record);return record;
  };
  for(let i=0;i<8;i++){await until(()=>evaluate('game.frame_ready()'),'warm frame slot');await drive(i===0||mode==='under'?0:1000/120);await delay(30);}
  let surfClockOrigin;
  if(fixedSurf){
    await until(async()=>{
      const status=await evaluate(`game.frame(${now});game.report()`);
      return /^Cooked generation [0-9]+ drawn/.test(status);
    },'cooked surf drawn');
    const anchor=await capture('surf-clock-anchor',0);
    surfClockOrigin=now-anchor.observation*1000;
  }
  if(mode==='under'){
    const rustc=execFileSync('rustup',['which','--toolchain','stable','rustc'],{encoding:'utf8'}).trim();
    const oracle=JSON.parse(execFileSync('rustup',['run','stable','cargo','run','--quiet','--release',
      '--locked','--offline','-p','water-scene','--example','lens_oracle'],{
      env:{...process.env,PATH:path.dirname(rustc)+path.delimiter+process.env.PATH,RUSTC:rustc}
    }));
    assert(oracle.extent.join()===`${width},${height}`,'oracle and browser extents differ');
    result.extent=oracle.extent;
    result.lens={};
    const lensFailures=[];
    const take=async (fixture,arm,targetTick=1)=>{
      const search=lensRoute(fixture,arm);
      const instantiated_wasm=search===route?result.instantiated_wasm:await navigate(search);
      now=await lensClockOrigin();
      await until(async()=>{
        if(!await evaluate('game.frame_ready()'))return false;
        await drive(0);
        const status=await evaluate('game.report()');
        if(status.startsWith('Asset load failed:'))throw Error(status);
        return /^Cooked generation [0-9]+ drawn/.test(status);
      },'lens witness panel drawn');
      for(let tick=1;tick<targetTick;tick++){
        await until(()=>evaluate('game.frame_ready()'),'impact frame slot');
        await drive(oracle.step_ns/1e6);
      }
      await evaluate('__lensPasses={gather:0,cut:0,composition:0}');
      // Water presents between committed states. Tick one at alpha zero
      // gives model time zero, while the initial presentation is a step early.
      const frame=await capture(`${fixture}-${arm}${targetTick===1?'':`-tick-${targetTick}`}`,oracle.step_ns/1e6);
      frame.lens_passes=await evaluate('__lensPasses');
      frame.instantiated_wasm=instantiated_wasm;
      frame.faults=await evaluate('__faults');
      assert(frame.faults.length===0,JSON.stringify(frame.faults));
      assert(!(await evaluate('document.querySelector("#problems").textContent')),'lens page reports a problem');
      assert(Math.abs(frame.tick+frame.alpha-targetTick)<1e-5,`the fixture requires endpoint ${targetTick}: ${frame.frame}`);
      return {frame,rgba:await pixels('__capturePixels')};
    };
    for(const fixture of ['trade','lake','swash','lip','carried-crest','carried-gain','carried-weight','trough-wet']){
      const expected=oracle.cases[fixture];
      assert(expected?.length===width*height,`missing model pixels for ${fixture}`);
      const baseline=await take(fixture,'baseline'),split=await take(fixture,'split');
      const fraction=expected.filter(Boolean).length/expected.length;
      const proof={fraction,checked:0,checkedWet:0,checkedDry:0,obscured:[],wrong:[]};
      result.lens[fixture]=proof;
      const range=fixture==='trough-wet'?[0.1,0.4]:[0.25,0.75];
      assert(fraction>range[0]&&fraction<range[1],`${fixture} camera must have the modelled split`);
      assert(baseline.frame.lens_passes.composition===0,'baseline used lens composition');
      assert(split.frame.lens_passes.composition===1&&split.frame.lens_passes.gather===1&&split.frame.lens_passes.cut===1,'split lens did not cut the mesh and compose the gathered panel');
      for(let y=2;y<height-2;y++)for(let x=1;x<width-1;x++){
        const i=y*width+x,wet=expected[i];
        if([i-1,i+1,i-2*width,i+2*width].some(n=>expected[n]!==wet))continue;
        const ordinary=baseline.rgba[4*i],measured=split.rgba[4*i];
        if(ordinary<40){proof.obscured.push({x,y,ordinary});continue;}
        proof.checked++;
        proof[wet?'checkedWet':'checkedDry']++;
        if((measured<0.85*ordinary)!==wet)proof.wrong.push({x,y,wet,ordinary,measured});
      }
      assert(proof.checkedWet>0&&proof.checkedDry>0,`${fixture} oracle did not cover both sides`);
      if(proof.obscured.length||proof.wrong.length)lensFailures.push(`${fixture}: ${proof.wrong.length} disagreements and ${proof.obscured.length} obscured panel pixels`);
    }
    const dry=await take('sea','dry');
    assert(dry.frame.lens_passes.gather===0&&dry.frame.lens_passes.composition===0,'dry lens encoded underwater work');
    const trough=await take('short-trough','dry'),crest=await take('short-trough','split');
    assert(trough.frame.lens_passes.gather===0&&trough.frame.lens_passes.composition===0,'dry short-wave trough encoded underwater work');
    assert(crest.frame.lens_passes.gather===1&&crest.frame.lens_passes.composition===1,'short-wave crest omitted underwater work');
    // Ordinary surface optics can gather in the mapped fixture. The lens
    // must leave the submitted image unchanged and omit its composition.
    const mappedBase=await take('mapped-trough','dry-baseline');
    const mappedDry=await take('mapped-trough','dry'),mappedWet=await take('mapped-trough','split');
    assert(mappedDry.frame.lens_passes.gather===mappedBase.frame.lens_passes.gather &&
      mappedDry.frame.lens_passes.composition===0 &&
      mappedDry.frame.sha256_rgba===mappedBase.frame.sha256_rgba,'dry mapped trough changed the ordinary optical frame');
    assert(mappedWet.frame.lens_passes.gather===1&&mappedWet.frame.lens_passes.composition===1,'mapped short-wave crest omitted underwater work');
    const tradeDry=await take('trade-admission','dry'),tradeWet=await take('trade-admission','split');
    assert(tradeDry.frame.lens_passes.gather===0&&tradeDry.frame.lens_passes.composition===0,'dry Trade lens encoded underwater work');
    assert(tradeWet.frame.lens_passes.gather===1&&tradeWet.frame.lens_passes.composition===1,'submerged Trade lens omitted underwater work');
    const impactStill=await take('impact','split');
    assert(impactStill.frame.lens_passes.cut===0&&impactStill.frame.lens_passes.composition===0,'the calm lake lens must be dry before the impact');
    const ordinary=await take('impact','baseline',8),impact=await take('impact','split',8);
    let impactPixels=0;
    for(let i=0;i<ordinary.rgba.length;i+=4){
      if(ordinary.rgba[i]>40&&impact.rgba[i]<0.85*ordinary.rgba[i])impactPixels++;
    }
    result.lens.impact={pixels:impactPixels,tick:impact.frame.tick,alpha:impact.frame.alpha};
    assert(ordinary.frame.tick===impact.frame.tick&&ordinary.frame.alpha===impact.frame.alpha,'impact arms sampled different endpoints');
    assert(impactPixels>16&&impact.frame.lens_passes.cut===1&&impact.frame.lens_passes.composition===1,'the lake impact did not wet the lens');
    assert(lensFailures.length===0,lensFailures.join('; '));
  }else if(mode==='rain-ground'){
    await until(async()=>{
      const status=await evaluate(`game.frame(${now});game.report()`);
      return /^Cooked generation [0-9]+ drawn/.test(status);
    },'cooked wet ground drawn');
    await evaluate("game.edit_control('camera.island',2);game.edit_control('sun.elevation',35);game.edit_control('rain.time_scale',600)");
    const steps=async count=>{
      for(let i=0;i<count;i++){
        await until(()=>evaluate('game.frame_ready()'),'wet ground frame slot');
        await drive(1000/60);
      }
    };
    const source=await fs.readFile('crates/thrum-water/src/wet_ground.wgsl','utf8');
    // Read the production ground function against the texture bound for the captured render.
    const evaluator=await fs.readFile('scripts/wet_ground_browser.wgsl','utf8');
    const sample=()=>evaluate(`(async()=>{
      const d=__devices.at(-1);
      const field=__island.bindings.find(e=>e.binding===26);
      if(!field)throw Error('the water film has no shared wetness texture');
      const code=${JSON.stringify(source+evaluator)};
      const module=d.createShaderModule({code});
      const stage=GPUShaderStage.COMPUTE;
      const entries=[{binding:0,visibility:stage,buffer:{type:'storage'}},
         {binding:26,visibility:stage,texture:{sampleType:'unfilterable-float'}}];
      const layout=d.createBindGroupLayout({entries});
      const pipeline=await d.createComputePipelineAsync({layout:d.createPipelineLayout({bindGroupLayouts:[layout]}),
        compute:{module,entryPoint:'probe_wet_ground'}});
      const size=80;
      const buffer=d.createBuffer({size,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
      const readback=d.createBuffer({size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
      const bind=d.createBindGroup({layout,entries:[{binding:0,resource:{buffer}},field]});
      const encoder=d.createCommandEncoder(), pass=encoder.beginComputePass();
      pass.setPipeline(pipeline);pass.setBindGroup(0,bind);pass.dispatchWorkgroups(1);pass.end();
      encoder.copyBufferToBuffer(buffer,0,readback,0,size);d.queue.submit([encoder.finish()]);
      await readback.mapAsync(GPUMapMode.READ);
      const values=Array.from(new Float32Array(readback.getMappedRange()));
      readback.unmap();readback.destroy();buffer.destroy();
      return {joint:values.slice(0,4),face:values.slice(4,8),shelter:values.slice(8,12), shiftedJoint:values.slice(12,16), oldJoint:values.slice(16,20)};
    })()`);
    await steps(2);
    await capture('ground-dry',0);
    const dry=await sample();
    await evaluate("game.edit_control('rain.rate',10)");
    await steps(21);
    await capture('ground-wet',0);
    const wet=await sample();
    await evaluate("game.edit_control('rain.rate',0);game.edit_control('rain.drying',100)");
    await steps(55);
    await capture('ground-dried',0);
    const dried=await sample();
    assert(dry.joint[0]===0&&dry.joint[1]===0,'joint starts dry');
    assert(wet.joint[0]>0.05&&wet.joint[1]>0&&wet.joint[2]>wet.face[2],
      `wetness should darken exposed paving and reach its joints first: ${JSON.stringify(wet)}`);
    assert(wet.shiftedJoint[2]>wet.oldJoint[2],'film follows the translated authored joint');
    assert(wet.shelter[0]===0&&wet.shelter[1]===0,'sheltered point must stay dry');
    assert(dried.joint[0]<wet.joint[0]*0.1&&dried.joint[2]===0,'film and then pores dry');
    assert(new Set(result.frames.map(frame=>frame.sha256_rgba)).size===3,'submitted ground states must differ');
    const layout=await evaluate(`(()=>{
      const device=__devices.at(-1), stage=GPUShaderStage.FRAGMENT;
      const count=entries=>entries.filter(e=>e.texture&&(e.visibility&stage)).length;
      return {forward:count(__island.forwardEntries),surface:count(__island.surfaceEntries),
        film:count(__island.filmEntries),limit:device.limits.maxSampledTexturesPerShaderStage};
    })()`);
    assert(layout.forward>0&&layout.surface>0&&layout.film>0&&
      [layout.forward,layout.surface,layout.film].every(count=>count<=layout.limit),'portable layouts');
    result.ground={dry,wet,dried,layout};
  }else if(mode==='rain-footfall'){
    await until(async()=>{
      const status=await evaluate(`game.frame(${now});game.report()`);
      return /^Cooked generation [0-9]+ drawn/.test(status);
    },'cooked footfall yard drawn');
    const step=async()=>{
      await until(()=>evaluate('game.frame_ready()'),'footfall frame slot');
      await drive(1000/60);
      return evaluate('(game.report(),globalThis.waterRainYard)');
    };
    await evaluate("game.edit_control('camera.island',8);game.edit_control('rain.rate',100);game.edit_control('rain.time_scale',600);game.edit_control('sun.elevation',29)");
    const separate=await until(async()=>{
      const state=await step();
      return state[0]>state[1] && state[1]>18.016 ? state : null;
    },'both puddles wet with a dry sill');
    await evaluate("game.edit_control('rain.rate',0);game.edit_control('rain.time_scale',1)");
    for(let i=0;i<65;i++)await step();
    await capture('still-puddles',0);
    await evaluate("game.edit_control('rain.walk',1)");
    const samples=[];
    for(let i=0;i<23;i++){
      await step();
      const {tick}=frameInfo(await evaluate('game.frame_info()'));
      const {fine}=await evaluate(`__readWake(${tick},[],null,[147.0,66.39,148.44])`);
      samples.push({tick,...fine});
    }
    const first=samples.find(row=>row.source>0.0004);
    const crossing=samples.find(row=>first&&row.tick>first.tick+3&&row.front>Math.max(0.00025,4*first.front));
    assert(first&&crossing,`no footfall propagated in the source puddle: ${JSON.stringify(samples)}`);
    assert(samples.every(row=>row.beyond===0&&row.beyondPotential===0),'wave leaked across the dry sill');
    await capture('footfall-in-motion',0);
    assert(result.frames[0].sha256_rgba!==result.frames[1].sha256_rgba,'the submitted footfall frame did not change');
    result.footfall={separate,first,crossing,samples};
  }else if(mode==='rain-rings'){
    await until(async()=>{
      const status=await evaluate(`game.frame(${now});game.report()`);
      return /^Cooked generation [0-9]+ drawn/.test(status);
    },'cooked rain yard drawn');
    const state=()=>evaluate('(game.report(),{yard:globalThis.waterRainYard,passes:globalThis.__rainPasses})');
    const frames=async count=>{
      for(let i=0;i<count;i++){
        await until(()=>evaluate('game.frame_ready()'),'rain frame slot');
        await drive(1000/60);
      }
      return state();
    };
    await evaluate("game.edit_control('camera.island',4);game.edit_control('rain.rate',100);game.edit_control('rain.time_scale',600)");
    await until(async()=>{
      const {yard}=await frames(4);
      return yard?.[0]>18.112 && yard[1]>18.112;
    },'filled paving puddle');
    await evaluate("game.edit_control('rain.rate',50);game.edit_control('rain.time_scale',1)");
    const wet=await frames(12);
    assert(wet.yard.slice(5).every(count=>count>0)&&wet.passes>0,`both rain bands must submit real drops: ${JSON.stringify(wet)}`);
    assert(wet.yard[3]===null,'roof column stayed dry');
    result.rain_layout=await evaluate(`(()=>{
      const device=__devices.at(-1), limits=device.limits;
      const count=stage=>Object.fromEntries(['texture','sampler','buffer'].map(type=>
        [type,__island.surfaceEntries.filter(entry=>(entry.visibility&stage)&&entry[type]).length]));
      return {
        field:__island.fieldLayout, textures:__island.rainTextures,
        vertex:count(GPUShaderStage.VERTEX), fragment:count(GPUShaderStage.FRAGMENT),
        limits:{texture:limits.maxSampledTexturesPerShaderStage,sampler:limits.maxSamplersPerShaderStage},
      };
    })()`);
    const {field,textures}=result.rain_layout;
    assert(field.find(entry=>entry.binding===20)?.texture?.sampleType==='unfilterable-float',
      'canopy must be a read-only texture');
    assert(field.find(entry=>entry.binding===21)?.sampler,'rain samples the shared array with its own sampler');
    assert(textures.some(({label,size})=>label==='sea filtered fields'&&
      (size.depthOrArrayLayers??size[2])===19),'shared cascade array must include the four rain planes');
    assert(!textures.some(({label})=>label==='rain bands'),'no separate rain field texture');
    for(const stage of ['vertex','fragment'])for(const type of ['texture','sampler'])
      assert(result.rain_layout[stage][type]<=result.rain_layout.limits[type],`${stage} ${type} limit`);
    await capture('rings-yard',0);
    await evaluate("game.edit_control('camera.island',6)");
    await frames(4);
    await capture('rings-lake',0);
    await evaluate("game.edit_control('rain.rate',0)");
    const fading=await frames(24);
    assert(fading.yard.slice(5).every(count=>count>0)&&fading.passes>wet.passes,'existing rings must run after the stop');
    const dry=await frames(44);
    assert(dry.yard[5]===0&&dry.yard[6]===0,'old rings must expire');
    const later=await frames(4);
    assert(dry.passes===later.passes,'dry rain passes must skip');
    await capture('still-lake',0);
    assert(result.frames[1].sha256_rgba!==result.frames[2].sha256_rgba,'lake appearance must change when the rings expire');
    result.rain={wet,fading,dry,later};
  } else if(mode==='rain-puddles'){
    await until(async()=>{
      const status=await evaluate(`game.frame(${now});game.report()`);
      return /^Cooked generation [0-9]+ drawn/.test(status);
    },'cooked rain yard drawn');
    const status=()=>evaluate('(game.report(),globalThis.waterRainYard)');
    assert((await evaluate('__island.yardDraws')).length===0,'the dry yard submitted puddle tiles');
    const step=async()=>{
      await until(()=>evaluate('game.frame_ready()'),'yard frame slot');
      await drive(1000/60);
      return status();
    };
    await evaluate("game.edit_control('camera.island',3);game.edit_control('rain.rate',100);game.edit_control('rain.time_scale',600)");
    const first=await until(async()=>{
      const state=await step();
      return state[0]!==null && state[1]!==null && state[0]!==state[1] ? state : null;
    },'separate paving puddles');
    await capture('rain-separate',0);
    const joined=await until(async()=>{
      const state=await step();
      return state[0]!==null && state[0]===state[1] && state[2]!==null &&
        Math.abs(state[2]-state[4])<0.0001 ? state : null;
    },'joined paving puddles and afloat float');
    await capture('rain-joined',200);
    await evaluate("game.edit_control('rain.rate',0)");
    const agrees=state=>state[2]!==null && Math.abs(state[2]-state[4])<0.0001;
    const high=await until(async()=>{
      const state=await step();
      return agrees(state)&&Math.abs(state[2]-state[0])<0.0001 ? state : null;
    },'float settles at the joined waterline');
    await evaluate("game.edit_control('rain.drying',100)");
    await until(async()=>{
      const state=await step();
      return state[0]!==null && state[1]!==null && state[0]!==state[1] &&
        agrees(state) && state[4]<high[4] ? state : null;
    },'puddles part while drying');
    await evaluate("game.edit_control('rain.drying',0)");
    const parted=await until(async()=>{
      const state=await step();
      return agrees(state)&&Math.abs(state[2]-state[0])<0.0001 ? state : null;
    },'float settles at the parted waterline');
    await capture('rain-parted',0);
    const submitted=await evaluate('__island.yardDraws');
    const draws=submitted.length;
    assert(draws>=3&&new Set(submitted).size>1&&submitted.every(count=>count>0),
      'the selected WebGPU surface must submit changing partial yard draws');
    assert(first[4]<joined[4]&&high[4]>parted[4]&&[first,joined,high,parted].every(s=>s[3]===null),
      'the float must rise and fall, with dry ground under the roof');
    assert(new Set(result.frames.map(frame=>frame.sha256_rgba)).size===3,'the submitted yard states must differ');
    result.rain={first,joined,high,parted,draws,submitted};
  } else if(mode==='river-calm'){
    await until(async()=>{
      const status=await evaluate(`game.frame(${now});game.report()`);
      return /^Cooked generation [0-9]+ drawn/.test(status);
    },'cooked calm river island drawn');
    const flowPasses=await evaluate(`(()=>{
      const passes={flow:0,foam:0};
      const start=GPUCommandEncoder.prototype.beginComputePass;
      GPUCommandEncoder.prototype.beginComputePass=function(desc) {
        if(desc?.label?.includes('river flow bands'))passes.flow++;
        if(desc?.label?.endsWith('whitewater'))passes.foam++;
        return start.call(this,desc);
      };
      globalThis.__riverPasses=passes;
      return passes;
    })()`);
    for(const [camera,name] of [[17,'riffle'],[0,'pool'],[3,'ledge']]){
      await evaluate(`game.edit_control('camera.river',${camera})`);
      for(let tick=0;tick<(name==='ledge'?72:16);tick++){
        await until(()=>evaluate('game.frame_ready()'),'calm river frame slot');
        await drive(1000/30);
      }
      await capture(name);
    }
    const submitted=await evaluate('__riverPasses');
    const specialized=await evaluate('__island.source.includes("fn surface_flow(") && __island.source.includes("fn foam_surface(")');
    assert(specialized&&submitted.flow>flowPasses.flow&&submitted.foam>flowPasses.foam,'calm river flow and whitewater must submit');
    assert(result.frames.length===3&&new Set(result.frames.map(frame=>frame.sha256_rgba)).size===3,'three calm river cameras must submit distinct output');
    const [riffle,pool,ledge]=result.frames.map(frame=>frame.water);
    assert(riffle[0].edge_rms>3*pool[0].edge_rms && riffle[0].edge_rms>4,'riffle must glitter while the pool stays smooth');
    assert(ledge[0].bright_cover>0.001 && ledge[0].bright_cover>2*ledge[1].bright_cover,'bright foam cover must thin below the ledge');
    result.river_calm={passes:submitted};
  } else if(mode==='river'){
    const floats=()=>evaluate('(game.report(),globalThis.waterRiverFloats)');
    await until(async()=>{
      const status=await evaluate(`game.frame(${now});game.report()`);
      return /^Cooked generation [0-9]+ drawn/.test(status);
    },'cooked river island drawn');
    const files=await evaluate('globalThis.waterRiverFiles');
    assert(files?.length===2&&files.every(([,key])=>/^[0-9a-f]{64}$/.test(key)),
      'both keyed river files must be loaded from the package');
    for(const [name,key] of files){
      const bytes=await fs.readFile(path.join(output,'rivers',key+'.river'));
      assert(bytes.subarray(12,44).toString('hex')===key,`stale packaged river ${name}`);
    }
    const start=await floats();
    const riverDraws=async(name,delta)=>{
      const before=await evaluate('__island.draws.length');
      await capture(name,delta);
      const draws=await evaluate(`__island.draws.slice(${before})`);
      assert(draws.some(([,count])=>count>0),`${name} submitted no water draws`);
      return draws;
    };
    const streamDraws=await riverDraws('spring-stream',1000);
    await evaluate("game.edit_control('camera.river',11)");
    const outletDraws=await riverDraws('lake-river',1000/30);
    const course=[];
    for(const seconds of [15,30,50,70,95,125,160,200,250,300]){
      await until(()=>evaluate('game.frame_ready()'),'river float travel frame slot');
      await drive(seconds*1000-now);
      const positions=await floats();
      course.push({seconds,positions});
      if(positions[0][3]===1&&positions[0][4]>0)break;
    }
    const end=course.at(-1).positions;
    assert(start[0][3]>=3&&end[0][3]===1&&end[0][4]>0&&end[0].every(Number.isFinite),
      'the stream float did not enter the lake: '+JSON.stringify({start,course,end}));
    assert(new Set(result.frames.map(frame=>frame.sha256_rgba)).size===2,'both views must submit distinct frames');
    result.rivers={files,solves_in_page:0,streamDraws,outletDraws,start,course,end};
  } else if(mode==='channel'){
    const float=()=>evaluate('(game.report(),globalThis.waterChannelFloat)');
    await until(async()=>{
      const status=await evaluate(`game.frame(${now});game.report()`);
      return /^Cooked generation [0-9]+ drawn/.test(status);
    },'cooked channel drawn');
    const start=await float();
    await capture('channel-narrows',1000);
    const ripples=await inspectChannelRipples(evaluate);
    for(let i=0;i<12;i++){
      await until(()=>evaluate('game.frame_ready()'),'channel frame slot');
      await drive(1000/30);
    }
    // A repeat submission rewrites the settings with a zero step duration.
    // Read the transport made by a submitted tick using that tick's settings.
    const detail=await until(async()=>{
      await until(()=>evaluate('game.frame_ready()'),'channel transport frame slot');
      await drive(1000/30);
      const sample=await evaluate('__readFoam([[256,5],[228,5]])');
      return sample.parcels.length ? sample : null;
    },'submitted channel foam step',30000);
    const [fast,slow]=detail.parcels;
    assert(fast.speed[0]>1.3&&slow.speed[0]>0.4&&fast.speed[0]>slow.speed[0]+0.65,'foam detail did not move with the current: '+JSON.stringify(detail));
    assert(detail.filmCells>0,'no submitted foam film');
    const course=[];
    for(const seconds of [20,40,75,110,145,180,215,250,285,320,355,390]){
      await until(()=>evaluate('game.frame_ready()'),'float travel frame slot');
      await drive(seconds*1000-now);
      const position=await float();
      course.push({seconds,position});
      if(position[0]>350.5&&position[3]===0&&position[4]>0)break;
    }
    await evaluate("game.edit_control('camera.channel',3)");
    await capture('channel-mouth',1000/30);
    const end=await float();
    assert(start[0]<215&&start[3]===3&&end[0]>350.5&&end[3]===0&&end[4]>0&&end.every(Number.isFinite),'the float did not enter the sea: '+JSON.stringify({start,course,end}));
    result.channel={start,ripples,detail,course,end};
  } else if(mode==='foam'){
    const runFoam=async steps=>{
      for(let index=0;index<steps;index++){
        await until(()=>evaluate('game.frame_ready()'),'whitewater submission slot');
        await drive(1000/10);
      }
      await until(()=>evaluate('game.frame_ready()'),'whitewater endpoint ready');
      return {...await evaluate('__readFoam()'),time_ms:now,...frameInfo(await evaluate('game.frame_info()'))};
    };
    const running=await runFoam(46);
    await evaluate('game.apply_params(\'{"craft.present":0}\')');
    const after=await runFoam(9);
    const faded=await runFoam(260);
    assert(running.peak>0.12&&after.peak>0.08,'no retained film after removing the craft');
    assert(faded.peak<after.peak*0.55&&faded.mean<after.mean,'retained film did not fade');
    await capture('foam-trail-after-removal',0);
    result.trail={running,after,faded};
    const winds={};
    for(const wind of ['calm','trade','gale']){
      await cdp.call('Page.navigate',{url:url+`?foam=${wind}&speed=0&view=waterline&wake-bodies=1`},sid);
      await until(()=>evaluate('!!window.game'),'whitecaps scene');
      now=0;
      await evaluate('game.apply_params(\'{"craft.present":0}\')');
      winds[wind]=await runFoam(70);
      await capture('foam-'+wind,0);
    }
    assert(winds.calm.mean===0&&winds.trade.mean>0&&winds.gale.mean>winds.trade.mean,JSON.stringify(winds));
    result.whitecaps=winds;
  } else if(mode==='wake'){
    const status=()=>evaluate('(game.report(),globalThis.waterBodyStatus)');
    const head=await status(),samples=[],buoy=[];
    assert(head?.poses.length===9,'wake scene did not publish its fleet');
    let continuing=null;
    for(let index=0;index<670;index++){
      if(index===390)await evaluate(`game.apply_params('{"craft.speed":0}')`);
      await until(()=>evaluate('game.frame_ready()'),'free wake frame slot');
      await drive(1000/60);
      const wake=await status();
      if(!wake?.wakeTick)continue;
      buoy.push([wake.wakeTick,wake.poses[1][1]-head.poses[1][1],wake.poses[0][0]]);
      if(wake.wakeSample){
        const sample=await evaluate('__checkWakeSample('+JSON.stringify(wake.wakeSample)+')');
        if(sample&&!samples.some(entry=>entry.tick===sample.tick))samples.push(sample);
      }
      if(index===490){
        const [x,,z]=wake.poses[0];
        const coastSpeed=60*Math.abs(x-buoy.at(-2)[2]);
        continuing={...await evaluate('__readWake('+wake.wakeTick+',[],'+JSON.stringify([x,z])+')'),coastSpeed};
      }else if(index>=170&&index%18===0){
        await evaluate('__readWake('+wake.wakeTick+')');
      }
    }
    assert(Math.max(...buoy.map(item=>item[1]))>0.001&&Math.min(...buoy.map(item=>item[1]))< -0.001,
      'the float did not rise and fall on the calm sea');
    assert(continuing?.coastSpeed<0.05&&continuing.outsideHull>0.005,'no retained wake away from the stopped craft');
    assert(samples.length>1&&samples.some(sample=>Math.abs(sample.expected)>0.001),'no resolved public wake samples');
    assert(samples.every(sample=>Math.abs(sample.height-sample.expected)<0.00002),'public query differs from submitted wake');
    await capture('wake-sea',0);
    result.wake_sea={public_query:samples,buoy,after_stop:continuing};

    await cdp.call('Page.navigate',{url:url+'?wake=calm&location=lake&stone=1&view=shore&wake-bodies=0'},sid);
    await until(()=>evaluate('!!window.game'),'lake wake scene');
    now=0;
    const rings=[];
    for(let index=0;index<260;index++){
      await until(()=>evaluate('game.frame_ready()'),'free ring frame slot');
      await drive(index===0?0:1000/60);
      const wake=await status();
      if(index%12===0&&wake?.wakeTick)
        rings.push(await evaluate('__readWake('+wake.wakeTick+',[[184,28],[184,31],[184,40],[104,93]])'));
    }
    assert(rings.some(r=>r.tick<60&&r.peak.every(value=>value===0)),'the lake was disturbed before the impact');
    assert(rings.some(r=>r.tick>60&&r.peak[0]>0.002),'the impact did not raise rings');
    const initialRing=rings.find(r=>r.tick>60);
    const lateRing=rings.at(-1);
    assert(lateRing.radius>initialRing.radius+1.5,'the rings did not spread across the lake');
    assert(lateRing.peak[0]<Math.max(...rings.map(r=>r.peak[0]))*0.25,'the ring field did not die away');
    assert(rings.every(r=>r.heights[2]===0),'the shore carried a wave on dry ground');
    assert(rings.every(r=>r.heights[3]===0),'the pond received the lake rings');
    await capture('wake-lake',0);
    result.wake_lake=rings;
  }else if(mode==='bodies'){
    const bodyStatus=()=>evaluate('(game.report(),globalThis.waterBodyStatus)');
    const start=await bodyStatus();
    assert(start?.poses.length===9,'expected craft and eight floats');
    const ages=new Map();
    const replyKey=([body,posted,used])=>`${body}/${posted}/${used}`;
    const beforeStart=new Set(start.replies.map(replyKey));
    const collect=({replies})=>{
      for(const reply of replies) {
        const key=replyKey(reply);
        if(!beforeStart.has(key)){
          const [body,posted,used]=reply;
          ages.set(key,{body,posted,used,age:used-posted});
        }
      }
    };
    const reconcile=(from,to,records)=>{
      const counts=[0,0,0];
      for(const {age} of records){
        assert(age>=1&&age<=3,`invalid reply age ${age}`);
        counts[Math.min(age,3)-1]++;
      }
      for(const [index,key] of ['ageOne','ageTwo','ageOlder'].entries())
        assert(counts[index]===to[key]-from[key],`missing ${key} records: ${counts[index]} vs ${to[key]-from[key]}`);
    };
    for(let index=0;index<96;index++){
      await until(()=>evaluate('game.frame_ready()'),'free body frame slot');
      await drive(1000/120);
      collect(await bodyStatus());
    }
    await capture('bodies',0);
    const end=await bodyStatus();
    collect(end);
    assert(end.dispatches>25&&end.ageOne+end.ageTwo>90,'body replies did not reach the game');
    assert(end.blockingReads===0&&end.refused===0,'query counters indicate a block or capacity refusal');
    assert(end.poses.every(p=>p.length===7&&p.every(Number.isFinite)&&Math.abs(p[1])<4),'a body left the water');
    assert(Math.abs(end.poses[0][0]-start.poses[0][0])>0.8,'the craft did not move under thrust');
    assert(end.poses.slice(1).some((p,i)=>Math.abs(p[1]-start.poses[i+1][1])>0.05),'floats did not respond to the swell');
    reconcile(start,end,ages.values());
    const beforeHitch=new Set(ages.keys());
    await until(()=>evaluate('game.frame_ready()'),'frame slot before hitch');
    await drive(100);
    const hitch=await bodyStatus();
    collect(hitch);
    const hitchAges=[...ages].filter(([key])=>!beforeHitch.has(key)).map(([,record])=>record);
    assert(hitchAges.length>0,'hitch did not exercise a delivered reply');
    reconcile(end,hitch,hitchAges);
    assert(hitch.unavailable>end.unavailable&&hitch.blockingReads===0,'hitch did not account for the unavailable replies');
    for(const status of [start,end,hitch]) delete status.replies;
    result.bodies={start,ages:[...ages.values()].slice(0,beforeHitch.size),end,hitch,hitchAges};
  } else if(mode==='surf'){
    const checkpoint=Math.max(surfClockOrigin+2000,now+250);
    const frame=await capture('surf-trade',checkpoint-now);
    assert(frame.tick>0,'surf field checkpoint did not advance');
    await inspectSurf(evaluate,result,frame);
  } else if(mode==='island'){
    await capture('island',10000-now);
    await inspectIsland(evaluate,result);
  } else if(mode==='first-frame'||mode==='frames'){
    const rustc=execFileSync('rustup',['which','--toolchain','stable','rustc'],{encoding:'utf8'}).trim();
    execFileSync('rustup',['run','stable','cargo','build','--release','--locked','--offline','-p','water-scene','--example','field_oracle'],{stdio:'inherit',env:{...process.env,PATH:path.dirname(rustc)+path.delimiter+process.env.PATH,RUSTC_WRAPPER:''}});
    await capture('first',0,true);
    if(mode==='frames'){
      await capture('between',1000/240,true);await capture('repeat',0,true);const beforeHide=await capture('next',1000/120,true);
      // Real tab visibility events enter the normal page listener. Only their wall-clock
      // input is controlled, keeping this scripted trace in the same origin as frame().
      await evaluate(`globalThis.__clock=${now};Object.defineProperty(performance,'now',{configurable:true,value:()=>__clock})`);
      const other=await cdp.call('Target.createTarget',{url:'about:blank'});
      await cdp.call('Target.activateTarget',{targetId:other.targetId});
      await until(()=>evaluate('document.hidden'),'tab hidden');
      await drive(5000);await evaluate(`__clock=${now}`);
      await cdp.call('Target.activateTarget',{targetId});
      await until(()=>evaluate('!document.hidden'),'tab shown');
      result.visibility='Real tab switch and visibilitychange listener; scripted performance.now at the two events';
      await cdp.call('Target.closeTarget',{targetId:other.targetId});
      await evaluate('delete performance.now');
      // C02 oracle: discard the old partial tick, then hold the committed endpoint
      // for a fresh full interval. These expectations do not use submitted alpha.
      const heldTime=beforeHide.tick/60;
      for(const [name,delta]of [['shown',1000/240],['shown-repeat',0],['shown-held',1000/120]]){
        const frame=await capture(name,delta,true);
        frame.expected_clock={tick:beforeHide.tick,alpha:1,display_time:heldTime};
        assert(frame.tick===beforeHide.tick&&frame.alpha===1,`${name}: resume must hold the committed endpoint`);
        assert(Math.abs(frame.fields.time-heldTime)<1e-9,`${name}: held display time differs`);
      }
      const committed=await capture('shown-commit',1000/120,true);
      committed.expected_clock={tick:beforeHide.tick+1,alpha:0.25,display_time:(beforeHide.tick+0.25)/60};
      assert(committed.tick===beforeHide.tick+1&&Math.abs(committed.alpha-0.25)<1e-5,'resume did not start a fresh full tick interval');
      assert(Math.abs(committed.fields.time-committed.expected_clock.display_time)<1e-7,'resumed display time differs from independent lifecycle oracle');
      await capture('hitch',1500,true);
      await evaluate('__devices.at(-1).destroy()');await delay(100);await drive(5);
      await evaluate('game.recover_graphics()');
      await until(async()=>{await drive(5);await delay(100);return /recovered=1/.test(await evaluate('game.frame_info()'));},'recovered device');
      for(let i=0;i<8;i++){await until(()=>evaluate('game.frame_ready()'),'replacement frame slot');await drive(1000/120);await delay(30);}
      await capture('recovered',1000/240,true);result.recovery=await evaluate('game.frame_info()');
      assert(result.frames[1].tick===result.frames[2].tick&&result.frames[1].alpha===result.frames[2].alpha,'repeat changed the model clock');

      // Fill this generation's 600 completion samples with normal, uncaptured draws.
      const before=frameInfo(result.recovery);
      const history=await evaluate(`(async()=>{
        let time=${now};
        const presented=()=>Number(/presented=(\\d+)/.exec(game.frame_info())[1]);
        const goal=presented()+605;
        const deadline=performance.now()+90000;
        while(presented()<goal){
          if(performance.now()>deadline)throw Error('completion history saturation timed out');
          if(game.frame_ready())game.frame(time+=1000/60);
          await new Promise(resolve=>setTimeout(resolve,0));
        }
        return {time,info:game.frame_info()};
      })()`);
      now=history.time;
      const after=frameInfo(history.info);
      assert(after.presented-before.presented===605,'completion history did not fill');
      assert(after.generation===before.generation&&after.submission-before.submission>=605&&after.tick>before.tick,
        'frame report stayed stale when completion history filled');
      const saturated=await capture('history-full',1000/240,true);
      assert(saturated.submission===after.submission+1,'post-saturation capture was not the next submission');
      result.frame_report_history={before,after};
    }
  } else if(opticsRoute || (diveRoute && mode==='fixed')){
    const routeParams=new URLSearchParams(route);
    if(!routeParams.has('bodies')){
      await until(async()=>{
        if(!await evaluate('game.frame_ready()'))return false;
        await drive(1000/60);
        const status=await evaluate('game.report()');
        if(status.startsWith('Asset load failed:')) throw Error(status);
        return /^Cooked generation [0-9]+ drawn/.test(status);
      },'optics content drawn');
    }
    let checkpoints;
    if(routeParams.has('dive')){
      checkpoints=[
        ['up-1m',{'camera.depth':1,'camera.pitch':89.99}],
        ['inclined-1m',{'camera.depth':1,'camera.pitch':45}],
        ['level-0.5m',{'camera.depth':0.5,'camera.pitch':0}],
        ['down-3m',{'camera.depth':3,'camera.pitch':-35}],
      ];
      if(routeParams.get('location')==='swash'){
        checkpoints=[
          ['above-swash',{'camera.depth':-0.35,'camera.pitch':0}],
          ['split-swash',{'camera.depth':0,'camera.pitch':0}],
          ['under-swash',{'camera.depth':0.25,'camera.pitch':25}],
        ];
      }else if(['sea','lake-high-stage'].includes(routeParams.get('location'))){
        checkpoints.push(
          ['up-5m',{'camera.depth':5,'camera.pitch':89.99}],
          ['inclined-5m',{'camera.depth':5,'camera.pitch':45}],
        );
        if(routeParams.get('location')==='sea')checkpoints.push(
          ['level-10m',{'camera.depth':10,'camera.pitch':0}],
          ['down-10m',{'camera.depth':10,'camera.pitch':-35}],
        );
      }
    }else if(routeParams.has('island')){
      checkpoints=[['beach-noon',{'camera.island':0}],['lake-noon',{'camera.island':1}],
        ['pond-noon',{'camera.island':2}],['lake-high-noon',{'camera.island':7}],['lake-shore-noon',{'camera.island':11}],
        ['beach-down-noon',{'camera.island':8}],['lake-down-noon',{'camera.island':9}],
        ['pond-down-noon',{'camera.island':10}]];
    }else if(routeParams.has('bodies')){
      checkpoints=[['hull-2',{'camera.optics_height':2,'sun.elevation':60}],
        ['hull-6',{'camera.optics_height':6,'sun.elevation':60}]];
    }else{
      checkpoints=[['noon',60],['low-sun',6],['night',-18]].flatMap(([name,sun])=>
        [2,40].map(height=>[name+'-'+height,{'camera.height':height,'camera.bearing':90,'camera.pitch':sun===60?-33.024:-18.004,'sun.elevation':sun}]));
      checkpoints.push(['submerged-noon-6',{'camera.height':6,'camera.bearing':90,'camera.pitch':-33.024,'sun.elevation':60}]);
    }
    for(const [name,params] of checkpoints){
      await evaluate(`game.apply_params(${JSON.stringify(JSON.stringify(params))})`);
      await capture(name,1000/60);
    }
  } else if(mode==='fixed' && surfInspection){
    const views=['sand','beach','offshore','lee','headland','island','seaward','handoff','approach'];
    const swellStills=process.env.THRUM_WATER_SWELL_STILLS!==undefined;
    for(const [index,name] of views.entries()){
      if(swellStills && index!==2 && index!==7)continue;
      await evaluate(`game.edit_control('camera.island',${index})`);
      for(let phase=0;phase<3;phase++){
        const seconds=swellStills?16+index*2.7+phase*(6.5/3):16+0.9*(3*index+phase);
        const timestamp=surfClockOrigin+seconds*1000;
        assert(timestamp>=now,'surf fixed checkpoint precedes the clock anchor');
        const frame=await capture(`surf-${name}-${phase}`,timestamp-now);
        assert(Math.abs(frame.observation-seconds)<0.00005,`${name}: surf checkpoint drifted`);
      }
    }
  } else if(mode==='fixed'){
    for(const [name,height,bearing]of [['noon-2-along',2,90],['noon-2-across',2,0],['noon-40-along',40,90],['noon-40-across',40,0]]){
      await evaluate(`game.apply_params(${JSON.stringify(JSON.stringify({'camera.height':height,'camera.bearing':bearing,'sun.elevation':60}))})`);
      await capture(name,10000-now);
    }
  } else {
    // Page's own RAF timestamps drive the ordinary scene loop; each image is the frame it submitted.
    const view=Number(process.env.THRUM_WATER_ISLAND_CAMERA??'0');
    const offshore=surfInspection&&view===2;
    const routeParams=new URLSearchParams(route);
    const dive=routeParams.has('dive');
    const diveDepth=routeParams.get('location')==='swash'?0.35:0.6;
    const diveView=dive?new Map([
      ['dive-up-1m',[1,89.99]],['dive-inclined-1m',[1,45]],
      ['dive-up-5m',[5,89.99]],['dive-inclined-5m',[5,45]],['dive-hold',[0,0]],
    ]).get(process.env.THRUM_WATER_ROUTE):undefined;
    const sweepDive=dive&&!diveView;
    const duration=offshore?22000:surfInspection?12000:dive?5000:1000;
    const spacing=offshore?200:surfInspection?100:dive?50:0;
    if(dive){
      const [depth,pitch]=diveView??[-diveDepth,0];
      result.dive_camera=diveView?{depth,pitch}:{sweep_depth:diveDepth,pitch};
      await evaluate(`game.edit_control('camera.pitch',${pitch});game.edit_control('camera.depth',${depth})`);
      await until(async()=>{
        const status=await evaluate('game.frame(performance.now());game.report()');
        return /^Cooked generation [0-9]+ drawn/.test(status);
      },'cooked dive scene drawn');
    }
    if(surfInspection){
      assert(Number.isInteger(view)&&view>=0&&view<=8,'invalid surf camera');
      if(handoffCut)assert(view===7,'start at the foam handoff camera');
      await evaluate(`game.edit_control('camera.island',${view})`);
      result.surf_camera=view;
      await until(async()=>{
        const status=await evaluate('game.frame(performance.now());game.report()');
        return /^Cooked generation [0-9]+ drawn/.test(status);
      },'cooked surf drawn');
      for(let frame=0;frame<8;frame++){
        await until(()=>evaluate('game.frame_ready()'),'free surf warm-up slot');
        await evaluate('game.frame(performance.now())');
      }
      await until(()=>evaluate('game.frame_ready()'),'surf warm-up completion');
    }
    await evaluate(`globalThis.__motion=[];globalThis.__intervals=[];globalThis.__motionDone=false;globalThis.__start=null;globalThis.__cameraCut=null;globalThis.__motionOmissions=null;
      let lastOpportunity=-Infinity;
      globalThis.__opportunities=0;const presented=()=>Number(/presented=(\\d+)/.exec(game.frame_info())[1]);
      globalThis.__firstPresented=presented();globalThis.__expected=0;
      const collect=()=>{let image;while((image=game.capture())){if(__motion.length>=120)throw Error('motion export reached its 120-image allowance');__motion.push(image);}};
      const drain=()=>{
        collect();
        if(__motion.length===__expected){
          __motionOmissions=game.capture_omissions();
          game.end_capture();
          __motionDone=true;
          return;
        }
        setTimeout(drain,5);
      };
      const loop=timestamp=>{
        __start??=timestamp;const elapsed=timestamp-__start;
        __intervals.push(timestamp);collect();
        if(${sweepDive})game.edit_control('camera.depth',${-diveDepth}*Math.cos(2*Math.PI*elapsed/${duration}));
        else if(!${dive}&&!${surfInspection})game.edit_control('camera.bearing',90+elapsed*0.012);
        if(${handoffCut}&&elapsed>=5000&&__cameraCut===null){
          game.edit_control('camera.island',8);
          __cameraCut={from:7,to:8,driver_elapsed:elapsed/1000,next_capture_index:presented()-__firstPresented};
        }
        if(elapsed-lastOpportunity>=${spacing}){
          game.frame(${surfInspection||dive?'timestamp':`${now}+elapsed`});__opportunities++;lastOpportunity=elapsed;
        }
        if(game.capture_omissions())throw Error('submitted motion observer omitted an image');
        if(elapsed<${duration})__requestCaptureFrame(loop);else {__expected=presented()-__firstPresented;drain();}
      };game.begin_capture();__requestCaptureFrame(loop);`);
    await until(()=>evaluate('__motionDone'),'realtime pan');
    if(handoffCut){
      result.camera_cut=await evaluate('__cameraCut');
      assert(result.camera_cut!==null,'capture did not reach the camera change');
    }
    const count=await evaluate('__motion.length');
    let clockOrigin=dive?undefined:now/1000;
    for(let i=0;i<count;i++){
      const image=await evaluate(`({identity:__motion[${i}].identity,frame:__motion[${i}].frame})`),rgba=await pixels(`__motion[${i}].rgba`);
      image.observation_seconds=Number(/observation=([0-9.]+)/.exec(image.frame)[1]);
      clockOrigin??=image.observation_seconds;
      image.elapsed=image.observation_seconds-clockOrigin;
      assert(rgba.length===width*height*4,'motion extent differs');
      const file=`motion-${String(i).padStart(4,'0')}.png`;await fs.writeFile(path.join(output,file),png(rgba,width,height));result.frames.push({...image,file,sha256_rgba:hash(rgba)});
    }
    const timestamps=await evaluate('__intervals');result.callback_intervals_ms=timestamps.slice(1).map((v,i)=>v-timestamps[i]);
    result.omissions=await evaluate('__motionOmissions');
    assert(result.omissions===0,'motion contains omissions');
    if(surfInspection){
      const first=result.frames[0]?.observation_seconds;
      assert(result.frames.every((frame,index)=>index===0||frame.observation_seconds>
        result.frames[index-1].observation_seconds),'surf presentation clock stalled');
      const minimumCoverage=offshore?20:10;
      assert(result.frames.at(-1)?.observation_seconds-first>=minimumCoverage,'surf motion has incomplete engine time coverage');
      for(const frame of result.frames)frame.elapsed=frame.observation_seconds-first;
    }
    const ids=result.frames.map(f=>Number(/submission=([0-9]+)/.exec(f.frame)[1]));
    assert(ids.length>1&&ids.every((v,i)=>i===0||v===ids[i-1]+1),'submitted motion IDs are not contiguous');
    assert(result.frames.every(f=>f.identity.includes('presented=true')),'motion contains an unpresented source');
    result.export_allowance_bytes=120*width*height*4;
    result.extent=[width,height];
    result.clock_scope='Page requestAnimationFrame timestamps; observer ON, export after the interval. Neither display swaps nor physical cadence.';
  }
  result.faults=[...cdp.faults,...await evaluate('__faults')];assert(result.faults.length===0,JSON.stringify(result.faults));
  assert(!(await evaluate('document.querySelector("#problems").textContent')),'page reports a problem');
  result.passed=true;
}catch(error){failure=error;result.passed=false;result.error=String(error);}
finally{
  if(cdp){try{await cdp.call('Browser.close');}catch{}cdp.socket.close();}chrome?.kill();
  try{process.kill(-command.pid,'SIGTERM');}catch{}await log.close();
  await fs.writeFile(path.join(output,'run.json'),JSON.stringify(result,null,2)+'\n');
}
console.log(JSON.stringify({passed:result.passed,output,error:result.error}));if(failure)process.exitCode=1;
