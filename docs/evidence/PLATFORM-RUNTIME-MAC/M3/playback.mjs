// Review a complete host-time movie at 1x in a native-size browser viewport.
// Run under the GPU lock. Browser decoder drops are separate from source omissions.
import fs from 'node:fs/promises';
import path from 'node:path';
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { once } from 'node:events';

const [movie, destination] = process.argv.slice(2);
if (!movie || !destination) throw new Error('Usage: node playback.mjs MOVIE OUTPUT');
const output = path.resolve(destination);
await fs.mkdir(output, { recursive: true });
const profile = await fs.mkdtemp(path.join(output, 'chrome-profile-'));
const bytes = await fs.readFile(movie);
const page = `<!doctype html><style>body{margin:0;background:black}video{width:1920px;height:1080px}</style>
<video src="/movie.mp4" muted preload="auto"></video><script>
const video=document.querySelector('video'); let started;
window.begin=async()=>{video.playbackRate=1;started=performance.now();await video.play()};
window.result=()=>({ended:video.ended,current:video.currentTime,rate:video.playbackRate,
duration_s:video.duration,wall_ms:performance.now()-started,width:video.videoWidth,height:video.videoHeight,
quality:{totalVideoFrames:video.getVideoPlaybackQuality().totalVideoFrames,
droppedVideoFrames:video.getVideoPlaybackQuality().droppedVideoFrames}});
</script>`;
const server = createServer((request, response) => {
  const data = request.url === '/movie.mp4' ? bytes : Buffer.from(page);
  response.writeHead(200, { 'Content-Type': request.url === '/movie.mp4' ? 'video/mp4' : 'text/html',
                            'Content-Length': data.length });
  response.end(data);
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const log = await fs.open(path.join(output, 'chrome.log'), 'w');
const chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  [`--user-data-dir=${profile}`, '--remote-debugging-port=0',
   '--no-first-run', '--no-default-browser-check', '--disable-background-networking',
   '--autoplay-policy=no-user-gesture-required', '--window-size=1920,1080', 'about:blank'],
  { stdio: ['ignore', log.fd, log.fd] });
await log.close();
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let socket;
const pending = new Map();
let next = 0;
try {
  let endpoint;
  const deadline = Date.now() + 15000;
  while (!endpoint && Date.now() < deadline) {
    try { endpoint = (await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n'); }
    catch { await sleep(100); }
  }
  if (!endpoint) throw new Error('Owned Chrome startup timed out');
  socket = new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);
  await once(socket, 'open');
  socket.addEventListener('message', event => {
    const packet = JSON.parse(event.data), item = pending.get(packet.id);
    if (!item) return;
    pending.delete(packet.id); clearTimeout(item.timer);
    packet.error ? item.reject(new Error(JSON.stringify(packet.error))) : item.resolve(packet.result);
  });
  const call = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
    const id = ++next;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(method + ' timed out')); }, 15000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
  });
  const { targetId } = await call('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await call('Target.attachToTarget', { targetId, flatten: true });
  await call('Page.bringToFront', {}, sessionId);
  await call('Emulation.setDeviceMetricsOverride', { width: 1920, height: 1080, deviceScaleFactor: 1, mobile: false }, sessionId);
  await call('Page.navigate', { url: `http://127.0.0.1:${server.address().port}/` }, sessionId);
  const evaluate = async expression => {
    const result = await call('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId);
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  for (let attempt = 0; attempt < 100; attempt++) {
    if (await evaluate('!!window.begin && document.querySelector("video").readyState>=2')) break;
    if (attempt === 99) throw new Error('Movie did not become ready');
    await sleep(100);
  }
  await evaluate('window.begin()');
  const samples = [];
  const playbackDeadline = Date.now() + 30000;
  while (Date.now() < playbackDeadline) {
    const state = await evaluate('window.result()');
    const filename = `sample-${String(samples.length).padStart(3, '0')}.png`;
    const shot = await call('Page.captureScreenshot', { format: 'png' }, sessionId);
    await fs.writeFile(path.join(output, filename), Buffer.from(shot.data, 'base64'));
    samples.push({ ...state, image: filename });
    if (state.ended) break;
    await sleep(200);
  }
  const result = await evaluate('window.result()');
  if (!result.ended || result.rate !== 1 || result.width !== 1920 || result.height !== 1080)
    throw new Error('Incomplete or rescaled playback');
  await fs.writeFile(path.join(output, 'playback.json'), JSON.stringify({ movie, ...result, samples }, null, 2) + '\n');
  await call('Browser.close');
} finally {
  socket?.close();
  for (const item of pending.values()) clearTimeout(item.timer);
  if (chrome.exitCode === null) chrome.kill('SIGTERM');
  if (chrome.exitCode === null) await once(chrome, 'exit');
  await new Promise(resolve => server.close(resolve));
  await fs.rm(profile, { recursive: true, force: true });
}
