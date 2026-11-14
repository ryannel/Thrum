// Exercise the shipped page without changing its source or game exports.
import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {Cdp, assets, serve, serveCooked, hardware, waitFor, cleanup} from '../../../../scripts/check_v1_browser.mjs';
const output=path.resolve('tmp/culling-m3/review-stars-default');
await fs.mkdir(output,{recursive:true});
const profile=await fs.mkdtemp(path.join(output,'chrome-'));
let child,cdp,server,cook,ended=false;
const result={source_revision:'21aa3fb764d1931b88ba2d08f919cd290d7adc82',cases:[]};
try {
 const files=await assets(path.resolve('target/v1-game-web'));
 const serving=await serve(files); server=serving.server;
 const cooked=await serveCooked({cook:'target/release/thrum-cook','assets-source':'examples/v1-game/assets/showcase'},output); cook=cooked.cook;
 child=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',[`--user-data-dir=${profile}`,'--remote-debugging-port=0','--headless=new','--no-first-run','--no-default-browser-check','about:blank'],{stdio:'ignore'});
 child.once('exit',()=>ended=true);
 const endpoint=await waitFor(async()=>{try{return (await fs.readFile(path.join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n')}catch{}},15000,'CDP');
 const socket=new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);
 await new Promise((res,rej)=>{socket.addEventListener('open',res,{once:true});socket.addEventListener('error',rej,{once:true})});cdp=new Cdp(socket);
 result.browser=await cdp.call('Browser.getVersion'); result.gpu=await cdp.call('SystemInfo.getInfo'); hardware(result.gpu);
 for(const suffix of ['', '&stars=off', '&stars=off&culling=frustum']) {
  const url=`${serving.origin}/?qualify&assets=${encodeURIComponent(cooked.origin)}${suffix}`;
  const {targetId}=await cdp.call('Target.createTarget',{url:'about:blank'});
  const {sessionId}=await cdp.call('Target.attachToTarget',{targetId,flatten:true});
  await cdp.call('Page.navigate',{url},sessionId);
  const status=await waitFor(async()=>{const r=await cdp.call('Runtime.evaluate',{expression:"document.querySelector('#status')?.textContent",returnByValue:true},sessionId); if(/fault|error/i.test(r.result.value??'')) throw Error(r.result.value); return /culling=/.test(r.result.value??'')?r.result.value:null},90000,'game report');
  result.cases.push({query:suffix||'ordinary',status}); console.log(suffix||'ordinary',status);
  await cdp.call('Target.closeTarget',{targetId});
 }
 await fs.writeFile('docs/evidence/CULLING/M3-review/stars-default.json',JSON.stringify(result,null,2)+'\n');
} finally {if(cook) cook.kill('SIGTERM'); await cleanup(child,()=>ended,cdp,server,profile);}
