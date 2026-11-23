#!/usr/bin/env node
// Drives the documented command and an owned Chrome on hardware WebGPU.
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import zlib from 'node:zlib';
const mode = process.argv[process.argv.indexOf('--case') + 1];
if (!['scene', 'controls', 'edit', 'journey', 'profiler', 'profiler-switch'].includes(mode)) throw Error('use --case scene|controls|edit|journey|profiler|profiler-switch');
const output = path.resolve(`tmp/creator-m1/${mode}-${Date.now()}`);
await fs.mkdir(output, { recursive:true });
const delay = ms => new Promise(r => setTimeout(r, ms));
const hash = data => createHash('sha256').update(data).digest('hex');
const assert = (ok, message) => {
  if (!ok) throw Error(message);
};
async function until(fn, label, limit = 180000) {
  const end = Date.now() + limit;
  while (Date.now() < end) {
    const result = await fn();
    if (result) return result;
    await delay(100);
  }
  throw Error(`Timed out waiting for ${label}`);
}
class Cdp {
  constructor(socket) {
    this.socket = socket;
    this.next = 0;
    this.pending = new Map();
    socket.onmessage = ({ data }) => {
      const packet = JSON.parse(data);
      const pending = this.pending.get(packet.id);
      if (pending) {
        this.pending.delete(packet.id);
        clearTimeout(pending.timer);
        packet.error ? pending.reject(Error(JSON.stringify(packet.error))) : pending.resolve(packet.result);
      }
    };
  }
  call(method, params = {}, sessionId) {
    return new Promise((resolve, reject) => {
      const id = ++this.next;
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(Error(`CDP timeout ${method}`));
      }, 15000);
      this.pending.set(id, { resolve, reject, timer });
      this.socket.send(JSON.stringify({
        id, method, params, ...(sessionId ? { sessionId } : {})
      }));
    });
  }
}
function png(rgba, width, height) {
  function chunk(type, bytes) {
    const body = Buffer.concat([Buffer.from(type), bytes]);
    let crc = ~0;
    for (const byte of body) {
      crc ^= byte;
      for (let k=0;k<8;k++) crc = crc & 1 ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1;
    }
    const out = Buffer.alloc(body.length+8);
    out.writeUInt32BE(bytes.length);
    body.copy(out,4);
    out.writeUInt32BE(~crc >>> 0, body.length+4);
    return out;
  }
  const header=Buffer.alloc(13);
  header.writeUInt32BE(width);
  header.writeUInt32BE(height,4);
  header[8]=8;
  header[9]=6;
  const rows=Buffer.concat(Array.from({ length:height },(_,y)=>Buffer.concat([Buffer.from([0]),rgba.subarray(y*width*4,(y+1)*width*4)])));
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),chunk('IDAT',zlib.deflateSync(rows)),chunk('IEND',Buffer.alloc(0))]);
}
const source = path.join(output,'assets');
await fs.cp('examples/starter/assets',source,{ recursive:true });
const serveLog = await fs.open(path.join(output,'command.log'),'w');
const command = spawn('./dev',['starter','--web'],{
  env:{ ...process.env,THRUM_SCENE_ASSETS:source,THRUM_SCENE_OUTPUT:output },stdio:['ignore',serveLog.fd,serveLog.fd],detached:true
});
let chrome, cdp, sid, failure;
const results = { mode,command:'./dev starter --web',output };
try {
  const url = await until(async()=> /SCENE_URL=(\S+)/.exec(await fs.readFile(path.join(output,'command.log'),'utf8'))?.[1], 'documented command URL',600000);
  results.url=url;
  results.build={
    revision:execFileSync('git',['rev-parse','HEAD'],{ encoding:'utf8' }).trim(),files:{}
  };
  for(const file of ['web/scene_bg.wasm','web/page.js','web/host-page.js','web/thrum-controls.js','store/HEAD']) results.build.files[file]=hash(await fs.readFile(path.join(output,file)));
  results.gameSource=hash(await fs.readFile('examples/starter/src/main.rs'));
  results.captureScope='Fixed still-scene checkpoints from the renderer submitted-output tap; DOM screenshots separately. Hardware WebGPU function only.';
  const profile=path.join(output,'chrome-profile');
  await fs.mkdir(profile);
  const log=await fs.open(path.join(output,'chrome.log'),'w');
  chrome=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',[`--user-data-dir=${profile}`,'--headless=new','--remote-debugging-port=0','--no-first-run','--no-default-browser-check','--window-size=1280,1000','about:blank'],{ stdio:['ignore',log.fd,log.fd] });
  const endpoint=await until(async()=>{
    try{
      return (await fs.readFile(path.join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n');
    }
    catch{
      return null;
    }
  },'Chrome DevTools',20000);
  const socket=new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);
  await new Promise((r,j)=>{
    socket.onopen=r;socket.onerror=j;
  });
  cdp=new Cdp(socket);
  const gpu=await cdp.call('SystemInfo.getInfo');
  results.gpu=gpu;
  results.browser=await cdp.call('Browser.getVersion');
  const devices=JSON.stringify(gpu.gpu.devices);
  assert(!/swiftshader|llvmpipe|lavapipe|software/i.test(devices) && /Apple M3 Max/i.test(devices) && /Metal/i.test(devices) && gpu.gpu.featureStatus.webgpu==='enabled',`Hardware WebGPU required: ${devices}`);
  const { targetId } = await cdp.call('Target.createTarget',{ url:'about:blank' });
  sid=(await cdp.call('Target.attachToTarget',{ targetId,flatten:true })).sessionId;
  await cdp.call('Page.enable',{},sid);
  await cdp.call('Page.addScriptToEvaluateOnNewDocument', {source: `
    globalThis.__timingWork = { resolves: 0, maps: 0, pairs: 0, submits: 0, supported: false };
    const resolve = GPUCommandEncoder.prototype.resolveQuerySet;
    GPUCommandEncoder.prototype.resolveQuerySet = function(...args) {
      if (args[0].label === 'thrum pass timestamps') __timingWork.resolves++;
      return resolve.apply(this, args);
    };
    const map = GPUBuffer.prototype.mapAsync;
    GPUBuffer.prototype.mapAsync = function(...args) {
      if (this.label === 'thrum timestamp readback') __timingWork.maps++;
      return map.apply(this, args);
    };
    for (const method of ['beginRenderPass', 'beginComputePass']) {
      const begin = GPUCommandEncoder.prototype[method];
      GPUCommandEncoder.prototype[method] = function(descriptor) {
        if (descriptor.timestampWrites) __timingWork.pairs++;
        return begin.call(this, descriptor);
      };
    }
    const submit = GPUQueue.prototype.submit;
    GPUQueue.prototype.submit = function(...args) {
      __timingWork.submits++;
      return submit.apply(this, args);
    };
    const requestDevice = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = async function(descriptor = {}) {
      if (location.search.includes('without-timestamps')) descriptor = {
        ...descriptor, requiredFeatures: [...(descriptor.requiredFeatures ?? [])].filter(f => !f.startsWith('timestamp-query')),
      };
      const device = await requestDevice.call(this, descriptor);
      __timingWork.supported = device.features.has('timestamp-query');
      (globalThis.__creatorDevices ??= []).push(device);
      return device;
    };
    const originalRequestAdapter = GPU.prototype.requestAdapter;
    GPU.prototype.requestAdapter = async function(...arguments_) {
      const adapter = await originalRequestAdapter.apply(this, arguments_);
      if (adapter) {
        // Exercise the real hardware path with timestamp queries denied at both
        // the exposed adapter and the enabled device, never with fabricated times.
        if (location.search.includes('without-timestamps')) Object.defineProperty(adapter, 'features', {
          value: new Set([...adapter.features].filter(f => !f.startsWith('timestamp-query'))),
        });
        const info = adapter.info;
        globalThis.__creatorSelectedAdapter = {
          vendor: info.vendor, architecture: info.architecture,
          isFallbackAdapter: info.isFallbackAdapter ?? adapter.isFallbackAdapter,
          timestampQuery: adapter.features.has('timestamp-query'),
        };
      }
      return adapter;
    };
  `}, sid);
  const evaluate=async expression=>{
    const r=await cdp.call('Runtime.evaluate',{ expression,awaitPromise:true,returnByValue:true },sid);
    if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));
    return r.result.value;
  };
  const screenshot=async name=>{
    const r=await cdp.call('Page.captureScreenshot',{ format:'png',captureBeyondViewport:true },sid);
    await fs.writeFile(path.join(output,`${name}.png`),Buffer.from(r.data,'base64'));
  };
  const drawn=()=>until(async()=>{
    const state=await evaluate('document.querySelector("#status")?.textContent'); if (/could not start|Scene frame:/.test(state)) throw Error(state);return /generation \d+ drawn/.test(state) && state;
  },'drawn cooked generation');
  await cdp.call('Page.navigate',{ url },sid);
  results.drawnStatus=await drawn();
  results.pageAdapter=await evaluate('globalThis.__creatorSelectedAdapter');
  assert(results.pageAdapter?.isFallbackAdapter===false && /apple/i.test(results.pageAdapter.vendor) && /metal/i.test(results.pageAdapter.architecture),
    `Running page did not receive the required hardware adapter: ${JSON.stringify(results.pageAdapter)}`);
  results.title=await evaluate('document.title');
  assert(results.title==='Thrum starter','page ignored the configured scene title');
  const capture=async name=>{
    await evaluate('game.begin_capture()');
    const image=await until(()=>evaluate('(()=>{const c=game.capture();return c?{rgba:btoa(Array.from({length:Math.ceil(c.rgba.length/8192)},(_,i)=>String.fromCharCode(...c.rgba.subarray(i*8192,(i+1)*8192))).join("")),identity:c.identity,width:document.querySelector("canvas").width,height:document.querySelector("canvas").height}:null})()'),'submitted image');
    await evaluate('game.end_capture()');
    assert(/presented=true/.test(image.identity),'capture did not belong to a presented submission');
    const rgba=Buffer.from(image.rgba,'base64');
    assert(rgba.length===image.width*image.height*4,'capture dimensions');
    await fs.writeFile(path.join(output,`${name}.png`),png(rgba,image.width,image.height));
    results[name]={ sha256:hash(rgba),identity:image.identity };
    return rgba;
  };
  const before=await capture('drawn');
  await screenshot('page');
  if(mode === 'profiler' || mode === 'profiler-switch') {
    const rows = () => evaluate('JSON.parse(game.controls_json()).controls.find(c=>c.key==="profiler.rows").value');
    const set = on => evaluate(`game.edit_control('profiler.enabled', ${on})`);
    const latest = async () => Number(/newest #(\d+)/.exec((await rows()).find(([label])=>label==='Window')?.[1])?.[1] ?? 0);
    const gpuRows = async () => (await rows()).filter(([label])=>label.startsWith('GPU '));
    await set(true);
    await until(async()=> (await latest()) > 650, 'live completed frame after 600', 180000);
    results.after600 = { newest:await latest(), rows:await rows(), work:await evaluate('__timingWork') };
    const supported = results.after600.work.supported;
    const numeric = rows => rows.some(([,text]) => /\d.*ms/.test(text));
    assert(supported ? numeric(await gpuRows()) : (await gpuRows()).every(([,text])=>text.includes('no timestamp queries')), 'GPU values contradict adapter features');
    await screenshot('profiler-wide');
    if(mode === 'profiler') {
      // A real geometry load change after frame 600, large enough for coarse timestamps.
      // Canvas size changes only the presentation surface, not the fixed render extent.
      const baseline = await gpuRows();
      const boundary = await latest();
      const gltf = path.join(source, 'meshes/rock.gltf');
      const original = await fs.readFile(gltf, 'utf8');
      const loadedMesh = JSON.parse(original);
      const originalBuffer = await fs.readFile(path.join(path.dirname(gltf), loadedMesh.buffers[0].uri));
      const position = loadedMesh.accessors[0];
      const sourceView = loadedMesh.bufferViews[position.bufferView];
      const positions = originalBuffer.subarray(sourceView.byteOffset, sourceView.byteOffset + sourceView.byteLength);
      const buffers = [originalBuffer];
      const primitive = loadedMesh.meshes[0].primitives[0];
      loadedMesh.meshes[0].primitives = [];
      let offset = originalBuffer.length;
      const layers = 4096;
      for (let layer = 0; layer < layers; layer++) {
        const bytes = Buffer.from(positions);
        const shift = (layer + 1) * 0.0001;
        for (let vertex = 0; vertex < position.count; vertex++) bytes.writeFloatLE(bytes.readFloatLE(vertex * 12 + 8) + shift, vertex * 12 + 8);
        const view = loadedMesh.bufferViews.length;
        loadedMesh.bufferViews.push({buffer:0,byteOffset:offset,byteLength:bytes.length,target:34962});
        const accessor = loadedMesh.accessors.length;
        loadedMesh.accessors.push({...position,bufferView:view,min:position.min.map((v,i)=>v+(i===2?shift:0)),max:position.max.map((v,i)=>v+(i===2?shift:0))});
        loadedMesh.meshes[0].primitives.push({...primitive,attributes:{...primitive.attributes,POSITION:accessor}});
        buffers.push(bytes);
        offset += bytes.length;
      }
      const binary = Buffer.concat(buffers);
      loadedMesh.buffers[0] = {uri:`data:application/octet-stream;base64,${binary.toString('base64')}`,byteLength:binary.length};
      await fs.writeFile(gltf, JSON.stringify(loadedMesh));
      await until(async()=> /generation 2 drawn/.test(await evaluate('document.querySelector("#status").textContent')), 'changed geometry workload drawn');
      const changedBoundary = await latest();
      await until(async()=> (await latest()) > changedBoundary + 150, 'recent window replaced after load change', 180000);
      const loaded = await gpuRows();
      const changed = await capture('profiler-load');
      assert(hash(changed)!==hash(before), 'load change did not change submitted scene pixels');
      results.load = { baseline, loaded, boundary, changedBoundary, newest:await latest(), edit:`${layers} independently optimized rock shells; ${position.count * layers} source vertices` };
      if(supported) {
        const number = text => Number(/([\d.]+) ms/.exec(text)?.[1] ?? NaN);
        const forward = rows => number(rows.find(([name])=>/forward/.test(name))?.[1] ?? '');
        assert(forward(loaded) > forward(baseline), `forward pass did not respond to added geometry: ${JSON.stringify(results.load)}`);
      }
      await fs.writeFile(gltf, original);
      await until(async()=> /generation 3 drawn/.test(await evaluate('document.querySelector("#status").textContent')), 'restored mesh drawn');
      const restoredBoundary = await latest();
      await until(async()=> (await latest()) > restoredBoundary + 130, 'restored recent window');
    }
    await cdp.call('Emulation.setDeviceMetricsOverride',{ width:480,height:900,deviceScaleFactor:1,mobile:false },sid);
    await delay(500);
    await screenshot('profiler-narrow');
    assert(await evaluate('document.documentElement.scrollWidth<=480'), 'profiler table overflows narrow page');
    const oldNewest = await latest();
    await set(false);
    await delay(1500); // more than the two in-flight batches and the readout refresh
    results.off = { rows:await rows(), work:await evaluate('__timingWork') };
    assert((await gpuRows()).every(([,text])=>text==='profiler off'), 'off rows retained numeric GPU values');
    await screenshot('profiler-off');
    await delay(1000);
    const settled = await evaluate('__timingWork');
    for(const key of ['resolves','maps','pairs']) assert(settled[key] === results.off.work[key], `off kept doing timestamp ${key}`);
    assert(settled.submits > results.off.work.submits, 'frames stopped submitting while profiler off');
    if(supported) assert(results.off.work.resolves > 0 && results.off.work.maps > 0 && results.off.work.pairs > 0, 'observer did not see real timestamp work while on');
    await capture('profiler-off-submitted'); // also asserts the real sample's presented=true
    const offSubmission = Number(/submission=(\d+)/.exec(results['profiler-off-submitted'].identity)?.[1]);
    assert(offSubmission > oldNewest, 'off capture reused a pre-switch submission');
    const offCaptured = await evaluate('__timingWork');
    for(const key of ['resolves','maps','pairs']) assert(offCaptured[key] === settled[key], `off presentation resumed timestamp ${key}`);
    await set(true);
    const immediate = await rows();
    assert(!numeric(immediate.filter(([label])=>label.startsWith('GPU '))), 'switch-on reused pre-switch numbers');
    await until(async()=> (await latest()) > oldNewest + 10, 'only post-switch completed samples');
    results.onAgain = { rows:await rows(), newest:await latest(), work:await evaluate('__timingWork') };
    if(supported) assert(results.onAgain.work.resolves > settled.resolves, 'switch-on did not resume real timestamp work');
    if(mode === 'profiler') {
      await evaluate('game.set_visible(false, performance.now())');
      results.hidden = await rows();
      assert((await gpuRows()).every(([,text])=>text.includes('no recent completed frame')), 'hidden page kept old GPU values');
      await evaluate('game.set_visible(true, performance.now())');
      await until(async()=> (await latest()) > results.onAgain.newest + 10, 'live frames after visibility restore');
      await evaluate('__creatorDevices.at(-1).destroy()');
      // The kit's existing device-loss policy asks the creator to reload. It does
      // not request detached replacement automatically; generation clearing is
      // covered by the shared window's replacement test.
      await until(async()=> (await gpuRows()).every(([,text])=>text.includes('no recent completed frame')), 'lost device expires GPU rows');
      results.lost = await rows();
      await cdp.call('Page.navigate', {url:url+'?without-timestamps'}, sid);
      await drawn();
      await set(true);
      await until(async()=> (await latest()) > 30, 'no-timestamp device completed frames');
      results.unsupported = { rows:await rows(), work:await evaluate('__timingWork'), adapter:await evaluate('__creatorSelectedAdapter') };
      assert(!results.unsupported.adapter.timestampQuery, 'no-timestamp adapter still exposed queries');
      assert(!results.unsupported.work.supported, 'no-timestamp device still enabled queries');
      assert((await gpuRows()).every(([,text])=>text.includes('no timestamp queries')), 'no-feature device showed numeric GPU time');
      assert(results.unsupported.work.resolves===0 && results.unsupported.work.maps===0 && results.unsupported.work.pairs===0, 'no-feature device wrote timestamps');
    }
  }
  if(mode==='scene') {
    await evaluate('(()=>{const input=document.querySelector("#sun-elevation");input.value=3;input.dispatchEvent(new Event("input",{bubbles:true}));})()');
    await delay(500);
    const after=await capture('sun-edit');
    let changed=0;
    for(let i=0;i<before.length;i+=4)if(Math.abs(before[i]-after[i])+Math.abs(before[i+1]-after[i+1])+Math.abs(before[i+2]-after[i+2])>10)changed++;
    assert(changed>1000,`Sun edit changed only ${changed} pixels`);
    results.changedPixels=changed;
    // Removing the rock through a refused publication must never silently switch to a diagnostic scene.
    assert(/Cooked generation/.test(results.drawnStatus),'cooked draw missing');
    await evaluate('game.apply_params(JSON.stringify({"sun.elevation":25}))');
    await delay(500);
  }
  if(mode==='controls') {
    const model=JSON.parse(await evaluate('game.controls_json()'));
    results.model=model;
    const dom=await evaluate('Array.from(document.querySelectorAll("[data-control]")).map(e=>({key:e.dataset.control,label:e.querySelector("span").textContent,text:e.querySelector("output")?.textContent}))');
    assert(JSON.stringify(dom.map(c=>c.key))===JSON.stringify(model.controls.map(c=>c.key)),'control keys differ');
    for(const c of model.controls){
      const row=dom.find(d=>d.key===c.key);
      assert(row.label===c.label && (!row.text || row.text===c.text),`control ${c.key} display differs`);
    }
    // Interactive values clamp and snap through the same setters as the native panel.
    await evaluate('game.edit_control("sun.elevation", 9999)');
    assert(JSON.parse(await evaluate('game.controls_json()')).controls.find(c=>c.key==='sun.elevation').value===60,'interactive model clamp missing');
    await evaluate('game.edit_control("sun.elevation", 25)');
    results.refusals=await evaluate('game.apply_params(JSON.stringify({"sun.elevaton":4,"sun.azimuth":9999}))');
    assert(/sun.elevaton/.test(results.refusals)&&/sun.azimuth/.test(results.refusals),'per-key refusals missing');
    const params=path.join(output,'web/check.params.json');
    await fs.writeFile(params,JSON.stringify({ 'sun.elevation':7,'sun.elevaton':3 }));
    await cdp.call('Page.navigate',{ url:url+'?params=check.params.json' },sid);
    await drawn();
    results.params=await until(()=>evaluate('document.querySelector("#problems").textContent'),'parameter refusal');
    assert(/sun.elevaton/.test(results.params),'params typo missing');
    assert((await evaluate('JSON.parse(game.controls_json()).controls.find(c=>c.key==="sun.elevation").value'))===7,'valid params not applied');
    await screenshot('params');
  }
  if(mode==='edit' || mode==='scene') {
    await evaluate('(()=>{globalThis.__creatorReloadWitness=game;return true})()');
    const gltf=path.join(source,'meshes/rock.gltf');
    const original=await fs.readFile(gltf,'utf8');
    const value=JSON.parse(original);
    // Move all vertex positions through the authored node, producing a distinct cooked geometry.
    value.nodes[0].scale=[1.8,1.8,1.8];
    await fs.writeFile(gltf,JSON.stringify(value));
    results.edited=await until(async()=>{
      const t=await evaluate('document.querySelector("#status").textContent');return /generation 2 drawn/.test(t)&&t;
    },'recooked generation');
    const after=await capture('asset-edit');
    assert(hash(before)!==hash(after),'asset edit pixels unchanged');
    const texture=Buffer.alloc(4*4*4);
    for(let i=0;i<texture.length;i+=4){
      texture[i]=255;
      texture[i+1]=30;
      texture[i+2]=20;
      texture[i+3]=255;
    }
    await fs.writeFile(path.join(source,'textures/white.png'),png(texture,4,4));
    results.texture=await until(async()=>{
      const t=await evaluate('document.querySelector("#status").textContent');return /generation 3 drawn/.test(t)&&t;
    },'texture recook');
    const textured=await capture('texture-edit');
    assert(hash(after)!==hash(textured),'texture edit pixels unchanged');
    await fs.rename(gltf,gltf+'.removed');
    results.refused=await until(async()=>{
      const t=await evaluate('document.querySelector("#status").textContent');return /generation 3 drawn/.test(t)&&/rock|mesh/.test(t)&&/fix/.test(t)&&t;
    },'refused placement retaining generation');
    const kept=await capture('asset-refused');
    assert(hash(textured)===hash(kept),'refused edit replaced last good image');
    assert(await evaluate('globalThis.__creatorReloadWitness===game'),'asset edits reloaded the page or replaced its running scene');
    results.samePageAndScene=true;
    await screenshot('refused');
    await fs.rename(gltf+'.removed',gltf);
    results.restored=await until(async()=>{
      const status=await evaluate('document.querySelector("#status").textContent');
      return /generation 5 drawn/.test(status) && !/stays drawn|fix the named asset/.test(status) && status;
    },'fixed generation with refusal cleared',15000);
    const restored=await capture('asset-restored');
    assert(hash(restored)===hash(textured),'fixed edit did not restore the pre-refusal pixels');
    assert(await evaluate('globalThis.__creatorReloadWitness===game'),'fixing an edit replaced the running page or scene');
    await screenshot('restored');
  }
  if(mode==='journey') {
    const other=await cdp.call('Target.createTarget',{ url:'about:blank',background:true });
    await cdp.call('Target.activateTarget',{ targetId:other.targetId });
    await until(()=>evaluate('document.hidden'),'page hidden by tab switch');
    await cdp.call('Target.activateTarget',{ targetId });
    await until(()=>evaluate('!document.hidden'),'page visible after tab switch');
    results.tabVisibility=[true,false];
    await drawn();
    await screenshot('tab-return');
    const returned=await capture('tab-return-canvas');
    assert(hash(returned)===hash(before),'tab return changed the still scene');
    await cdp.call('Emulation.setDeviceMetricsOverride',{ width:480,height:900,deviceScaleFactor:1,mobile:false },sid);
    await delay(200);
    await screenshot('narrow');
    assert(await evaluate('document.documentElement.scrollWidth<=480'),'narrow page overflows');
    await fs.writeFile(path.join(output,'web/typo.params.json'),JSON.stringify({ 'sun.elevaton':5 }));
    await cdp.call('Page.navigate',{ url:url+'?params=typo.params.json' },sid);
    await drawn();
    await until(()=>evaluate('document.querySelector("#problems").textContent'),'typo');
    await screenshot('typo');
    await cdp.call('Page.navigate',{ url:url+'?store=http://127.0.0.1:1' },sid);
    await until(()=>evaluate('document.querySelector("#status").textContent.includes("could not start")'),'missing store');
    assert((await evaluate('document.querySelector("#status").textContent')).length<500,'store diagnostic buries next step');
    await screenshot('missing-store');
    await cdp.call('Network.enable',{
    },sid);
    await cdp.call('Network.emulateNetworkConditions',{ offline:false,latency:2000,downloadThroughput:500000,uploadThroughput:500000 },sid);
    await cdp.call('Page.navigate',{ url },sid);
    await delay(2300);
    await screenshot('loading');
    await cdp.call('Network.emulateNetworkConditions',{ offline:false,latency:0,downloadThroughput:-1,uploadThroughput:-1 },sid);
    await drawn();
    await cdp.call('Target.closeTarget',{ targetId:other.targetId });
  }
  results.passed=true;
}
catch(error) {
  failure=error;
  results.passed=false;
  results.error=String(error);
}
finally {
  if(cdp) {
    try{
      await cdp.call('Browser.close');
    }
    catch{
    }
    cdp.socket.close();
  }
  chrome?.kill();
  try{
    process.kill(-command.pid,'SIGTERM');
  }
  catch{
  }
  await serveLog.close();
  await fs.writeFile(path.join(output,'run.json'),JSON.stringify(results,null,2)+'\n');
}
console.log(JSON.stringify({ passed:results.passed,output,error:results.error }));
if(failure)process.exitCode=1;
