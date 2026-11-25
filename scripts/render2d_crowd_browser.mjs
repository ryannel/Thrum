// Render the retained Crowd replay on a real Chrome WebGPU canvas.
import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const [web, folder, encoded] = process.argv.slice(2);
const request = JSON.parse(encoded);
const profile = path.join(folder, 'chrome-profile');
const images = path.join(folder, 'frames.rgba');
const workerSource = new URL('../examples/v1-game/web/capture_upload.mjs', import.meta.url);
const server = createServer(async (req, res) => {
  try {
    res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
    res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
    if (req.url === '/capture_upload.mjs') {
      res.setHeader('Content-Type', 'text/javascript');
      res.end(await fs.readFile(workerSource));
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
  if (request.action === 'realtime') {
    await send('Browser.setDownloadBehavior', { behavior: 'allow', downloadPath: path.resolve(folder) });
  }
  const response = await send('Runtime.evaluate', {
    expression: `(() => { const captureMotion = ${captureMotion.toString()}; return (${run.toString()})(${JSON.stringify(request)}); })()`,
    awaitPromise: true, returnByValue: true, timeout: 1_500_000,
  });
  if (response.exceptionDetails) throw new Error(response.result.description);
  const record = response.result.value;
  if (request.action === 'realtime') {
    const wanted = record.expected * request.extent[0] * request.extent[1] * 4;
    const deadline = performance.now() + 60_000;
    while (performance.now() < deadline) {
      try {
        if ((await fs.stat(images)).size === wanted) break;
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      await delay(100);
    }
    if ((await fs.stat(images)).size !== wanted) throw new Error('downloaded spool length differs from submitted images');
  } else {
    for (const name of ['publications', 'samples']) {
      record[name] = [];
      const length = record.row_counts[name];
      for (let start = 0; start < length; start += 400) {
        const chunk = await send('Runtime.evaluate', {
          expression: `globalThis.crowdReplayRows.${name}.slice(${start},${start + 400})`,
          returnByValue: true,
        });
        if (chunk.exceptionDetails) throw new Error(chunk.result.description);
        if (chunk.result.value.length !== Math.min(400, length - start)) throw new Error('short replay chunk');
        record[name].push(...chunk.result.value);
      }
    }
  }
  await fs.writeFile(path.join(folder, 'replay.json'), JSON.stringify({
    ...record, launch_options: launchOptions, target: 'chrome-webgpu', workload: 'retained Crowd',
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
      const { startCaptureUploadWorker } = await import('/capture_upload.mjs');
      const worker = await startCaptureUploadWorker({ spool: true });
      try {
        return await captureMotion(crowd, worker, yieldToBrowser, samples, opportunities, identity, extent);
      } finally {
        worker.close();
      }
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
    // Keep large timelines out of a single debugger message.
    globalThis.crowdReplayRows = { publications, samples };
    return { ...identity, row_counts: { publications: publications.length, samples: samples.length },
      timing_status: JSON.parse(crowd.timings()), user_agent: navigator.userAgent };
  } finally {
    channel.port1.close();
    channel.port2.close();
    canvas.getContext('webgpu').unconfigure();
    crowd.free();
  }
}

async function captureMotion(crowd, worker, yieldToBrowser, samples, opportunities, identity, extent) {
  const slots = 32;
  const byteLength = extent[0] * extent[1] * 4;
  const free = Array.from({ length: slots }, () => new Uint8Array(byteLength));
  const inflight = new Set();
  const hashes = [];
  let exportError;
  function drain() {
    if (exportError) throw exportError;
    while (free.length) {
      const pixels = free.pop();
      const image = crowd.capture(pixels);
      if (!image) {
        free.push(pixels);
        return;
      }
      const job = { metadata: image.metadata(), pixels: image.pixels() };
      image.free();
      const ordinal = samples.length;
      samples.push(JSON.parse(job.metadata));
      const task = worker.upload(ordinal, job).then(result => {
        free.push(result.pixels);
        if (result.error) throw new Error(result.error);
        hashes[ordinal] = result.rgba_sha256;
      }).catch(error => { exportError = error; }).finally(() => inflight.delete(task));
      inflight.add(task);
    }
  }
  const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
  crowd.begin_capture();
  const start = performance.now();
  const period = 1000 / 45;
  let next = 0, expected = 0;
  while (performance.now() - start < 22_500) {
    if (document.hidden) throw new Error('capture page is hidden');
    drain();
    const elapsed = performance.now() - start;
    if (elapsed < next) {
      await wait(Math.min(3, next - elapsed));
      continue;
    }
    const ordinal = Math.floor(elapsed / period);
    next = (ordinal + 1) * period;
    if (free.length < 4) {
      opportunities.push({ request_ns: Math.round(elapsed * 1e6), submitted: false, export_backpressure: true });
    } else {
      const submitted = crowd.step(elapsed);
      expected += Number(submitted);
      opportunities.push({ request_ns: Math.round(elapsed * 1e6), submitted });
    }
    await wait(0);
  }
  const deadline = performance.now() + 15_000;
  while (samples.length < expected && performance.now() < deadline) {
    drain();
    if (samples.length < expected) await yieldToBrowser();
  }
  await Promise.all(inflight);
  if (exportError) throw exportError;
  if (samples.length !== expected || hashes.length !== expected || expected < 2) {
    throw new Error('some submitted images were not retained');
  }
  const written = await worker.seal();
  const directory = await navigator.storage.getDirectory();
  const entry = await directory.getFileHandle('sprite-capture.rgba');
  const file = await entry.getFile();
  if (file.size !== written || file.size !== expected * byteLength) throw new Error('invalid sealed browser spool');
  const download = document.createElement('a');
  download.href = URL.createObjectURL(file);
  download.download = 'frames.rgba';
  download.click();
  return { ...identity, mode: 'realtime', encoding: 'Raw', images: samples, opportunities, hashes, expected };
}
