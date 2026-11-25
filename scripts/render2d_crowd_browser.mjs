// Render the retained Crowd replay on a real Chrome WebGPU canvas.
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const [web, folder, encoded] = process.argv.slice(2);
const request = JSON.parse(encoded);
const profile = path.join(folder, 'chrome-profile');
const images = path.join(folder, 'frames.rgba');
if (request.action === 'realtime') await fs.writeFile(images, '');
const hashes = [];
const server = createServer(async (req, res) => {
  try {
    res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
    res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
    if (req.method === 'POST' && req.url === '/image' && request.action === 'realtime') {
      const chunks = [];
      const limit = request.extent[0] * request.extent[1] * 4;
      let length = 0;
      for await (const chunk of req) {
        length += chunk.length;
        if (length > limit) throw new Error('image exceeds the backing extent');
        chunks.push(chunk);
      }
      if (length !== limit || hashes.length >= 1600) throw new Error('image length or record limit');
      const image = Buffer.concat(chunks);
      await fs.appendFile(images, image);
      hashes.push(createHash('sha256').update(image).digest('hex'));
      res.end();
      return;
    }
    if (req.url === '/') {
      res.setHeader('Content-Type', 'text/html');
      res.end('<!doctype html><title>Crowd replay</title><canvas></canvas>');
      return;
    }
    if (!['/crowd_2d_view.js', '/crowd_2d_view_bg.wasm'].includes(req.url)) throw new Error('unknown asset');
    res.setHeader('Content-Type', req.url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
    res.end(await fs.readFile(path.join(web, path.basename(req.url))));
  } catch (error) {
    res.writeHead(500);
    res.end(String(error));
  }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
// Use one fixed Metal counter period for the local timing runs. Dawn's changing
// estimated period can move absolute counters backwards between batches.
// Realtime and correctness use default Chrome settings.
const launchOptions = request.action === 'crowd-costs'
  ? ['--disable-dawn-features=timestamp_quantization', '--enable-dawn-features=metal_disable_timestamp_period_estimation'] : [];
const chrome = spawn(process.env.CHROME_BIN ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
  '--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-background-networking',
  '--remote-debugging-port=0', `--user-data-dir=${profile}`, ...launchOptions, 'about:blank',
], { stdio: ['ignore', 'ignore', 'ignore'] });
let exited = false;
const exit = new Promise(resolve => chrome.once('exit', () => { exited = true; resolve(); }));
let socket, send;
try {
  let port;
  for (let attempt = 0; attempt < 300 && !port; attempt++) {
    try { port = Number((await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]); }
    catch { await delay(100); }
  }
  if (!port) throw new Error('Chrome CDP connection unavailable');
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  socket = new WebSocket(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.onopen = resolve;
    socket.onerror = reject;
  });
  const pending = new Map();
  let sequence = 0, loaded;
  socket.onmessage = event => {
    const reply = JSON.parse(event.data);
    if (reply.method === 'Page.loadEventFired') loaded?.();
    const request = pending.get(reply.id);
    if (request) {
      pending.delete(reply.id);
      if (reply.error) request.reject(new Error(JSON.stringify(reply.error)));
      else request.resolve(reply.result);
    }
  };
  socket.onclose = () => {
    for (const action of pending.values()) action.reject(new Error('Chrome closed CDP'));
    pending.clear();
  };
  send = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++sequence;
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params }));
  });
  await send('Page.enable');
  const ready = new Promise(resolve => { loaded = resolve; });
  await send('Page.navigate', { url: origin });
  await ready;
  const response = await send('Runtime.evaluate', {
    expression: `(${run.toString()})(${JSON.stringify(request)})`,
    awaitPromise: true, returnByValue: true, timeout: 1_500_000,
  });
  if (response.exceptionDetails) throw new Error(response.result.description);
  await fs.writeFile(path.join(folder, 'replay.json'), JSON.stringify({
    ...response.result.value, hashes, launch_options: launchOptions, target: 'chrome-webgpu', workload: 'retained Crowd',
    count: request.count, extent: request.extent,
  }));
} finally {
  if (socket?.readyState === WebSocket.OPEN) void send('Browser.close').catch(() => {});
  await Promise.race([exit, delay(5000, undefined, { ref: false })]);
  if (!exited) {
    chrome.kill('SIGKILL');
    await exit;
  }
  socket?.close();
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
  await fs.rm(profile, { recursive: true, force: true, maxRetries: 20, retryDelay: 100 });
}

async function run({ action, count, extent, ticks, close, run_id }) {
  const { default: init, BrowserCrowd } = await import('/crowd_2d_view.js');
  await init();
  const canvas = document.querySelector('canvas');
  [canvas.width, canvas.height] = extent;
  const realtime = action === 'realtime';
  const crowd = await BrowserCrowd.open(canvas, count, close, !realtime, action === 'crowd-costs');
  const adapter = await navigator.gpu.requestAdapter({ powerPreference: 'high-performance' });
  if (!adapter || adapter.info.isFallbackAdapter) throw new Error('hardware WebGPU is unavailable');
  const gpu = { vendor: adapter.info.vendor, architecture: adapter.info.architecture };
  const identity = { ...JSON.parse(crowd.identity()), gpu, run_id };
  const channel = new MessageChannel();
  const yieldToBrowser = () => new Promise(resolve => {
    channel.port1.onmessage = resolve;
    channel.port2.postMessage(null);
  });
  const publications = [], samples = [], opportunities = [];
  const nextFrame = () => new Promise(resolve => requestAnimationFrame(resolve));
  try {
    if (realtime) {
      crowd.begin_capture();
      const start = await nextFrame();
      async function image() {
        const capture = crowd.capture();
        if (!capture) return false;
        try {
          const result = await fetch('/image', { method: 'POST', body: capture.pixels() });
          if (!result.ok) throw new Error(await result.text());
          samples.push(JSON.parse(capture.metadata()));
        } finally {
          capture.free();
        }
        return true;
      }
      let elapsed, expected = 0;
      do {
        const now = await nextFrame();
        elapsed = now - start;
        const submitted = crowd.step(now);
        expected += Number(submitted);
        opportunities.push({ request_ns: Math.round(elapsed * 1e6), submitted });
        while (await image()) {}
      } while (elapsed < 20_400);
      const deadline = performance.now() + 15_000;
      while (samples.length < expected && performance.now() < deadline) {
        if (!await image()) await yieldToBrowser();
      }
      if (samples.length !== expected || expected < 2) throw new Error('some submitted images were not retained');
      return { ...identity, mode: 'realtime', encoding: 'Raw', images: samples, opportunities, expected };
    }
    for (let tick = 1; tick <= ticks; tick++) {
      const now = action === 'crowd-costs' ? await nextFrame() : performance.now();
      if (!crowd.step(now)) throw new Error(`no WebGPU submission for tick ${tick}`);
      publications.push({ ...JSON.parse(crowd.publication(action === 'crowd' || tick % 120 === 0)), run_id });
      const deadline = performance.now() + 10_000;
      for (;;) {
        const ready = JSON.parse(crowd.samples());
        if (ready.length) {
          samples.push(...ready.map(row => ({ ...row, run_id })));
          break;
        }
        if (performance.now() > deadline) throw new Error(`no completed submission for tick ${tick}`);
        await yieldToBrowser();
      }
    }
    return { ...identity, publications, samples, timing_status: JSON.parse(crowd.timings()), user_agent: navigator.userAgent };
  } finally {
    channel.port1.close();
    channel.port2.close();
    canvas.getContext('webgpu').unconfigure();
    crowd.free();
  }
}
