import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {Cdp,serve,waitFor,cleanup} from '../../scripts/check_v1_browser.mjs';
const root=path.resolve('tmp/t2-capture-m1/playback-review');await fs.mkdir(root);
const profile=await fs.mkdtemp(path.join(root,'chrome-'));
const files=new Map([
 ['/index.html',{type:'text/html',bytes:Buffer.from('<html><body style="margin:0"><video id="v" width="1280" height="720" muted autoplay src="preview.mp4"></video></body></html>')}],
 ['/preview.mp4',{type:'video/mp4',bytes:await fs.readFile('tmp/t2-capture-m1/final-00/preview-1x.mp4')}],
]);
let child,cdp,server,ended=false;
try {
 const serving=await serve(files);server=serving.server;
 const log=await fs.open(path.join(root,'chrome.log'),'w');
 child=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',[`--user-data-dir=${profile}`,'--remote-debugging-port=0','--headless=new','--no-first-run','--no-default-browser-check','--autoplay-policy=no-user-gesture-required','about:blank'],{stdio:['ignore',log.fd,log.fd]});child.on('exit',()=>ended=true);await log.close();
 const endpoint=await waitFor(async()=>{try{return(await fs.readFile(path.join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n')}catch{return undefined}},15000,'DevTools');
 const socket=new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);await new Promise((r,j)=>{socket.addEventListener('open',r,{once:true});socket.addEventListener('error',j,{once:true})});cdp=new Cdp(socket);
 const {targetId}=await cdp.call('Target.createTarget',{url:'about:blank'});const {sessionId}=await cdp.call('Target.attachToTarget',{targetId,flatten:true});
 await cdp.call('Page.enable',{},sessionId);await cdp.call('Emulation.setDeviceMetricsOverride',{width:1280,height:720,deviceScaleFactor:1,mobile:false},sessionId);await cdp.call('Page.navigate',{url:serving.origin},sessionId);
 const read=async()=>{const r=await cdp.call('Runtime.evaluate',{expression:'(()=>{const v=document.querySelector("video");return v?{time:v.currentTime,rate:v.playbackRate,paused:v.paused,ended:v.ended,ready:v.readyState,quality:v.getVideoPlaybackQuality(),now:performance.now()}:null})()',returnByValue:true},sessionId);return r.result.value};
 const snapshots=[];
 for(const desired of [0.1,1,2,3,4,4.25,4.45,5,6,7,7.9]) {
  const state=await waitFor(async()=>{const s=await read();if(s?.ready>=2&&s.time>=desired)return s},15000,`playback ${desired}`);
  if(state.rate!==1 || (state.paused&&!state.ended))throw Error('Playback was not normal speed');
  const shot=await cdp.call('Page.captureScreenshot',{format:'png',fromSurface:true},sessionId);const file=`${String(snapshots.length).padStart(2,'0')}.png`;await fs.writeFile(path.join(root,file),Buffer.from(shot.data,'base64'));snapshots.push({...state,file});
 }
 await fs.writeFile(path.join(root,'playback.json'),JSON.stringify(snapshots,null,2));
}finally{await cleanup(child,()=>ended,cdp,server,profile)}
