// Actual prepared light-list readback on hardware Chrome. Run under chrome lock.
import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn,execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {Cdp,serve,hardware,waitFor,cleanup} from './check_v1_browser.mjs';
const output=path.resolve(process.argv[2]);
const root=path.resolve(process.argv[3]??'target/v1-game-web-probe');
await fs.mkdir(output,{recursive:false});
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const files=new Map();
for(const name of ['v1_game.js','v1_game_bg.wasm']) {
 const bytes=await fs.readFile(path.join(root,name));
 files.set('/'+name,{bytes,type:name.endsWith('.wasm')?'application/wasm':'text/javascript',sha256:sha(bytes)});
}
files.set('/probe.html',{bytes:Buffer.from('<!doctype html><title>Actual light-list readback</title>'),type:'text/html'});
const profile=await fs.mkdtemp(path.join(output,'profile-'));
const bundle=path.join(output,'ThrumCapture.app');
const launcher=path.join(bundle,'Contents/MacOS/Launcher');
await fs.mkdir(path.dirname(launcher),{recursive:true});
await fs.writeFile(path.join(bundle,'Contents/Info.plist'),'<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>org.thrum.capture</string><key>CFBundleName</key><string>Thrum Capture</string><key>CFBundleExecutable</key><string>Launcher</string><key>CFBundlePackageType</key><string>APPL</string><key>LSUIElement</key><true/></dict></plist>');
execFileSync('swiftc',['-module-cache-path',path.join(output,'swift-cache'),'scripts/capture_browser_launcher.swift','-o',launcher]);
execFileSync('codesign',['--force','--sign','-',bundle]);
const report={passed:false,revision:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),
 diff_sha256:sha(execFileSync('git',['diff','HEAD'])),artifacts:Object.fromEntries([...files].map(([key,value])=>[key,value.sha256]))};
let child,cdp,server,ended=false,failure;
try {
 const serving=await serve(files);server=serving.server;
 const flags=[`--user-data-dir=${profile}`,'--remote-debugging-port=0','--headless=new','--no-first-run','--no-default-browser-check','--disable-background-networking','--disable-component-update','about:blank'];
 child=spawn('/usr/bin/open',['-n','-W',bundle,'--args',output,'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',...flags],{stdio:'ignore'});
 child.once('exit',()=>{ended=true;});
 const endpoint=await waitFor(async()=>{if(ended)throw Error('Chrome exited');try{return(await fs.readFile(path.join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n');}catch(error){if(error.code!=='ENOENT')throw error;}},15000,'Chrome DevTools');
 const socket=new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);
 await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
 cdp=new Cdp(socket);
 report.browser=await cdp.call('Browser.getVersion');report.gpu=await cdp.call('SystemInfo.getInfo');hardware(report.gpu);
 const {targetId}=await cdp.call('Target.createTarget',{url:'about:blank'});
 const {sessionId}=await cdp.call('Target.attachToTarget',{targetId,flatten:true});
 await cdp.call('Page.navigate',{url:`${serving.origin}/probe.html`},sessionId);
 await waitFor(async()=>{const r=await cdp.call('Runtime.evaluate',{expression:'location.pathname === "/probe.html" && document.readyState === "complete"',returnByValue:true},sessionId);return r.result.value;},15000,'probe page');
 report.rows=[];
 for(const count of [4,32,64]) for(const ties of [false,true]) {
  const r=await cdp.call('Runtime.evaluate',{expression:`(async()=>{const m=await import('/v1_game.js');await m.default();const p=await m.create_light_list_probe(${count},${ties});try{const deadline=performance.now()+30000;while(performance.now()<deadline){const r=p.poll();if(r)return r;await new Promise(resolve=>setTimeout(resolve,5));}throw Error('probe mapping timeout');}finally{p.free();}})()`,awaitPromise:true,returnByValue:true},sessionId);
  if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));
  const row=JSON.parse(r.result.value);report.rows.push(row);
  if(!row.equal||row.gpu_build!==(count===4)||row.checked_bytes!==9216*(8+Math.min(count,32)*4)||row.dropped_references!==(count===64?9216*32:0))throw Error('list mismatch, incomplete overlap or wrong execution path');
 }
 report.passed=true;
}catch(error){failure=error;report.error=String(error);}
finally {
 try{await cleanup(child,()=>ended,cdp,server,profile);}catch(error){failure??=error;report.error=String(error);}
 try{report.preflight=JSON.parse(await fs.readFile(path.join(output,'launcher.json'),'utf8'));if(report.preflight.screen_recording_before!==false||report.preflight.screen_recording_after!==false)throw Error('denied preflights required');}catch(error){failure??=error;report.error=String(error);}
 report.passed&&=!failure;
 await fs.writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2)+'\n');
}
if(failure)throw failure;
