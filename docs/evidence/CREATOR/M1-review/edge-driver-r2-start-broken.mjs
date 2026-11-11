// Round-2 reviewer edge: the scene starts with its mesh missing; the creator restores it while the
// page runs. Run from the repo root:
//   node docs/evidence/CREATOR/M1-review/edge-driver-r2-start-broken.mjs tmp/creator-m1-review/start-broken docs/evidence/CREATOR/M1-review
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
const out = path.resolve(process.argv[2]); await fs.mkdir(out,{recursive:true});
const shots = path.resolve(process.argv[3] ?? out);
const delay = ms => new Promise(r=>setTimeout(r,ms));
async function until(fn,label,limit=240000){const end=Date.now()+limit;while(Date.now()<end){const r=await fn();if(r)return r;await delay(150);}throw Error('timeout '+label);}
const source=path.join(out,'assets'); await fs.cp('examples/starter/assets',source,{recursive:true});
const gltf=path.join(source,'meshes/rock.gltf'); await fs.rename(gltf,gltf+'.gone');
const logf=await fs.open(path.join(out,'command.log'),'w');
const cmd=spawn('./dev',['starter','--web'],{env:{...process.env,THRUM_SCENE_ASSETS:source,THRUM_SCENE_OUTPUT:out},stdio:['ignore',logf.fd,logf.fd],detached:true});
const results={};
let chrome,socket;
try{
const url=await until(async()=>/SCENE_URL=(\S+)/.exec(await fs.readFile(path.join(out,'command.log'),'utf8'))?.[1],'url',600000);
const profile=path.join(out,'profile');await fs.mkdir(profile);
chrome=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',[`--user-data-dir=${profile}`,'--headless=new','--remote-debugging-port=0','--no-first-run','--window-size=1280,1000','about:blank'],{stdio:'ignore'});
const ep=await until(async()=>{try{return (await fs.readFile(path.join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n')}catch{return null}},'devtools',20000);
socket=new WebSocket(`ws://127.0.0.1:${ep[0]}${ep[1]}`);await new Promise((r,j)=>{socket.onopen=r;socket.onerror=j});
let n=0;const pend=new Map();socket.onmessage=({data})=>{const p=JSON.parse(data);const q=pend.get(p.id);if(q){pend.delete(p.id);p.error?q.j(Error(JSON.stringify(p.error))):q.r(p.result)}};
const call=(m,params={},sid)=>new Promise((r,j)=>{const id=++n;pend.set(id,{r,j});socket.send(JSON.stringify({id,method:m,params,...(sid?{sessionId:sid}:{})}))});
const {targetId}=await call('Target.createTarget',{url:'about:blank'});
const sid=(await call('Target.attachToTarget',{targetId,flatten:true})).sessionId;
const ev=async e=>{const r=await call('Runtime.evaluate',{expression:e,awaitPromise:true,returnByValue:true},sid);if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value};
const shot=async name=>{const r=await call('Page.captureScreenshot',{format:'png'},sid);await fs.writeFile(path.join(shots,name+'.png'),Buffer.from(r.data,'base64'))};
const st=()=>ev('document.querySelector("#status")?.textContent ?? document.body.innerText');
await call('Page.navigate',{url},sid);
await delay(8000);
results.a_start_broken=await st(); await shot('r2-i-start-broken');
await fs.rename(gltf+'.gone',gltf);
await until(async()=>/drawn/.test(await st()),'drawn after fix',30000).catch(()=>null);
await delay(1500);
results.b_after_fix=await st(); await shot('r2-j-start-broken-fixed');
results.reloads=await ev('performance.getEntriesByType("navigation").length');
}catch(e){results.error=String(e)}
finally{try{socket?.close()}catch{};chrome?.kill();try{process.kill(-cmd.pid,'SIGTERM')}catch{};await fs.writeFile(path.join(shots,'edge-run-r2-start-broken.json'),JSON.stringify(results,null,2));console.log(JSON.stringify(results,null,2));process.exit(0)}
