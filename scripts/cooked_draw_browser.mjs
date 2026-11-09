// One owned Chrome session, real hardware WebGPU, exact T2 submitted RGBA bytes.
import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn,execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {Cdp,serve,hardware,waitFor,cleanup} from './check_v1_browser.mjs';
const [output,store]=process.argv.slice(2);
const sha=raw=>createHash('sha256').update(raw).digest('hex');
const files=new Map();
for(const arm of ['base','claim']) for(const name of ['v1_game.js','v1_game_bg.wasm']) {
 const bytes=await fs.readFile(path.join(output,`${arm}-target/v1-game-web`,name));
 files.set(`/${arm}/${name}`,{bytes,type:name.endsWith('.wasm')?'application/wasm':'text/javascript'});
}
files.set('/probe.html',{bytes:Buffer.from('<!doctype html><canvas></canvas><script type="module" src="/probe.mjs"></script>'),type:'text/html'});
files.set('/probe.mjs',{bytes:await fs.readFile('scripts/cooked_draw_browser_page.mjs'),type:'text/javascript'});
files.set('/head',{bytes:await fs.readFile(path.join(store,'HEAD')),type:'application/octet-stream'});
for(const prefix of await fs.readdir(path.join(store,'cas'))) for(const name of await fs.readdir(path.join(store,'cas',prefix)))
 files.set(`/cas/${name}`,{bytes:await fs.readFile(path.join(store,'cas',prefix,name)),type:'application/octet-stream'});
const profile=await fs.mkdtemp(path.join(output,'chrome-profile-'));
const bundle=path.join(output,'ThrumCapture.app');
const launcher=path.join(bundle,'Contents/MacOS/Launcher');
await fs.mkdir(path.dirname(launcher),{recursive:true});
await fs.writeFile(path.join(bundle,'Contents/Info.plist'),'<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>org.thrum.capture</string><key>CFBundleName</key><string>Thrum Capture</string><key>CFBundleExecutable</key><string>Launcher</string><key>CFBundlePackageType</key><string>APPL</string><key>LSUIElement</key><true/></dict></plist>');
execFileSync('/usr/bin/swiftc',['-module-cache-path',path.join(output,'swift-cache'),'scripts/capture_browser_launcher.swift','-o',launcher]);
execFileSync('/usr/bin/codesign',['--force','--sign','-',bundle]);
const report={pass:false,views:[],driver_sha256:sha(await fs.readFile('scripts/cooked_draw_browser_page.mjs')),artifacts:Object.fromEntries([...files].filter(([key])=>/wasm|\.js$/.test(key)).map(([key,val])=>[key,sha(val.bytes)]))};
let child,cdp,server,ended=false,failure,current,received,pending;
try {
 const serving=await serve(files,(request,response)=>{
  if(request.url!='/capture') return false;
  pending=(async()=>{
   const chunks=[];for await(const chunk of request)chunks.push(chunk);
   const raw=Buffer.concat(chunks);
   if(raw.length!==1920*1080*4)throw Error('incorrect export size');
   const row=JSON.parse(request.headers['x-frame']);
   await fs.writeFile(path.join(output,`${current}.rgba`),raw,{flag:'wx'});
   received={...row,rgba_sha256:sha(raw)};
   response.writeHead(200).end();
  })().catch(error=>{failure=error;response.writeHead(500).end(String(error));});
  return true;
 });
 server=serving.server;report.origin=serving.origin;
 const flags=[`--user-data-dir=${profile}`,'--remote-debugging-port=0','--headless=new','--no-first-run','--no-default-browser-check','--disable-background-networking','--disable-component-update','--window-size=1920,1200','about:blank'];
 report.flags=flags;
 child=spawn('/usr/bin/open',['-n','-W',bundle,'--args',output,'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',...flags],{stdio:'ignore'});
 child.once('exit',()=>{ended=true;});
 const endpoint=await waitFor(async()=>{if(ended)throw Error('Chrome exited');try{return (await fs.readFile(path.join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n');}catch(error){if(error.code!=='ENOENT')throw error;}},15000,'Chrome DevTools');
 const socket=new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);
 await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
 cdp=new Cdp(socket);
 report.browser=await cdp.call('Browser.getVersion');report.gpu=await cdp.call('SystemInfo.getInfo');hardware(report.gpu);
 for(const [name,yaw,solar] of [['noon-0',0,12],['noon-18',18,12],['noon-36',36,12],['low-sun',0,6.5]]) {
  const arms={};
  for(const arm of ['base','claim','claim-repeat','base-repeat']) {
   current=`${name}-${arm}`;received=undefined;
   const {targetId}=await cdp.call('Target.createTarget',{url:'about:blank'});
   const {sessionId}=await cdp.call('Target.attachToTarget',{targetId,flatten:true});
   await cdp.call('Page.enable',{},sessionId);
   await cdp.call('Page.navigate',{url:`${serving.origin}/probe.html?arm=${arm.split('-')[0]}&yaw=${yaw}&solar=${solar}`},sessionId);
   const state=await waitFor(async()=>{const result=await cdp.call('Runtime.evaluate',{expression:'window.cookedDraw?.done ? JSON.stringify(window.cookedDraw) : null',returnByValue:true},sessionId);return result.result.value&&JSON.parse(result.result.value);},90000,current);
   await pending;
   if(state.error||failure||!received)throw failure??Error(state.error??'missing export');
   if(arm.startsWith('claim')&&(!received.status.initialDownlevel.includes('INDIRECT_EXECUTION')||received.status.drawPath!=='Some(Indirect)'))throw Error('claim did not select adapter-backed indirect');
   if(received.status.errors.length || received.status.drawn!==1)throw Error('cooked error or undrawn set');
   arms[arm]=received;
   await cdp.call('Target.closeTarget',{targetId});
  }
  const exact=Object.values(arms).every(row=>row.rgba_sha256===arms.base.rgba_sha256);
  const counts=Object.values(arms).every(row=>JSON.stringify(row.status.visibleMeshlets)===JSON.stringify(arms.base.status.visibleMeshlets));
  report.views.push({name,yaw,solar,exact,counts,arms});
  console.log(name,'all submitted bytes exact',exact,'counts equal',counts);
  if(!exact||!counts)throw Error(`before/after mismatch ${name}`);
 }
 report.pass=true;
} catch(error){failure=error;report.error=String(error);}
finally {
 await pending;
 try{await cleanup(child,()=>ended,cdp,server,profile);}catch(error){failure??=error;report.error=String(error);}
 try{report.permission_preflight=JSON.parse(await fs.readFile(path.join(output,'launcher.json'),'utf8'));if(report.permission_preflight.screen_recording_before!==false||report.permission_preflight.screen_recording_after!==false||report.permission_preflight.chrome_exit_code!==0)throw Error('launcher preflight/exit failed');}catch(error){failure??=error;report.error=String(error);}
 report.pass&&=!failure;
 await fs.writeFile(path.join(output,'browser.json'),JSON.stringify(report,null,2));
}
if(failure)throw failure;
