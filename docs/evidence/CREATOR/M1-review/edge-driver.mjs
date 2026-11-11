import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn, execSync } from 'node:child_process';
const out = path.resolve(process.argv[2]); await fs.mkdir(out,{recursive:true});
const delay = ms => new Promise(r=>setTimeout(r,ms));
async function until(fn,label,limit=240000){const end=Date.now()+limit;while(Date.now()<end){const r=await fn();if(r)return r;await delay(150);}throw Error('timeout '+label);}
const source=path.join(out,'assets'); await fs.cp('examples/starter/assets',source,{recursive:true});
const logf=await fs.open(path.join(out,'command.log'),'w');
const cmd=spawn('./dev',['starter','--web'],{env:{...process.env,THRUM_SCENE_ASSETS:source,THRUM_SCENE_OUTPUT:out},stdio:['ignore',logf.fd,logf.fd],detached:true});
const results={};
let chrome,socket;
try{
const url=await until(async()=>/SCENE_URL=(\S+)/.exec(await fs.readFile(path.join(out,'command.log'),'utf8'))?.[1],'url');
const profile=path.join(out,'profile');await fs.mkdir(profile);
chrome=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',[`--user-data-dir=${profile}`,'--headless=new','--remote-debugging-port=0','--no-first-run','--window-size=1280,1000','about:blank'],{stdio:'ignore'});
const ep=await until(async()=>{try{return (await fs.readFile(path.join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n')}catch{return null}},'devtools',20000);
socket=new WebSocket(`ws://127.0.0.1:${ep[0]}${ep[1]}`);await new Promise((r,j)=>{socket.onopen=r;socket.onerror=j});
let n=0;const pend=new Map();socket.onmessage=({data})=>{const p=JSON.parse(data);const q=pend.get(p.id);if(q){pend.delete(p.id);p.error?q.j(Error(JSON.stringify(p.error))):q.r(p.result)}};
const call=(m,params={},sid)=>new Promise((r,j)=>{const id=++n;pend.set(id,{r,j});socket.send(JSON.stringify({id,method:m,params,...(sid?{sessionId:sid}:{})}))});
const {targetId}=await call('Target.createTarget',{url:'about:blank'});
const sid=(await call('Target.attachToTarget',{targetId,flatten:true})).sessionId;
const ev=async e=>{const r=await call('Runtime.evaluate',{expression:e,awaitPromise:true,returnByValue:true},sid);if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value};
const shot=async name=>{const r=await call('Page.captureScreenshot',{format:'png'},sid);await fs.writeFile(path.join(out,name+'.png'),Buffer.from(r.data,'base64'))};
const st=()=>ev('document.querySelector("#status").textContent+" || problems: "+document.querySelector("#problems").textContent');
results.adapter=null;
await call('Page.navigate',{url},sid);
await until(async()=>/generation 1 drawn/.test(await st()),'gen1');
results.adapter=await ev('(async()=>{const a=await navigator.gpu.requestAdapter({powerPreference:"high-performance"});const i=a.info||{};return {vendor:i.vendor,architecture:i.architecture,description:i.description,isFallbackAdapter:i.isFallbackAdapter??a.isFallbackAdapter}})()');
results.s1_drawn=await st(); await shot('e1-drawn');
// slider via real pointer drag
const box=await ev('(()=>{const r=document.querySelector("#sun-elevation").getBoundingClientRect();return [r.left,r.top,r.width,r.height]})()');
const y=box[1]+box[3]/2;
await call('Input.dispatchMouseEvent',{type:'mousePressed',x:box[0]+box[2]*0.6,y,button:'left',clickCount:1},sid);
for(let k=0;k<10;k++){await call('Input.dispatchMouseEvent',{type:'mouseMoved',x:box[0]+box[2]*(0.6-0.05*k),y,button:'left',buttons:1},sid);await delay(30)}
await call('Input.dispatchMouseEvent',{type:'mouseReleased',x:box[0]+box[2]*0.1,y,button:'left',clickCount:1},sid);
await delay(500);
results.s2_drag=await ev('document.querySelector("#sun-elevation-value").textContent+" model="+JSON.parse(game.controls_json()).controls[0].value');
await shot('e2-dragged');
// malformed gltf (cooker refusal)
const gltf=path.join(source,'meshes/rock.gltf');const orig=await fs.readFile(gltf,'utf8');
await fs.writeFile(gltf,'{ not json');await delay(3000);
results.s3_malformed=await st(); await shot('e3-malformed');
await fs.writeFile(gltf,orig);await delay(3000);
results.s3b_restored=await st();
// rename that breaks placement, then fix
await fs.rename(gltf,gltf+'.renamed');
await until(async()=>/stays drawn/.test(await st()),'refusal');
results.s4_refused=await st(); await shot('e4-refused');
await fs.rename(gltf+'.renamed',gltf);
const before=await st();
await until(async()=>{const s=await st();return /generation (\d+) drawn/.exec(s) && s!==before && s},'fixed gen',30000).catch(e=>null);
await delay(2000);
results.s5_after_fix=await st(); await shot('e5-after-fix');
// store goes away while running: kill the watcher
execSync(`pkill -f "thrum-cook watch ${source}" || true`);
await delay(4000);
results.s6_store_gone=await st(); await shot('e6-store-gone');
}catch(e){results.error=String(e)}
finally{try{socket?.close()}catch{};chrome?.kill();try{process.kill(-cmd.pid,'SIGTERM')}catch{};await fs.writeFile(path.join(out,'edge.json'),JSON.stringify(results,null,2));console.log(JSON.stringify(results,null,2));process.exit(0)}
