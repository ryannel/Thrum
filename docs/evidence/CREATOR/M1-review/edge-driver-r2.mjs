// Round-2 reviewer edge driver: refuse/fix/refuse cycles, a second refusal while broken,
// a long refusal at 480 px, a malformed glTF and the watcher stopping. Run from the repo root:
//   node docs/evidence/CREATOR/M1-review/edge-driver-r2.mjs tmp/creator-m1-review/edge
import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawn, execSync } from 'node:child_process';
const out = path.resolve(process.argv[2]); await fs.mkdir(out,{recursive:true});
const shots = path.resolve(process.argv[3] ?? out);
const delay = ms => new Promise(r=>setTimeout(r,ms));
async function until(fn,label,limit=240000){const end=Date.now()+limit;while(Date.now()<end){const r=await fn();if(r)return r;await delay(150);}throw Error('timeout '+label);}
const source=path.join(out,'assets'); await fs.cp('examples/starter/assets',source,{recursive:true});
const logf=await fs.open(path.join(out,'command.log'),'w');
const cmd=spawn('./dev',['starter','--web'],{env:{...process.env,THRUM_SCENE_ASSETS:source,THRUM_SCENE_OUTPUT:out},stdio:['ignore',logf.fd,logf.fd],detached:true});
const results={};
let chrome,socket;
try{
const url=await until(async()=>/SCENE_URL=(\S+)/.exec(await fs.readFile(path.join(out,'command.log'),'utf8'))?.[1],'url',600000);
results.url=url;
const profile=path.join(out,'profile');await fs.mkdir(profile);
chrome=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',[`--user-data-dir=${profile}`,'--headless=new','--remote-debugging-port=0','--no-first-run','--window-size=1280,1000','about:blank'],{stdio:'ignore'});
const ep=await until(async()=>{try{return (await fs.readFile(path.join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n')}catch{return null}},'devtools',20000);
socket=new WebSocket(`ws://127.0.0.1:${ep[0]}${ep[1]}`);await new Promise((r,j)=>{socket.onopen=r;socket.onerror=j});
let n=0;const pend=new Map();socket.onmessage=({data})=>{const p=JSON.parse(data);const q=pend.get(p.id);if(q){pend.delete(p.id);p.error?q.j(Error(JSON.stringify(p.error))):q.r(p.result)}};
const call=(m,params={},sid)=>new Promise((r,j)=>{const id=++n;pend.set(id,{r,j});socket.send(JSON.stringify({id,method:m,params,...(sid?{sessionId:sid}:{})}))});
const {targetId}=await call('Target.createTarget',{url:'about:blank'});
const sid=(await call('Target.attachToTarget',{targetId,flatten:true})).sessionId;
await call('Page.addScriptToEvaluateOnNewDocument',{source:`(()=>{const o=GPU.prototype.requestAdapter;GPU.prototype.requestAdapter=async function(...a){const ad=await o.apply(this,a);if(ad){const i=ad.info;globalThis.__adapter={vendor:i.vendor,architecture:i.architecture,isFallbackAdapter:i.isFallbackAdapter??ad.isFallbackAdapter??null}}return ad}})()`},sid);
const ev=async e=>{const r=await call('Runtime.evaluate',{expression:e,awaitPromise:true,returnByValue:true},sid);if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value};
const shot=async name=>{const r=await call('Page.captureScreenshot',{format:'png'},sid);await fs.writeFile(path.join(shots,name+'.png'),Buffer.from(r.data,'base64'))};
const st=()=>ev('document.querySelector("#status").textContent');
const pix=async()=>{await ev('game.begin_capture()');const h=await until(()=>ev('(()=>{const c=game.capture();if(!c)return null;let x=0;for(let i=0;i<c.rgba.length;i+=97)x=(x*31+c.rgba[i])>>>0;return x+":"+c.rgba.length})()'),'capture',20000);await ev('game.end_capture()');return h};
await call('Page.navigate',{url},sid);
await until(async()=>/generation 1 drawn/.test(await st()),'gen1');
results.adapter=await ev('globalThis.__adapter');
results.a_drawn=await st(); results.a_pix=await pix(); await shot('r2-a-drawn');
const gltf=path.join(source,'meshes/rock.gltf');
const tex=(await fs.readdir(path.join(source,'textures')))[0]; results.texture=tex;
const texPath=path.join(source,'textures',tex);
// 1 refuse
await fs.rename(gltf,gltf+'.gone');
await until(async()=>/stays drawn/.test(await st()),'refusal 1',30000);
results.b_refused=await st(); results.b_pix=await pix(); await shot('r2-b-refused');
// 2 a second save while the mesh is still missing (touch the texture)
const tbytes=await fs.readFile(texPath); await fs.writeFile(texPath,tbytes);
await delay(4000);
results.c_second_save=await st(); await shot('r2-c-second-save');
// 3 fix
await fs.rename(gltf+'.gone',gltf);
await until(async()=>{const s=await st();return !/stays drawn/.test(s)&&/drawn/.test(s)&&s},'fix 1',30000).catch(()=>null);
await delay(1500);
results.d_fixed=await st(); results.d_pix=await pix(); await shot('r2-d-fixed');
// 4 refuse again after a fix: the message must come back
await fs.rename(gltf,gltf+'.gone');
await until(async()=>/stays drawn/.test(await st()),'refusal 2',30000).catch(()=>null);
results.e_refused_again=await st(); await shot('r2-e-refused-again');
// 5 the long refusal at 480 px
await call('Emulation.setDeviceMetricsOverride',{width:480,height:900,deviceScaleFactor:1,mobile:false},sid);
await delay(800);
results.e_narrow_overflow=await ev('document.documentElement.scrollWidth>innerWidth');
await shot('r2-e-refused-480');
await call('Emulation.clearDeviceMetricsOverride',{},sid);
await fs.rename(gltf+'.gone',gltf);
await until(async()=>{const s=await st();return !/stays drawn/.test(s)&&s},'fix 2',30000).catch(()=>null);
await delay(1500);
results.f_fixed_again=await st(); results.f_pix=await pix(); await shot('r2-f-fixed-again');
// 6 malformed glTF while running (cooker refuses before publishing)
const orig=await fs.readFile(gltf,'utf8');
await fs.writeFile(gltf,'{ not json'); await delay(4000);
results.g_malformed=await st(); await shot('r2-g-malformed');
await fs.writeFile(gltf,orig); await delay(4000);
results.g_restored=await st();
// 7 the watcher stops while the page runs
execSync(`pkill -f "thrum-cook watch ${source}" || true`);
await delay(5000);
results.h_store_gone=await st(); results.h_pix=await pix(); await shot('r2-h-store-gone');
}catch(e){results.error=String(e)}
finally{try{socket?.close()}catch{};chrome?.kill();try{process.kill(-cmd.pid,'SIGTERM')}catch{};await fs.writeFile(path.join(shots,'edge-run-r2.json'),JSON.stringify(results,null,2));console.log(JSON.stringify(results,null,2));process.exit(0)}
