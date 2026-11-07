#!/usr/bin/env node
// Actual built-game qualification, not display cadence or a performance measurement.
import fs from 'node:fs/promises';
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import zlib from 'node:zlib';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = fileURLToPath(new URL('../', import.meta.url));
const cases = ['normal', 'pause', 'resize', 'overload', 'input', 'controls', 'params', 'device-loss', 'late-device-loss', 'device-recovery', 'device-recovery-retry', 'sky-reflections', 'valley', 'valley-recovery'];
const configureFailureSentinel = 'THRUM_G3B_CONFIGURE_ONCE';
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
let interrupted;
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => { interrupted = new Error(signal); });

function options(args) {
  const result = { case: 'all', 'web-root': 'target/v1-game-web', cook: 'target/debug/thrum-cook', 'assets-source': 'examples/v1-game/assets/showcase', chrome: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', 'timeout-ms': 90000 };
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--help') return null;
    const key = args[i].slice(2);
    if (!args[i].startsWith('--') || !['case', 'web-root', 'chrome', 'output', 'timeout-ms', 'cook', 'assets-source'].includes(key) || !args[i + 1]) throw new Error(`Unknown or incomplete argument: ${args[i]}`);
    result[key] = args[++i];
  }
  if (result.case !== 'all' && !cases.includes(result.case)) throw new Error(`Unknown case: ${result.case}`);
  result['timeout-ms'] = Number(result['timeout-ms']);
  if (!Number.isSafeInteger(result['timeout-ms']) || result['timeout-ms'] < 10000 || result['timeout-ms'] > 600000) throw new Error('--timeout-ms must be 10000..600000 per case');
  return result;
}

async function waitFor(check, milliseconds, description) {
  const deadline = Date.now() + milliseconds;
  while (Date.now() < deadline) {
    const value = await check();
    if (value) return value;
    await sleep(100);
  }
  throw new Error(`Timed out: ${description}`);
}

class Cdp {
  constructor(socket) {
    this.socket = socket;
    this.next = 0;
    this.pending = new Map();
    this.event = () => {};
    socket.addEventListener('message', event => {
      const packet = JSON.parse(event.data);
      if (!packet.id) return this.event(packet);
      const item = this.pending.get(packet.id);
      if (!item) return;
      clearTimeout(item.timer);
      this.pending.delete(packet.id);
      packet.error ? item.reject(new Error(JSON.stringify(packet.error))) : item.resolve(packet.result);
    });
    socket.addEventListener('close', () => {
      for (const item of this.pending.values()) { clearTimeout(item.timer); item.reject(new Error('CDP connection closed')); }
      this.pending.clear();
    });
  }
  call(method, params = {}, sessionId) {
    return new Promise((resolve, reject) => {
      const id = ++this.next;
      const timer = setTimeout(() => { this.pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15000);
      this.pending.set(id, { resolve, reject, timer });
      try { this.socket.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) })); }
      catch (error) { clearTimeout(timer); this.pending.delete(id); reject(error); }
    });
  }
}

async function assets(directory) {
  const result = new Map();
  for (const [name, type] of [['index.html', 'text/html'], ['v1_game.js', 'text/javascript'], ['v1_game_bg.wasm', 'application/wasm'], ['thrum-controls.js', 'text/javascript']]) {
    const bytes = await fs.readFile(path.join(directory, name));
    result.set(`/${name}`, { bytes, type, sha256: sha(bytes) });
  }
  return result;
}

