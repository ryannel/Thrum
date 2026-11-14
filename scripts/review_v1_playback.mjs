#!/usr/bin/env node
// Record complete 1x movie playback for frame-sequence self-review. Run under the GPU lock.
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { Cdp, serve, waitFor, cleanup } from './check_v1_browser.mjs';

const [input, destination] = process.argv.slice(2);
if (!input || !destination) throw new Error('Usage: review_v1_playback.mjs CAMPAIGN OUTPUT');
const root = path.resolve(input), output = path.resolve(destination);
if (!output.startsWith(path.resolve('tmp')+path.sep)) throw new Error('Use a new output under tmp/');
const campaign = JSON.parse(await fs.readFile(path.join(root,'campaign.json'),'utf8'));
if (!campaign.complete) throw new Error('Capture campaign is incomplete');
await fs.mkdir(output);
const profile = await fs.mkdtemp(path.join(output,'chrome-profile-'));
const files = new Map();
const {server,origin} = await serve(files);
const log = await fs.open(path.join(output,'chrome.log'),'w');
const child = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  [`--user-data-dir=${profile}`,'--remote-debugging-port=0','--headless=new',
   '--no-first-run','--no-default-browser-check','--disable-background-networking',
   '--autoplay-policy=no-user-gesture-required','about:blank'],{stdio:['ignore',log.fd,log.fd]});
await log.close();
let ended = false, cdp, failure;
child.once('exit',()=>{ended=true;});
const results = [];
const sleep = ms => new Promise(resolve=>setTimeout(resolve,ms));
try {
  const endpoint = await waitFor(async()=>{
    if (ended) throw new Error('Owned playback Chrome exited');
    try {return (await fs.readFile(path.join(profile,'DevToolsActivePort'),'utf8')).trim().split('\n');}
    catch {return undefined;}
  },15000,'owned playback Chrome');
  const socket = new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);
  await once(socket,'open'); cdp = new Cdp(socket);
  const {targetId} = await cdp.call('Target.createTarget',{url:'about:blank'});
  const {sessionId} = await cdp.call('Target.attachToTarget',{targetId,flatten:true});
  const evaluate = async expression => {
    const value = await cdp.call('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true},sessionId);
    if (value.exceptionDetails) throw new Error(JSON.stringify(value.exceptionDetails));
    return value.result.value;
  };
  for (const arm of campaign.arms) {
    const name = `${arm.case}-${arm.reconstruction}`;
    const session = path.join(root,name,...(campaign.target==='mac-metal'?['session']:[]));
    const movie = path.join(session,'preview-1x.mp4');
    const bytes = await fs.readFile(movie);
    const info = JSON.parse(execFileSync('ffprobe',['-v','error','-select_streams','v:0',
      '-show_entries','stream=width,height','-of','json',movie],{encoding:'utf8'})).streams[0];
    const folder = path.join(output,name); await fs.mkdir(folder);
    const page = `<!doctype html><style>body{margin:0;background:black}video{display:block}</style>
<video src="/movie.mp4?${name}" muted preload="auto"></video><script>
const video=document.querySelector('video');let started;
window.caseId=${JSON.stringify(name)};
window.begin=async()=>{video.playbackRate=1;started=performance.now();await video.play()};
window.state=()=>({ended:video.ended,time_s:video.currentTime,duration_s:video.duration,
rate:video.playbackRate,wall_ms:performance.now()-started,width:video.videoWidth,height:video.videoHeight,
quality:{totalVideoFrames:video.getVideoPlaybackQuality().totalVideoFrames,droppedVideoFrames:video.getVideoPlaybackQuality().droppedVideoFrames}});
</script>`;
    files.set('/index.html',{bytes:Buffer.from(page),type:'text/html'});
    files.set('/movie.mp4',{bytes,type:'video/mp4'});
    await cdp.call('Emulation.setDeviceMetricsOverride',{
      width:info.width,height:info.height,deviceScaleFactor:1,mobile:false},sessionId);
    await cdp.call('Page.navigate',{url:`${origin}/?${name}`},sessionId);
    await waitFor(()=>evaluate(`window.caseId===${JSON.stringify(name)} && !!window.begin && document.querySelector("video").readyState>=2`),15000,'movie readiness');
    await evaluate('window.begin()');
    const samples = [];
    const deadline = Date.now()+30000;
    while (Date.now()<deadline) {
      const state = await evaluate('window.state()');
      const image = `sample-${String(samples.length).padStart(3,'0')}.png`;
      const shot = await cdp.call('Page.captureScreenshot',{format:'png'},sessionId);
      await fs.writeFile(path.join(folder,image),Buffer.from(shot.data,'base64'));
      samples.push({...state,image});
      if (state.ended) break;
      await sleep(200);
    }
    const state = await evaluate('window.state()');
    if (!state.ended || state.rate!==1 || state.width!==info.width || state.height!==info.height)
      throw new Error(`Incomplete or rescaled playback: ${name}`);
    const result = {name,movie,movie_sha256:createHash('sha256').update(bytes).digest('hex'),
      ...state,samples,scope:'Native-size complete 1x decoder playback; inspect saved temporal frames separately. Decoder drops are not source omissions.'};
    await fs.writeFile(path.join(folder,'playback.json'),JSON.stringify(result,null,2)+'\n');
    results.push(result);
  }
} catch (error) {failure=error;}
finally {
  try {await cleanup(child,()=>ended,cdp,server,profile);}
  catch (error) {failure=failure?new AggregateError([failure,error]):error;}
  await fs.writeFile(path.join(output,'run.json'),JSON.stringify({complete:!failure,results,
    error:failure?String(failure):undefined},null,2)+'\n');
}
if (failure) throw failure;
console.log(`Played ${results.length} complete native-size movies at 1x; ${output}`);
