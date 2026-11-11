#!/usr/bin/env node
// EFFECT-GRAPH M1: enabled Chrome WebGPU storage contract, on the actual device.
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';

const output = path.resolve(process.argv[2] ?? 'tmp/effect-graph-m1/chrome-limits');
await fs.mkdir(output, { recursive: true });
const profile = await fs.mkdtemp(path.join(output, 'profile-'));
const log = await fs.open(path.join(output, 'chrome.log'), 'w');
const server = createServer((_request, response) => response.end('<title>Effect graph capability record</title>'));
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const child = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [`--user-data-dir=${profile}`, '--remote-debugging-port=0', '--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-component-update', 'about:blank'], { stdio: ['ignore', log.fd, log.fd] });
let socket;
try {
  const deadline = Date.now() + 60000;
  let port;
  while (!port && Date.now() < deadline) {
    try { port = Number((await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]); } catch {}
    if (!port) await new Promise(resolve => setTimeout(resolve, 100));
  }
  if (!port) throw new Error('Chrome did not start');
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  socket = new WebSocket(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
  let next = 0;
  const pending = new Map();
  socket.onmessage = event => {
    const packet = JSON.parse(event.data);
    const entry = pending.get(packet.id);
    if (!entry) return;
    pending.delete(packet.id);
    clearTimeout(entry.timer);
    if (packet.error) entry.reject(new Error(JSON.stringify(packet.error))); else entry.resolve(packet.result);
  };
  const call = (method, params) => new Promise((resolve, reject) => {
    const id = ++next;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 60000);
    pending.set(id, { resolve, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
  });
  await call('Page.navigate', { url: `http://127.0.0.1:${server.address().port}` });
  // Wait for the secure localhost context to replace about:blank.
  let result;
  while (Date.now() < deadline) {
    result = await call('Runtime.evaluate', { expression: 'location.hostname === "127.0.0.1" && Boolean(navigator.gpu)', returnByValue: true });
    if (result.result.value) break;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  const expression = `(async () => {
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) throw new Error('No hardware WebGPU adapter');
    const device = await adapter.requestDevice();
    const formats = ['r32float','r32sint','r32uint','rg32float','rg32sint','rg32uint','rgba8unorm','rgba8snorm','rgba8sint','rgba8uint','rgba16float','rgba16sint','rgba16uint','rgba32float','rgba32sint','rgba32uint','bgra8unorm','rgba8unorm-srgb','depth32float'];
    const storage = {};
    for (const format of formats) {
      storage[format] = {};
      for (const access of ['write-only','read-only','read-write']) {
        device.pushErrorScope('validation');
        device.createBindGroupLayout({ entries: [{ binding: 0, visibility: GPUShaderStage.COMPUTE, storageTexture: { access, format, viewDimension: '2d-array' } }] });
        storage[format][access] = (await device.popErrorScope()) === null;
      }
    }
    const views = {};
    for (const viewDimension of ['2d','2d-array','3d','cube','cube-array']) {
      device.pushErrorScope('validation');
      device.createBindGroupLayout({ entries: [{ binding: 0, visibility: GPUShaderStage.COMPUTE, storageTexture: { access: 'write-only', format: 'rgba32float', viewDimension } }] });
      views[viewDimension] = (await device.popErrorScope()) === null;
    }
    const mipBindings = {};
    const texture = device.createTexture({ size: [4,4,1], mipLevelCount: 2, format: 'rgba32float', usage: GPUTextureUsage.STORAGE_BINDING });
    const layout = device.createBindGroupLayout({ entries: [{ binding: 0, visibility: GPUShaderStage.COMPUTE, storageTexture: { access: 'write-only', format: 'rgba32float' } }] });
    for (const mipLevelCount of [1,2]) {
      device.pushErrorScope('validation');
      const view = texture.createView({ mipLevelCount });
      device.createBindGroup({ layout, entries: [{ binding: 0, resource: view }] });
      mipBindings[mipLevelCount] = (await device.popErrorScope()) === null;
    }
    texture.destroy();
    const limits = {};
    for (const key of ['maxStorageTexturesPerShaderStage','maxTextureDimension2D','maxTextureDimension3D','maxTextureArrayLayers']) limits[key] = device.limits[key];
    const record = { userAgent: navigator.userAgent, adapter: { vendor: adapter.info.vendor, architecture: adapter.info.architecture, device: adapter.info.device, description: adapter.info.description }, features: [...device.features], limits, storage, views, mipBindings };
    device.destroy(); return record;
  })()`;
  result = await call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (result.exceptionDetails || !result.result.value) throw new Error(JSON.stringify(result));
  await fs.writeFile(path.join(output, 'record.json'), JSON.stringify(result.result.value, null, 2) + '\n');
  console.log(JSON.stringify(result.result.value));
} finally {
  socket?.close(); child.kill('SIGTERM'); server.close(); await log.close();
  await new Promise(resolve => { if (child.exitCode !== null || child.signalCode !== null) resolve(); else child.once('exit', resolve); });
  await fs.rm(profile, { recursive: true, force: true });
}