async function serve(files) {
  const server = createServer((request, response) => {
    let pathname;
    try { pathname = new URL(request.url, 'http://localhost').pathname; }
    catch { response.writeHead(400).end(); return; }
    const file = files.get(pathname === '/' ? '/index.html' : pathname);
    if (!file || !['GET', 'HEAD'].includes(request.method)) { response.writeHead(404).end(); return; }
    response.writeHead(200, { 'Content-Type': file.type, 'Content-Length': file.bytes.length, 'Cache-Control': 'no-store' });
    response.end(request.method === 'HEAD' ? undefined : file.bytes);
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  return { server, origin: `http://127.0.0.1:${server.address().port}` };
}

function hardware(gpu) {
  const devices = JSON.stringify(gpu.gpu.devices);
  if (/swiftshader|llvmpipe|lavapipe|software raster|basic render/i.test(devices)) throw new Error(`Software GPU rejected: ${devices}`);
  if (!/Apple M3 Max/i.test(devices) || !/Metal/i.test(devices)) throw new Error(`Expected this qualification target's Apple M3 Max/Metal GPU: ${devices}`);
  if (gpu.gpu.featureStatus.webgpu !== 'enabled') throw new Error(`Hardware WebGPU unavailable: ${gpu.gpu.featureStatus.webgpu}`);
}

const observeFaults = `document.addEventListener('DOMContentLoaded', () => {
  const status = document.querySelector('#status');
  if (!status) return;
  const inspect = () => { if (/fault:/i.test(status.textContent)) console.error(status.textContent); };
  new MutationObserver(inspect).observe(status, {childList:true,subtree:true,characterData:true}); inspect();
});`;
const readState = `JSON.stringify({status:document.querySelector('#status')?.textContent,player:document.querySelector('#player')?.textContent,width:document.querySelector('canvas')?.width,height:document.querySelector('canvas')?.height,hidden:document.hidden})`;
const playerX = state => { const match = /player x=([-\d.]+)/.exec(state.player ?? ''); return match ? Number(match[1]) : undefined; };
const field = (status, key) => Number(new RegExp(`(?:^|[ ;])${key}=(\\d+)(?:[ ;]|$)`).exec(status)?.[1]);
const gameState = '({report:game.report(),complete:game.complete(),paused:game.paused(),time:game.time_of_day(),fault:game.graphics_fault(),recovering:game.recovering(),recoveryError:game.graphics_recovery_error(),player:game.controlled_position()})';

// A 4x4 opaque RGB PNG of one colour.
function solidPng(rgb) {
  const crc = buffer => { let c = ~0; for (const byte of buffer) { c ^= byte; for (let k = 0; k < 8; k++) c = c & 1 ? (c >>> 1) ^ 0xedb88320 : c >>> 1; } return ~c >>> 0; };
  const chunk = (type, data) => { const body = Buffer.concat([Buffer.from(type), data]); const out = Buffer.alloc(body.length + 8); out.writeUInt32BE(data.length, 0); body.copy(out, 4); out.writeUInt32BE(crc(body), body.length + 4); return out; };
  const header = Buffer.alloc(13); header.writeUInt32BE(4, 0); header.writeUInt32BE(4, 4); header[8] = 8; header[9] = 2;
  const rows = Buffer.concat(Array.from({ length: 4 }, () => Buffer.from([0, ...rgb, ...rgb, ...rgb, ...rgb])));
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', header), chunk('IDAT', zlib.deflateSync(rows)), chunk('IEND', Buffer.alloc(0))]);
}

// Cook the valley into a fresh store under the run directory and serve it with the G5 dev server.
async function serveCooked(args, output) {
  const log = await fs.open(path.join(output, 'cook.log'), 'a');
  // Watch a copy, so the live-edit step never touches repository sources.
  const source = path.join(output, 'source');
  await fs.cp(path.resolve(repo, args['assets-source']), source, { recursive: true });
  const cook = spawn(path.resolve(repo, args.cook), ['watch', source, path.join(output, 'store'), '--serve', '127.0.0.1:0'], { stdio: ['ignore', log.fd, log.fd] });
  await log.close();
  let exited = false;
  cook.once('exit', () => { exited = true; });
  const origin = await waitFor(async () => {
    if (exited) throw new Error('thrum-cook exited before serving');
    const text = await fs.readFile(path.join(output, 'cook.log'), 'utf8');
    const served = /^serving (http:\/\/\S+)$/m.exec(text);
    return served && /generation 1\b/.test(text) ? served[1] : undefined;
  }, 120000, 'cooked valley dev server');
  return { cook, origin, source };
}

async function existingGame(cdp, sessionId) {
  const prototype = await cdp.call('Runtime.evaluate', { expression: "import('/v1_game.js').then(m => m.BrowserGame.prototype)", awaitPromise: true }, sessionId);
  if (prototype.exceptionDetails || !prototype.result.objectId) throw new Error('Existing BrowserGame prototype unavailable');
  const { objects } = await cdp.call('Runtime.queryObjects', { prototypeObjectId: prototype.result.objectId }, sessionId);
  await cdp.call('Runtime.releaseObject', { objectId: prototype.result.objectId }, sessionId);
  const invoke = async (body, args = []) => {
    const result = await cdp.call('Runtime.callFunctionOn', { objectId: objects.objectId, returnByValue: true,
      functionDeclaration: `function(...args){if(this.length!==1)throw new Error(\`Expected one existing BrowserGame, found \${this.length}\`);const game=this[0];${body}}`,
      arguments: args.map(value => ({ value })) }, sessionId);
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  return {
    read: () => invoke(`return ${gameState};`),
    setVisible: visible => invoke('game.set_visible(args[0],performance.now());return true;', [visible]),
    invoke,
  };
}

async function destroyActualDevice(cdp, sessionId, previous, onDestroy) {
  const prototype = await cdp.call('Runtime.evaluate', { expression: 'GPUDevice.prototype' }, sessionId);
  if (prototype.exceptionDetails || !prototype.result.objectId) throw new Error('Actual WebGPU device prototype unavailable');
  const { objects } = await cdp.call('Runtime.queryObjects', { prototypeObjectId: prototype.result.objectId }, sessionId);
  await cdp.call('Runtime.releaseObject', { objectId: prototype.result.objectId }, sessionId);
  const selected = await cdp.call('Runtime.callFunctionOn', { objectId: objects.objectId,
    functionDeclaration: 'function(previous){const fresh=this.filter(device=>!previous?.includes(device));if(fresh.length!==1)throw new Error(`Expected one fresh actual GPUDevice, found ${fresh.length}`);return fresh[0];}',
    arguments: [previous ? { objectId: previous } : { value: null }] }, sessionId);
  if (selected.exceptionDetails || !selected.result.objectId) throw new Error('Unique current GPUDevice unavailable');
  onDestroy();
  await destroyRetainedDevice(cdp, sessionId, selected.result.objectId);
  const count = await cdp.call('Runtime.callFunctionOn', { objectId: objects.objectId,
    functionDeclaration: 'function(){return this.length;}', returnByValue: true }, sessionId);
  return { allDevices: objects.objectId, device: selected.result.objectId, count: count.result.value };
}

async function destroyRetainedDevice(cdp, sessionId, objectId) {
  const result = await cdp.call('Runtime.callFunctionOn', { objectId,
    functionDeclaration: 'function(){this.destroy();return true;}', returnByValue: true }, sessionId);
  if (result.exceptionDetails || result.result.value !== true) throw new Error('Actual WebGPU device destruction failed');
}

async function injectConfigureFailure(cdp, sessionId) {
  const result = await cdp.call('Runtime.evaluate', { expression: `(()=>{
    const prototype=GPUCanvasContext.prototype;
    const descriptor=Object.getOwnPropertyDescriptor(prototype,'configure');
    if(typeof descriptor?.value!=='function'||(!descriptor.configurable&&!descriptor.writable))throw new Error('Canvas configure cannot be wrapped');
    const canvas=document.querySelector('canvas');const state={hits:0};
    state.restored=()=>Object.getOwnPropertyDescriptor(prototype,'configure').value===descriptor.value;
    const wrapper=function(...args){
      if(this.canvas!==canvas)return Reflect.apply(descriptor.value,this,args);
      Object.defineProperty(prototype,'configure',descriptor);state.hits++;
      throw new Error('${configureFailureSentinel}');
    };
    Object.defineProperty(prototype,'configure',{...descriptor,value:wrapper});
    if(prototype.configure!==wrapper)throw new Error('Canvas configure wrapper was not installed');
    return state;
  })()` }, sessionId);
  if (result.exceptionDetails || !result.result.objectId) throw new Error(`Configure injection unavailable: ${JSON.stringify(result.exceptionDetails)}`);
  return result.result.objectId;
}

async function readConfigureInjection(cdp, sessionId, objectId) {
  const result = await cdp.call('Runtime.callFunctionOn', { objectId, returnByValue: true,
    functionDeclaration: 'function(){return {hits:this.hits,restored:this.restored()};}' }, sessionId);
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
}

async function qualifyControls(cdp, sessionId, poll, timeout) {
  const deadline = Date.now() + timeout;
  const until = (check, description) => waitFor(check, Math.max(0, deadline - Date.now()), description);
  const evaluate = async expression => {
    const result = await cdp.call('Runtime.evaluate', { expression, returnByValue: true }, sessionId);
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  await until(async () => /^Running/.test((await poll()).status ?? ''), 'continuous demo startup');
  const game = await existingGame(cdp, sessionId);
  const snapshots = {};
  const read = async name => {
    const state = await poll();
    if (!/^Running/.test(state.status ?? '')) throw new Error(`Continuous demo stopped: ${state.status}`);
    const value = await game.read();
    if (!Number.isFinite(field(value.report, 'ticks'))) throw new Error('Public world tick unavailable');
    snapshots[name] = value;
    return value;
  };
  const hold = async (name, base, paused, presents, milliseconds) => {
    const start = Date.now();
    return until(async () => {
      const value = await read(name);
      if (value.paused !== paused || field(value.report, 'ticks') !== field(base.report, 'ticks')) throw new Error(`World tick or pause preference changed during ${name}: baseline=${JSON.stringify(base)} observed=${JSON.stringify(value)}`);
      return Date.now() - start >= milliseconds && field(value.report, 'presented') >= field(base.report, 'presented') + presents ? value : undefined;
    }, name);
  };
  const clickPause = () => evaluate('document.querySelector("#pause-world").click()');
  const pauseAndSettle = async name => {
    await clickPause();
    const requested = await read(`${name}Requested`);
    // This game's eight-update queue drains in two capped callbacks. Observe
    // three actual presents before the hold check instead of assuming a sleep
    // gave the browser enough callbacks to drain already accepted work.
    return until(async () => {
      const value = await read(name);
      return value.paused && field(value.report, 'presented') >= field(requested.report, 'presented') + 3 ? value : undefined;
    }, `${name} accepted-work drain`);
  };
  const firstPause = await pauseAndSettle('firstPause');
  await hold('pauseWithPresentation', firstPause, true, 12, 600);
  await evaluate('document.querySelector("[data-sky=sunrise]").click();const solar=document.querySelector("#solar-time");solar.value=18.48;solar.dispatchEvent(new Event("input",{bubbles:true}));document.querySelector("#camera-yaw").focus()');
  const before = await evaluate('Number(document.querySelector("#camera-yaw").value)');
  for (const type of ['keyDown', 'keyUp']) await cdp.call('Input.dispatchKeyEvent', { type, key: 'ArrowRight', code: 'ArrowRight', windowsVirtualKeyCode: 39 }, sessionId);
  const after = await evaluate('Number(document.querySelector("#camera-yaw").value)');
  const edited = await hold('edited', firstPause, true, 24, 350);
  if (after !== before + 1 || !Number.isFinite(edited.time) || Math.abs(edited.time - 18.48 / 24) > 1e-6) throw new Error('Paused camera/solar controls did not apply');
  await game.setVisible(false);
  await hold('hiddenPaused', firstPause, true, 0, 350);
  await game.setVisible(true);
  const shown = await read('shown');
  await hold('shownStillPaused', firstPause, true, field(shown.report, 'presented') - field(firstPause.report, 'presented') + 12, 350);
  await game.setVisible(false);
  await clickPause();
  await hold('hiddenResume', firstPause, false, 0, 350);
  await game.setVisible(true);
  await until(async () => {
    const value = await read('resumed');
    return !value.paused && field(value.report, 'ticks') > field(firstPause.report, 'ticks');
  }, 'visible resume advances world');
  const secondPause = await pauseAndSettle('secondPause');
  const complete = await until(async () => {
    const value = await read('complete');
    if (!value.paused || field(value.report, 'ticks') !== field(secondPause.report, 'ticks')) throw new Error('Re-paused world advanced during continuous presentation');
    return value.complete && field(value.report, 'samples') === 600 && field(value.report, 'presented') >= 650 ? value : undefined;
  }, 'continuous presentation beyond 600 GPU samples');
  if (field(complete.report, 'overload_events') !== 0) throw new Error('Manual pause/visibility composition caused overload');
  return { snapshots, rangeKeyboard: { before, after } };
}

// A parameter file applies through the page: valid entries move the controls and the game,
// a typo is explained with the nearest key, and the reload button applies an edited file.
async function qualifyParams(cdp, sessionId, poll, timeout, files, output) {
  const deadline = Date.now() + timeout;
  const until = (check, description) => waitFor(check, Math.max(0, deadline - Date.now()), description);
  const page = `({status:document.querySelector('#params-status').textContent,hidden:document.querySelector('#params').hidden,hours:document.querySelector('#solar-time').valueAsNumber,label:document.querySelector('#solar-time-value').textContent,paused:document.querySelector('#pause-world').checked})`;
  const evaluate = async expression => {
    const result = await cdp.call('Runtime.evaluate', { expression, returnByValue: true }, sessionId);
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  await until(async () => /^Running/.test((await poll()).status ?? ''), 'params demo startup');
  const game = await existingGame(cdp, sessionId);
  const first = await until(async () => { await poll(); const value = await evaluate(page); return value.status ? value : undefined; }, 'first parameter report');
  if (first.hidden || !first.status.startsWith('case.params.json: 2 changed, 0 unchanged, 1 refused') || !first.status.includes('did you mean "camera.yaw"')) throw new Error(`Parameter report wrong: ${first.status}`);
  if (first.hours !== 17.9 || first.label !== '17:54' || !first.paused) throw new Error(`Parameter file did not move the controls: ${JSON.stringify(first)}`);
  const held = await until(async () => { await poll(); const value = await game.read(); return value.paused ? value : undefined; }, 'parameter pause');
  // The pause drains already accepted ticks, so the clock may pass the file's time by a few ticks.
  await screenshot(cdp, sessionId, output, 'params-diagnostic');
  if (!(held.time >= 17.9 / 24 - 1e-6 && held.time < 17.9 / 24 + 30 / 18000)) throw new Error(`Game clock ignored the file: ${held.time}`);
  const bytes = Buffer.from('{"sky.solar_time": 6.5, "world.paused": true}');
  files.set('/case.params.json', { bytes, type: 'application/json', sha256: sha(bytes) });
  await evaluate('document.querySelector("#reload-params").click()');
  const second = await until(async () => { await poll(); const value = await evaluate(page); return value.status.startsWith('case.params.json: 1 changed, 1 unchanged, 0 refused') ? value : undefined; }, 'reloaded parameter report');
  const reloaded = await game.read();
  if (second.label !== '06:30' || Math.abs(reloaded.time - 6.5 / 24) > 1e-6) throw new Error(`Reload did not apply: ${JSON.stringify({ second, time: reloaded.time })}`);
  // The page combines ?assets with ?params: the cooked valley is drawn too.
  const cooked = JSON.parse(await game.invoke('return game.cooked_status();') ?? 'null');
  if (!cooked || !(cooked.drawn >= 1) || cooked.errors.length) throw new Error(`Cooked valley not drawn with ?params: ${JSON.stringify(cooked)}`);
  return { first, second, time: reloaded.time, cooked };
}

async function qualifyDeviceLoss(cdp, sessionId, poll, timeout, late, onDestroy) {
  const deadline = Date.now() + timeout;
  const until = (check, description) => waitFor(check, Math.max(0, deadline - Date.now()), description);
  await until(async () => /^Running/.test((await poll()).status ?? ''), 'loss demo startup');
  const game = await existingGame(cdp, sessionId);
  const snapshots = {};
  const read = async name => {
    await poll();
    const value = await game.read(); snapshots[name] = value; return value;
  };
  await until(async () => {
    const value = await read('beforeLoss');
    return Number.isFinite(playerX(await poll())) && (late ? value.complete : field(value.report, 'presented') >= 12);
  }, late ? 'healthy profile completes before loss' : 'healthy views before loss');
  await destroyActualDevice(cdp, sessionId, undefined, onDestroy);
  const fault = await until(async () => {
    const value = await read('lossObserved');
    return value.fault?.startsWith('Destroyed:') && /^Graphics fault: Destroyed:/.test((await poll()).status ?? '') ? value : undefined;
  }, 'persistent device-loss status');
  const assertStoppedGpu = value => {
    if (value.fault !== fault.fault || field(value.report, 'presented') !== field(fault.report, 'presented') || field(value.report, 'samples') !== field(fault.report, 'samples')) throw new Error(`Lost GPU was used or first fault changed: ${JSON.stringify(value)}`);
  };
  for (const type of ['keyDown', 'keyUp']) {
    const before = await poll();
    await cdp.call('Input.dispatchKeyEvent', { type, code: 'KeyD', key: 'd', windowsVirtualKeyCode: 68 }, sessionId);
    if (type === 'keyDown') {
      snapshots.inputStart = { x: playerX(before) };
      await new Promise(resolve => setTimeout(resolve, 700));
    }
  }
  await until(async () => {
    const value = await read('runningAfterLoss'); assertStoppedGpu(value);
    return field(value.report, 'ticks') > field(fault.report, 'ticks') + 5 && playerX(await poll()) > snapshots.inputStart.x + 0.01;
  }, 'same world and input advance after actual device loss');
  const clickPause = async () => {
    const result = await cdp.call('Runtime.evaluate', { expression: 'document.querySelector("#pause-world").click()' }, sessionId);
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  };
  await clickPause();
  const requested = await read('pauseRequested');
  const paused = await until(async () => {
    const value = await read('pausedAfterLoss'); assertStoppedGpu(value);
    return value.paused && field(value.report, 'callbacks') >= field(requested.report, 'callbacks') + 3 ? value : undefined;
  }, 'accepted ticks drain without GPU');
  await until(async () => {
    const value = await read('heldPause'); assertStoppedGpu(value);
    if (field(value.report, 'ticks') !== field(paused.report, 'ticks')) throw new Error('Paused lost-GPU world advanced');
    return field(value.report, 'callbacks') >= field(paused.report, 'callbacks') + 12;
  }, 'lost-GPU pause holds through ongoing callbacks');
  await game.setVisible(false);
  await clickPause();
  const hidden = await read('hiddenResume');
  await sleep(350);
  const stillHidden = await read('hiddenHeld'); assertStoppedGpu(stillHidden);
  if (stillHidden.paused || field(stillHidden.report, 'ticks') !== field(hidden.report, 'ticks')) throw new Error('Lost GPU hidden gap advanced authority');
  await game.setVisible(true);
  await until(async () => {
    const value = await read('resumedAfterLoss'); assertStoppedGpu(value);
    return !value.paused && field(value.report, 'ticks') > field(paused.report, 'ticks') + 5;
  }, 'lost-GPU visibility resume without GPU polling');
  const beforeDelay = await read('beforeOverload');
  await cdp.call('Runtime.evaluate', { expression: '(()=>{const end=performance.now()+250;while(performance.now()<end){}return true;})()' }, sessionId);
  await until(async () => {
    const value = await read('overloadAfterLoss'); assertStoppedGpu(value);
    return field(value.report, 'overload_events') > field(beforeDelay.report, 'overload_events') && field(value.report, 'refused_ticks') > field(beforeDelay.report, 'refused_ticks');
  }, 'lost-GPU clock overload remains explicit');
  const finalState = await read('final'); assertStoppedGpu(finalState);
  if (late && !snapshots.beforeLoss.complete) throw new Error('Late loss did not follow profile completion');
  return { snapshots, scope: 'Actual GPUDevice.destroy; authority/input/pause/hooks/overload stay live; no graphics reconstruction' };
}

async function qualifyDeviceRecovery(cdp, sessionId, poll, timeout, evidence, onDestroy, failFirst, capturePhase) {
  const deadline = Date.now() + timeout;
  const until = (check, description) => waitFor(check, Math.max(0, deadline - Date.now()), description);
  await until(async () => /^Running/.test((await poll()).status ?? ''), 'recovery demo startup');
  const game = await existingGame(cdp, sessionId); // Retain this exact BrowserGame through both replacements.
  let lastTick = 0, devices, expectedRecoveryFailure = false;
  const read = async name => {
    await poll();
    const value = await game.read(); evidence.snapshots[name] = value;
    const tick = field(value.report, 'ticks');
    if (!Number.isFinite(tick) || tick < lastTick || !Number.isFinite(playerX(value))) throw new Error('World identity/position reset or became unavailable');
    lastTick = tick;
    if (value.recoveryError && !expectedRecoveryFailure) throw new Error(`Graphics replacement failed: ${value.recoveryError}`);
    return value;
  };
  const clickPause = () => game.invoke('document.querySelector("#pause-world").click();return true;');
  const pauseAndSettle = async name => {
    await clickPause(); const requested = await read(`${name}Requested`);
    return until(async () => {
      const value = await read(name);
      return value.paused && field(value.report, 'callbacks') >= field(requested.report, 'callbacks') + 3 ? value : undefined;
    }, `${name} accepted-work drain`);
  };
  await until(async () => field((await read('initial')).report, 'presented') >= 12, 'healthy views before recovery');
  for (let round = 1; round <= (failFirst ? 1 : 2); round++) {
    const before = await read(`beforeLoss${round}`);
    const extent = await poll();
    if (!(extent.width > 0 && extent.height > 0)) throw new Error('Healthy canvas has no extent');
    if (before.fault || before.recovering || field(before.report, 'generation') !== round || field(before.report, 'recovered') !== round - 1) throw new Error('Unexpected graphics generation before loss');
    devices = await destroyActualDevice(cdp, sessionId, devices?.allDevices, onDestroy);
    evidence.destroyedDeviceCounts.push(devices.count);
    const lost = await until(async () => {
      const value = await read(`lost${round}`);
      return value.fault?.startsWith('Destroyed:') && /^Graphics fault: Destroyed:/.test((await poll()).status ?? '') ? value : undefined;
    }, `generation${round} actual device loss`);
    if (failFirst) await capturePhase('lost');
    const assertUnavailable = value => {
      if (value.fault !== lost.fault || field(value.report, 'generation') !== round || field(value.report, 'presented') !== field(lost.report, 'presented') || field(value.report, 'samples') !== field(lost.report, 'samples')) throw new Error('Unavailable generation performed GPU work or changed identity');
    };
    await game.invoke('game.set_controls(1,0);return true;');
    await until(async () => {
      const value = await read(`lostInput${round}`); assertUnavailable(value);
      return field(value.report, 'ticks') >= field(lost.report, 'ticks') + 6 && playerX(value) > playerX(lost) + 0.01;
    }, 'input advances the same lost-GPU world');
    const paused = await pauseAndSettle(`paused${round}`); assertUnavailable(paused);
    let injection;
    if (failFirst) {
      injection = await injectConfigureFailure(cdp, sessionId);
      expectedRecoveryFailure = true;
    }
    // First replacement also covers a zero-extent ready-candidate deferral.
    // Batch edits immediately after the REAL button click, before detached
    // preparation microtasks can complete. No production test hook is installed.
    const request = await game.invoke(`
      const button=document.querySelector('#restore-graphics');
      if(button.hidden||button.disabled)throw new Error('Restore button unavailable');
      button.click();const pending=game.recovering();
      ${round === 1 ? "document.querySelector('canvas').width=0;document.querySelector('canvas').height=0;" : ''}
      game.set_paused(false,performance.now());game.set_controls(1,0);
      ${round === 2 ? 'game.set_paused(true,performance.now());' : ''}
      const solar=document.querySelector('#solar-time');solar.value=18.48;solar.dispatchEvent(new Event('input',{bubbles:true}));
      return {pending,buttonDisabled:button.disabled,snapshot:${gameState}};`);
    evidence.snapshots[`requested${round}`] = request;
    if (!request.pending || !request.buttonDisabled || field(request.snapshot.report, 'recovery_attempts') !== round) throw new Error('Restore button did not admit one asynchronous attempt');
    let retained = request.snapshot;
    if (failFirst) {
      await until(async () => {
        const value = await read('configurePending'); assertUnavailable(value);
        if (!value.recovering) throw new Error('Zero-extent replacement stopped preparing before configure injection');
        return field(value.report, 'callbacks') >= field(request.snapshot.report, 'callbacks') + 3;
      }, 'pending configure attempt with ongoing CPU callbacks');
      await capturePhase('pending');
      await game.invoke('document.querySelector("canvas").width=args[0];document.querySelector("canvas").height=args[1];return true;', [extent.width, extent.height]);
      const expectedError = 'replacement canvas configuration did not become usable';
      const failed = await until(async () => {
        const value = await read('configureFailed'); assertUnavailable(value);
        if (value.recovering) return undefined;
        if (value.recoveryError !== expectedError || field(value.report, 'recovery_attempts') !== 1 || field(value.report, 'recovered') !== 0) throw new Error('Injected configure failure was not retained as a failed attempt');
        return value;
      }, 'injected configure failure ends the first attempt');
      evidence.configureInjection = await readConfigureInjection(cdp, sessionId, injection);
      if (evidence.configureInjection.hits !== 1 || !evidence.configureInjection.restored) throw new Error('Configure wrapper did not throw exactly once and restore itself');
      if (!await game.invoke('const button=document.querySelector("#restore-graphics");return !button.hidden&&!button.disabled;')) throw new Error('Failed attempt did not offer an enabled retry button');
      await capturePhase('failed');
      await until(async () => {
        const value = await read('inputAfterFailure'); assertUnavailable(value);
        if (value.recovering || value.recoveryError !== expectedError || value.paused) throw new Error('Failed attempt lost its diagnostic or changed play state');
        return field(value.report, 'ticks') >= field(failed.report, 'ticks') + 6 && playerX(value) > playerX(failed) + 0.01;
      }, 'failed replacement keeps world and input live without GPU work');
      await pauseAndSettle('pausedAfterFailure');
      const retry = await game.invoke(`
        const button=document.querySelector('#restore-graphics');button.click();
        game.set_paused(false,performance.now());game.set_controls(1,0);game.set_paused(true,performance.now());
        const solar=document.querySelector('#solar-time');solar.value=18.48;solar.dispatchEvent(new Event('input',{bubbles:true}));
        return {pending:game.recovering(),buttonDisabled:button.disabled,snapshot:${gameState}};`);
      evidence.snapshots.retryRequested = retry;
      if (!retry.pending || !retry.buttonDisabled || retry.snapshot.recoveryError || field(retry.snapshot.report, 'recovery_attempts') !== 2) throw new Error('Real-button retry did not clear failure and admit one new attempt');
      expectedRecoveryFailure = false;
      retained = retry.snapshot;
    } else if (round === 1) {
      const pending = await until(async () => {
        const value = await read('zeroExtentPending'); assertUnavailable(value);
        if (!value.recovering) throw new Error('Zero-extent candidate published before a valid surface');
        return field(value.report, 'callbacks') >= field(request.snapshot.report, 'callbacks') + 6 && field(value.report, 'ticks') >= field(paused.report, 'ticks') + 4 && playerX(value) > playerX(paused) + 0.01 ? value : undefined;
      }, 'CPU input and clock continue during zero-extent preparation');
      if (pending.paused) throw new Error('Pending recovery ignored explicit resume');
      await pauseAndSettle('pendingPause');
      retained = await game.invoke(`
        const solar=document.querySelector('#solar-time');solar.value=18.48;solar.dispatchEvent(new Event('input',{bubbles:true}));
        document.querySelector('canvas').width=args[0];document.querySelector('canvas').height=args[1];return ${gameState};`,
      [extent.width, extent.height]);
    }
    evidence.snapshots[`retained${round}`] = retained;
    const targetGeneration = round + 1 + Number(failFirst);
    const targetAttempts = round + Number(failFirst);
    const ready = await until(async () => {
      const value = await read(`restored${round}`);
      if (!value.paused || field(value.report, 'ticks') !== field(retained.report, 'ticks') || Math.abs(playerX(value) - playerX(retained)) > 0.001 || Math.abs(value.time - retained.time) > 1e-6) throw new Error('Replacement restarted or changed paused world/player/solar state');
      if (value.recovering && field(value.report, 'generation') === round) { assertUnavailable(value); return undefined; }
      if (value.fault || field(value.report, 'generation') !== targetGeneration || field(value.report, 'recovered') !== round || field(value.report, 'recovery_attempts') !== targetAttempts) throw new Error('Replacement completion/generation mismatch');
      if (value.recovering) return undefined; // Published generation awaits its first actual present call.
      return field(value.report, 'presented') >= field(lost.report, 'presented') + 12 && field(value.report, 'samples') >= 12 && /^Running/.test((await poll()).status ?? '') ? value : undefined;
    }, `new generation${targetGeneration} presents and collects fresh samples`);
    if (!await game.invoke('const button=document.querySelector("#restore-graphics");return button.hidden&&!button.disabled;')) throw new Error('Restore button did not return to healthy state');
    if (failFirst) await capturePhase('restored');
    await destroyRetainedDevice(cdp, sessionId, devices.device);
    await until(async () => {
      const value = await read(`oldDeviceRecheck${round}`);
      if (value.fault || value.recovering || field(value.report, 'generation') !== targetGeneration || !value.paused || field(value.report, 'ticks') !== field(ready.report, 'ticks')) throw new Error('Retained old device refaulted or changed the new generation');
      return field(value.report, 'presented') >= field(ready.report, 'presented') + 6;
    }, 'healthy new generation survives re-destroy of retained old device');
    await clickPause();
    await until(async () => {
      const value = await read(`resumed${round}`);
      return !value.paused && field(value.report, 'ticks') >= field(ready.report, 'ticks') + 6 && playerX(value) > playerX(ready) + 0.01;
    }, 'held controls survive GPU replacement');
    const released = await game.invoke(`game.set_controls(0,0);return ${gameState};`);
    await until(async () => {
      const value = await read(`released${round}`);
      if (Math.abs(playerX(value) - playerX(released)) > 0.001) throw new Error('Released controls moved the actor');
      return field(value.report, 'ticks') >= field(released.report, 'ticks') + 6;
    }, 'input release remains effective after recovery');
  }
  evidence.snapshots.final = await read('final');
  evidence.scope = failFirst
    ? 'Actual GPUDevice.destroy; one caught main-canvas configure failure, failed attempt2 then real-button retry publishes generation3; same world/input/pause/time and fresh GPU samples. No timing/display claim.'
    : 'Two actual GPUDevice.destroy/button replacements on the same BrowserGame; generations1→2→3, CPU controls/lifecycle during pending zero extent, fresh GPU samples. Old-device re-destroy is not callback replay or display/cadence qualification.';
}

async function screenshot(cdp, sessionId, output, mode, options = {}) {
  const image = await cdp.call('Page.captureScreenshot', { format: 'png', ...(options.clip ? { clip: options.clip, captureBeyondViewport: true } : {}) }, sessionId);
  const bytes = Buffer.from(image.data, 'base64');
  if (options.maximumBytes !== undefined && bytes.length > options.maximumBytes) throw new Error('Screenshot image budget exceeded');
  const file = options.file ?? `${mode}-final.png`;
  await fs.writeFile(path.join(output, file), bytes);
  return { file, bytes: bytes.length, sha256: sha(bytes), width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}

async function qualifySkyReflections(cdp, sessionId, poll, timeout, output, evidence) {
  const deadline = Date.now() + timeout;
  const until = (check, description) => waitFor(check, Math.max(0, deadline - Date.now()), description);
  await until(async () => /^Running/.test((await poll()).status ?? ''), 'continuous sky demo startup');
  const game = await existingGame(cdp, sessionId);
  const controls = `({hours:document.querySelector('#solar-time').valueAsNumber,yaw:document.querySelector('#camera-yaw').valueAsNumber,pitch:document.querySelector('#camera-pitch').valueAsNumber,solarLabel:document.querySelector('#solar-time-value').textContent})`;
  const read = async () => {
    const state = await poll();
    const value = await game.read();
    if (!/^Running/.test(state.status ?? '') || !Number.isFinite(value.time) || value.fault || value.recovering || value.recoveryError || field(value.report, 'generation') !== 1 || field(value.report, 'recovery_attempts') !== 0 || field(value.report, 'recovered') !== 0) throw new Error('Sky capture lost its healthy original graphics generation');
    return value;
  };
  await game.invoke('document.querySelector("#pause-world").click();return true;');
  const requested = await read();
  evidence.pauseRequested = requested;
  const paused = await until(async () => {
    const value = await read();
    return value.paused && field(value.report, 'presented') >= field(requested.report, 'presented') + 3 ? value : undefined;
  }, 'sky pause drains accepted ticks through actual presents');
  evidence.paused = paused;
  const assertPaused = value => {
    if (!value.paused || field(value.report, 'ticks') !== field(paused.report, 'ticks')) throw new Error('Authoritative tick changed during paused sky captures');
  };
  const capture = async (name, file, clip) => {
    const before = await read(); assertPaused(before);
    const image = await screenshot(cdp, sessionId, output, name, { file, clip, maximumBytes: 56 * 1024 * 1024 - evidence.captureBytes });
    evidence.captureBytes += image.bytes;
    const after = await read(); assertPaused(after);
    return { ...image, clip: clip ?? null, before, after };
  };
  const held = async (name, body, expected, fullUi) => {
    const edited = await game.invoke(`${body};return {snapshot:${gameState},controls:${controls}};`);
    assertPaused(edited.snapshot);
    if (![edited.controls.hours, edited.controls.yaw, edited.controls.pitch, edited.snapshot.time].every(Number.isFinite) || Math.abs(edited.controls.hours - expected.hours) > 0.0051 || Math.abs(edited.controls.yaw - expected.yaw) > 0.5001 || Math.abs(edited.controls.pitch - expected.pitch) > 0.5001 || Math.abs(edited.snapshot.time - edited.controls.hours / 24) > 1e-6) throw new Error(`Sky controls did not apply ${name}: ${JSON.stringify(edited)}`);
    const settled = await until(async () => {
      const value = await read(); assertPaused(value);
      if (Math.abs(value.time - edited.snapshot.time) > 1e-6) throw new Error('Solar time changed while the world was paused');
      return field(value.report, 'presented') >= field(edited.snapshot.report, 'presented') + 3 ? value : undefined;
    }, `${name} three present calls after real UI edits`);
    const clip = await game.invoke(`const r=document.querySelector('canvas').getBoundingClientRect();return {x:r.x+scrollX,y:r.y+scrollY,width:r.width,height:r.height,scale:1};`);
    if (![clip.x, clip.y, clip.width, clip.height].every(Number.isFinite) || clip.width <= 0 || clip.height <= 0) throw new Error('Canvas screenshot clip unavailable');
    const record = { name, requested: expected, edited, settled, images: [] };
    // Retain partial metadata before I/O too, so a failed case is reviewable.
    (fullUi ? evidence.phases : evidence.motion).push(record);
    record.images.push(await capture(name, `${evidence.prefix}-${name}.png`, clip));
    if (fullUi) record.images.push(await capture(name, `${evidence.prefix}-${name}-ui.png`));
  };
  for (const [name, preset, sceneView, hours, yaw, pitch] of [
    ['phase-paused-scene-noon', 'noon', true, 12, 0, 0],
    ['phase-sunrise', 'sunrise', false, 6.1, 90, 0],
    ['phase-scene-sunrise', 'sunrise', true, 6.1, 0, 0],
    ['phase-noon', 'noon', false, 12, 0, 20],
    ['phase-sunset', 'sunset', false, 17.9, -90, 0],
    ['phase-twilight', 'twilight', false, 18.48, -90, 0],
    ['phase-scene-twilight', 'twilight', true, 18.48, 0, 0],
  ]) {
    await held(name, `document.querySelector('[data-sky=${preset}]').click();${sceneView ? 'document.querySelector("#scene-view").click();' : ''}`, { hours, yaw, pitch }, true);
  }
  // This is a sampled sequence, not a real-time playback or cadence recording.
  // Range step=1 quantizes camera yaw; record the actual accepted UI values.
  for (let index = 0; index < 24; index++) {
    const fraction = index / 23;
    const hours = 17.5 + 1.3 * fraction;
    const yaw = -10 + 20 * fraction;
    await held(`motion-${String(index).padStart(2, '0')}`, `
      document.querySelector('#scene-view').click();
      for(const [selector,value] of [['#solar-time',${hours}],['#camera-yaw',${yaw}],['#camera-pitch',0]]){
        const control=document.querySelector(selector);control.value=String(value);control.dispatchEvent(new Event('input',{bubbles:true}));
      }`, { hours, yaw, pitch: 0 }, false);
  }
  evidence.final = await read(); assertPaused(evidence.final);
  evidence.scope = 'One continuous built BrowserGame: seven paused held sky/scene phases and 24 range-driven camera/Sun samples, three present calls after every edit; healthy generation 1 and fixed authoritative tick. Canvas PNGs are compositor captures, not physical display cadence, GPU timing or transport-accuracy qualification.';
}

async function qualify(cdp, mode, origin, output, directory, wasm, timeout, assetsOrigin, assetsSource, files) {
  const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
  let sessionId, state, response, input, game, pausedAt, pause, controls, params;
  let failed = false;
  const lossCase = mode === 'device-loss' || mode === 'late-device-loss';
  const retryCase = mode === 'device-recovery-retry';
  const valleyRecovery = mode === 'valley-recovery';
  const recoveryCase = mode === 'device-recovery' || retryCase || valleyRecovery;
  const destroyCase = lossCase || recoveryCase;
  const valleyCase = mode === 'valley';
  const skyCase = mode === 'sky-reflections' || valleyCase;
  const faults = [], timeline = [];
  let loss, recovery, sky, intentionalDestroys = 0;
  const onDestroy = () => { intentionalDestroys++; };
  const url = valleyCase || valleyRecovery ? `${origin}/?assets=${encodeURIComponent(assetsOrigin)}` : mode === 'params' ? `${origin}/?assets=${encodeURIComponent(assetsOrigin)}&params=case.params.json` : mode === 'controls' || destroyCase || skyCase ? `${origin}/` : `${origin}/?qualify${['normal', 'input'].includes(mode) ? '' : `&${mode}-probe`}`;
  const log = line => { timeline.push(line); console.log(`${mode}: ${line}`); };
  try {
    ({ sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true }));
    cdp.event = packet => {
      if (packet.sessionId !== sessionId) return;
      if (packet.method === 'Runtime.exceptionThrown') faults.push(packet.params.exceptionDetails);
      if (packet.method === 'Runtime.consoleAPICalled' && packet.params.type === 'error') {
        const messages = packet.params.args.map(arg => arg.value ?? arg.description);
        const expectedDestroyed = destroyCase && intentionalDestroys > 0 && messages.length === 1 && /^Graphics fault: Destroyed:/.test(messages[0] ?? '');
        const expectedConfigure = retryCase && intentionalDestroys > 0 && messages.length === 1 && /^Surface configuration failed:/.test(messages[0] ?? '') && messages[0].includes(configureFailureSentinel) && messages[0].includes('The surface will report as lost until it is successfully reconfigured.');
        if (!expectedDestroyed && !expectedConfigure) faults.push(messages);
      }
      if (packet.method === 'Network.responseReceived' && packet.params.response.url === `${origin}/v1_game_bg.wasm`) response = packet.params;
    };
    for (const domain of ['Runtime', 'Page']) await cdp.call(`${domain}.enable`, {}, sessionId);
    await cdp.call('Network.enable', { maxResourceBufferSize: wasm.bytes.length * 2, maxTotalBufferSize: wasm.bytes.length * 3 + (valleyCase || valleyRecovery || mode === 'params' ? 256 << 20 : 0) }, sessionId);
    await cdp.call('Network.setCacheDisabled', { cacheDisabled: true }, sessionId);
    await cdp.call('Page.addScriptToEvaluateOnNewDocument', { source: observeFaults }, sessionId);
    await cdp.call('Emulation.setDeviceMetricsOverride', { width: 1280, height: 1100, deviceScaleFactor: 1, mobile: false }, sessionId);
    await cdp.call('Target.activateTarget', { targetId });
    await cdp.call('Page.navigate', { url }, sessionId);
    let last = '', pressedAt, releasedAt;
    const poll = async () => {
      if (interrupted) throw interrupted;
      const evaluated = await cdp.call('Runtime.evaluate', { expression: readState, returnByValue: true }, sessionId);
      if (evaluated.exceptionDetails) throw new Error(JSON.stringify(evaluated.exceptionDetails));
      state = JSON.parse(evaluated.result.value ?? '{}');
      if (state.status !== last) { log(state.status ?? ''); last = state.status; }
      if (faults.length || (/fault:/i.test(state.status ?? '') && !(destroyCase && intentionalDestroys > 0 && /^Graphics fault: Destroyed:/.test(state.status ?? '')))) throw new Error(`Browser fault: ${JSON.stringify(faults)} ${state.status}`);
      return state;
    };
    if (skyCase) {
      sky = { prefix: mode, phases: [], motion: [], captureBytes: 0 };
      await qualifySkyReflections(cdp, sessionId, poll, timeout, output, sky);
      if (valleyCase) {
        const cooked = JSON.parse(await (await existingGame(cdp, sessionId)).invoke('return game.cooked_status();') ?? 'null');
        sky.cooked = cooked;
        if (!cooked || !(cooked.drawn >= 1) || cooked.swaps < 1 || cooked.errors.length) throw new Error(`Cooked valley not drawn: ${JSON.stringify(cooked)}`);
        // Live edit: retint the shared base-colour texture in the watched copy; the page's
        // long poll must fetch and swap in the next generation with only that product uploaded.
        const valleyGame = await existingGame(cdp, sessionId);
        const settle = async () => {
          const start = field((await valleyGame.read()).report, 'presented');
          await waitFor(async () => field((await valleyGame.read()).report, 'presented') >= start + 3 || undefined, timeout, 'three presents');
        };
        await valleyGame.invoke('document.querySelector("[data-sky=noon]").click();document.querySelector("#scene-view").click();return true;');
        await settle();
        await screenshot(cdp, sessionId, output, `${mode}-before-edit`);
        await fs.writeFile(path.join(assetsSource, 'white.png'), solidPng([214, 150, 96]));
        const edited = await waitFor(async () => {
          await poll();
          const value = JSON.parse(await (await existingGame(cdp, sessionId)).invoke('return game.cooked_status();') ?? 'null');
          if (value?.errors.length) throw new Error(`Cooked edit failed: ${JSON.stringify(value)}`);
          return value?.drawn > cooked.drawn ? value : undefined;
        }, timeout, 'edited cooked publication drawn in Chrome');
        sky.cookedEdit = edited;
        if (edited.swaps !== cooked.swaps + 1 || !(edited.uploadedBytes > 0 && edited.uploadedBytes < cooked.uploadedBytes)) throw new Error(`Cooked edit uploaded more than the texture: ${JSON.stringify(edited)}`);
        await settle();
        await screenshot(cdp, sessionId, output, `${mode}-edited`);
      }
    } else if (recoveryCase) {
      recovery = { snapshots: {}, destroyedDeviceCounts: [] };
      const cookedStatus = async () => JSON.parse(await (await existingGame(cdp, sessionId)).invoke('return game.cooked_status();') ?? 'null');
      if (valleyRecovery) {
        recovery.cookedBefore = await waitFor(async () => {
          if (!/^Running/.test((await poll()).status ?? '')) return undefined;
          const value = await cookedStatus();
          if (value?.errors.length) throw new Error(`Cooked valley failed: ${JSON.stringify(value)}`);
          return value?.drawn >= 1 ? value : undefined;
        }, timeout, 'cooked valley drawn before device loss');
      }
      await qualifyDeviceRecovery(cdp, sessionId, poll, timeout, recovery, onDestroy, retryCase, async phase => {
        const name = `${mode}-${phase}`;
        await screenshot(cdp, sessionId, output, name);
        (recovery.captures ??= []).push(`${name}-final.png`);
      });
      if (valleyRecovery) {
        // Each replacement device must prepare the newest fetched generation again: one full-set swap
        // per replacement, the same generation drawn, and frames still submitted on the new device.
        const before = recovery.cookedBefore;
        const after = await waitFor(async () => {
          await poll();
          const value = await cookedStatus();
          if (value?.errors.length) throw new Error(`Cooked valley failed after recovery: ${JSON.stringify(value)}`);
          return value?.swaps >= before.swaps + 2 && value.submittedFrames > before.submittedFrames + 12 ? value : undefined;
        }, timeout, 'cooked valley restaged on both replacement devices');
        recovery.cookedAfter = after;
        if (after.drawn !== before.drawn || after.swaps !== before.swaps + 2 || after.uploadedBytes !== before.uploadedBytes) throw new Error(`Cooked valley restage mismatch: ${JSON.stringify({ before, after })}`);
        await screenshot(cdp, sessionId, output, `${mode}-restored`);
        recovery.captures = [`${mode}-restored-final.png`];
      }
    } else if (lossCase) loss = await qualifyDeviceLoss(cdp, sessionId, poll, timeout, mode === 'late-device-loss', onDestroy);
    else if (mode === 'controls') controls = await qualifyControls(cdp, sessionId, poll, timeout);
    else if (mode === 'params') {
      params = await qualifyParams(cdp, sessionId, poll, timeout, files, output);
      // Parameter problems belong under the controls, never in the game status line.
      if (timeline.some(line => /params|camera\.yw|refused/.test(line))) throw new Error(`Parameter problem reached #status: ${JSON.stringify(timeline)}`);
    }
    else await waitFor(async () => {
      await poll();
      if (mode === 'pause' && /^Paused/.test(state.status ?? '')) {
        if (!game) {
          game = await existingGame(cdp, sessionId);
          const value = await game.read();
          pause = { startTick: field(value.report, 'ticks') };
          pausedAt = Date.now();
        } else if (pause.heldTick === undefined && Date.now() - pausedAt >= 600) {
          const value = await game.read();
          pause.heldTick = field(value.report, 'ticks');
          if (!Number.isFinite(pause.startTick) || pause.heldTick !== pause.startTick || value.complete) throw new Error('Existing game advanced while explicitly paused');
        }
      }
      if (mode === 'input') {
        const x = playerX(state);
        if (pressedAt === undefined && Number.isFinite(x)) {
          input = { initialX: x };
          await cdp.call('Input.dispatchKeyEvent', { type: 'keyDown', code: 'KeyD', key: 'd', windowsVirtualKeyCode: 68 }, sessionId);
          pressedAt = Date.now();
        }
        if (pressedAt !== undefined && releasedAt === undefined && Date.now() - pressedAt >= 700) {
          await cdp.call('Input.dispatchKeyEvent', { type: 'keyUp', code: 'KeyD', key: 'd', windowsVirtualKeyCode: 68 }, sessionId);
          releasedAt = Date.now();
        }
        if (releasedAt !== undefined && Date.now() - releasedAt >= 900 && input.stoppedX === undefined && Number.isFinite(x)) input.stoppedX = x;
      }
      return /^adapter=/.test(state.status ?? '');
    }, timeout, `${mode} 600 completed GPU samples`);
    const report = sky?.final.report ?? recovery?.snapshots.final.report ?? loss?.snapshots.final.report ?? controls?.snapshots.complete.report ?? state.status;
    if (!destroyCase && !skyCase && mode !== 'params' && (field(report, 'samples') !== 600 || !(field(report, 'presented') >= 600))) throw new Error('Qualification requires 600 GPU-complete samples and at least 600 present calls');
    if (state.hidden) throw new Error('Qualification target finished hidden');
    if (mode === 'pause' && (!timeline.some(line => /^Paused/.test(line)) || !timeline.some(line => /^Resumed/.test(line)) || field(state.status, 'overload_events') !== 0 || pause?.heldTick !== pause?.startTick || !Number.isFinite(pause?.heldTick) || !(field(state.status, 'ticks') > pause.heldTick))) throw new Error('Explicit two-second pause/resume was not observed without overload');
    if (mode === 'resize' && (state.width !== 960 || state.height !== 540 || !/extent=960x540;/.test(state.status))) throw new Error('Canvas/surface resize to 960x540 not applied');
    if (mode === 'overload' && !(field(state.status, 'overload_events') > 0 && field(state.status, 'refused_ticks') > 0)) throw new Error('Intentional visible delay did not report refused ticks');
    if (mode === 'input' && !(input?.stoppedX > input?.initialX + 0.01 && Math.abs(playerX(state) - input.stoppedX) <= 0.001)) throw new Error(`Held/released KeyD failed: ${JSON.stringify(input)}, final=${playerX(state)}`);
    if (!response || response.response.status !== 200) throw new Error('Actual Wasm response missing or not HTTP 200');
    const body = await cdp.call('Network.getResponseBody', { requestId: response.requestId }, sessionId);
    const actualWasmSha256 = sha(Buffer.from(body.body, body.base64Encoded ? 'base64' : 'utf8'));
    const disk = sha(await fs.readFile(path.join(directory, 'v1_game_bg.wasm')));
    if (actualWasmSha256 !== disk || disk !== wasm.sha256) throw new Error('Loaded Wasm differs from the built artifact on disk');
    if (sky) sky.actualWasmSha256 = actualWasmSha256;
    if (faults.length) throw new Error(`Runtime faults: ${JSON.stringify(faults)}`);
    game ??= await existingGame(cdp, sessionId);
    if (!lossCase && (await game.read()).fault) throw new Error('Graphics device fault invalidated qualification');
    await screenshot(cdp, sessionId, output, mode);
    if (faults.length) throw new Error(`Runtime faults: ${JSON.stringify(faults)}`);
    const scope = sky?.scope ?? recovery?.scope ?? (lossCase ? loss.scope : mode === 'params' ? 'Parameter file applied and reloaded through the page; no timing claim' : mode === 'controls' ? 'Manual pause with ongoing RAF and explicit visibility-hook composition; not actual tab visibility' : mode === 'pause' ? 'Explicit set_visible(false/true) hook; not actual tab visibility' : 'Headless WebGPU function; not display/cadence qualification');
    const result = { case: mode, url, state, actualWasmSha256, responseStatus: 200, input, pause, controls, params, loss, recovery, sky, timeline, scope };
    await fs.writeFile(path.join(output, `${mode}.json`), JSON.stringify(result, null, 2));
    log(`PASS actualWasmSha256=${actualWasmSha256}`);
    return result;
  } catch (error) {
    failed = true;
    log(`FAIL ${error.message}`);
    await fs.writeFile(path.join(output, `${mode}-failed.json`), JSON.stringify({ url, state, error: String(error), faults, timeline, recovery, sky, wasmResponse: response?.response }, null, 2));
    throw error;
  } finally {
    if (sessionId && failed) {
      try { await screenshot(cdp, sessionId, output, mode); }
      catch (error) { log(`Failure screenshot unavailable: ${error.message}`); }
    }
    await fs.writeFile(path.join(output, `${mode}.log`), timeline.join('\n') + '\n');
    try { await cdp.call('Target.closeTarget', { targetId }); }
    catch (error) { if (!failed) throw error; }
    cdp.event = () => {};
  }
}

async function cleanup(child, exited, cdp, server, profile) {
  const failures = [];
  if (child?.pid && !exited()) {
    if (!cdp) failures.push('Chrome could not be closed through its own DevTools connection');
    try { if (cdp) await cdp.call('Browser.close'); else child.kill('SIGTERM'); } catch { /* Browser.close may close its own CDP connection first. */ }
    try { await waitFor(() => exited(), 10000, 'owned Chrome graceful exit'); }
    catch (error) {
      failures.push(error.message);
      child.kill('SIGTERM');
      try { await waitFor(() => exited(), 3000, 'owned Chrome SIGTERM exit'); }
      catch {
        child.kill('SIGKILL'); failures.push('Owned Chrome needed SIGKILL');
        try { await waitFor(() => exited(), 3000, 'owned Chrome forced exit'); } catch (error) { failures.push(error.message); }
      }
    }
    if (child.exitCode !== 0 || child.signalCode) failures.push(`Chrome did not exit cleanly: code=${child.exitCode} signal=${child.signalCode}`);
  } else if (child?.pid) failures.push('Owned Chrome exited before graceful cleanup');
  cdp?.socket.close();
  if (server) {
    try { await new Promise((resolve, reject) => { const timer = setTimeout(() => { server.closeAllConnections(); reject(new Error('Owned HTTP server did not close')); }, 3000); server.close(error => { clearTimeout(timer); error ? reject(error) : resolve(); }); }); }
    catch (error) { failures.push(error.message); }
  }
  if (profile && !failures.length) {
    try { await fs.rm(profile, { recursive: true }); } catch (error) { failures.push(`Profile cleanup: ${error.message}`); }
  }
  if (failures.length) throw new Error(`Cleanup failed; profile preserved at ${profile}: ${failures.join('; ')}`);
}

async function main() {
  const args = options(process.argv.slice(2));
  if (!args) { console.log('node scripts/check_v1_browser.mjs [--case all|normal|pause|resize|overload|input|controls|params|device-loss|late-device-loss|device-recovery|device-recovery-retry|sky-reflections|valley|valley-recovery] [--web-root target/v1-game-web] [--cook target/debug/thrum-cook] [--assets-source examples/v1-game/assets/showcase] [--chrome /path/to/Chrome] [--output tmp/new-run] [--timeout-ms 90000]\nRequires Node 24+, an already-built game, and this Mac\'s Apple M3 Max/Metal Chrome. Controls checks manual pause with ongoing RAF; params applies, explains and reloads a parameter file; visibility checks use the explicit hook, not tab hiding. Recovery uses the real restore button; retry injects exactly one standard canvas configure failure. Sky reflections saves seven paused phases and 24 camera/Sun samples; valley repeats them on the cooked valley served by thrum-cook; valley-recovery runs device recovery with that valley attached. No timing claim.'); return; }
  if (Number(process.versions.node.split('.')[0]) < 24 || typeof WebSocket !== 'function') throw new Error('Node 24+ with builtin WebSocket is required');
  const directory = await fs.realpath(path.resolve(repo, args['web-root']));
  const files = await assets(directory);
  // The params case's file: two valid entries and one typo; the case rewrites it to test reload.
  const caseParams = Buffer.from('{"world.paused": true, "sky.solar_time": 17.9, "camera.yw": 10}');
  files.set('/case.params.json', { bytes: caseParams, type: 'application/json', sha256: sha(caseParams) });
  const output = path.resolve(repo, args.output ?? `tmp/v1-browser-${Date.now()}`);
  const temp = await fs.realpath(path.join(repo, 'tmp'));
  if (!output.startsWith(temp + path.sep)) throw new Error('--output must name a new directory under repository tmp/');
  const parent = await fs.realpath(path.dirname(output));
  if (parent !== temp && !parent.startsWith(temp + path.sep)) throw new Error('Output parent escapes repository tmp/');
  await fs.mkdir(output);
  const profile = await fs.mkdtemp(path.join(output, 'chrome-profile-'));
  let server, child, cdp, cook, ended = false, failure;
  const results = [];
  try {
    const serving = await serve(files); server = serving.server;
    const chromeLog = await fs.open(path.join(output, 'chrome.log'), 'a');
    let startupError;
    child = spawn(args.chrome, [`--user-data-dir=${profile}`, '--remote-debugging-port=0', '--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-component-update', '--window-size=1280,1100', 'about:blank'], { stdio: ['ignore', chromeLog.fd, chromeLog.fd] });
    child.once('error', error => { startupError = error; ended = true; });
    child.once('exit', () => { ended = true; });
    await chromeLog.close();
    const endpoint = await waitFor(async () => {
      if (interrupted || startupError || ended) throw interrupted ?? startupError ?? new Error('Owned Chrome exited before DevTools startup');
      try { return (await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n'); }
      catch (error) { if (error.code !== 'ENOENT') throw error; return undefined; }
    }, 15000, 'owned Chrome DevTools endpoint');
    if (!/^\d+$/.test(endpoint[0]) || !/^\/devtools\/browser\/[\w-]+$/.test(endpoint[1])) throw new Error('Malformed owned Chrome DevTools endpoint');
    const socket = new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => { socket.close(); reject(new Error('Owned Chrome WebSocket did not open')); }, 15000);
      socket.addEventListener('open', () => { clearTimeout(timer); resolve(); }, { once: true });
      socket.addEventListener('error', error => { clearTimeout(timer); socket.close(); reject(error); }, { once: true });
    });
    cdp = new Cdp(socket);
    const version = await cdp.call('Browser.getVersion');
    const gpu = await cdp.call('SystemInfo.getInfo');
    await fs.writeFile(path.join(output, 'browser-gpu.json'), JSON.stringify({ version, gpu }, null, 2));
    hardware(gpu);
    const modes = args.case === 'all' ? cases : [args.case];
    let assetsOrigin;
    let assetsSource;
    if (modes.includes('valley') || modes.includes('valley-recovery') || modes.includes('params')) ({ cook, origin: assetsOrigin, source: assetsSource } = await serveCooked(args, output));
    for (const mode of modes) results.push(await qualify(cdp, mode, serving.origin, output, directory, files.get('/v1_game_bg.wasm'), args['timeout-ms'], assetsOrigin, assetsSource, files));
  } catch (error) { failure = error; }
  finally {
    cook?.kill();
    try { await cleanup(child, () => ended, cdp, server, profile); }
    catch (error) { failure = failure ? new AggregateError([failure, error], 'Qualification and cleanup failed') : error; }
    await fs.writeFile(path.join(output, 'run.json'), JSON.stringify({ passed: !failure, results, artifacts: Object.fromEntries([...files].map(([name, value]) => [name, value.sha256])), error: failure ? String(failure) : undefined, scope: 'Actual built game; headless Apple M3 Max/Metal WebGPU function only; no timing/cadence claim' }, null, 2));
  }
  if (failure) throw failure;
  console.log(`PASS ${results.length} case(s); artifacts: ${output}`);
}

main().catch(error => { console.error(error); process.exitCode = 1; });
