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
const cases = ['normal', 'pause', 'resize', 'overload', 'input', 'controls', 'tabs', 'params', 'device-loss', 'late-device-loss', 'device-recovery', 'device-recovery-retry', 'sky-reflections', 'valley', 'valley-recovery', 'material', 'material-disabled', 'field', 'field-direct', 'field-occlusion', 'field-oracle', 'route'];
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
    // Requests per path, so a check can prove the page never fetched a file.
    files.requested ??= new Map();
    files.requested.set(pathname, (files.requested.get(pathname) ?? 0) + 1);
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
    objectId: objects.objectId,
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
    const result = await cdp.call('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId);
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
  // The browser view on its own: a step-0 slider is continuous, and every element id a control
  // would emit (its own and a slider's value id) is reserved against other controls, in either
  // order, and against the page's elements; a clash is refused once and not drawn.
  const view = await evaluate(`(async () => {
    const { mountControls } = await import("/thrum-controls.js");
    const control = (key, kind = "slider") => ({ key, kind, group: "", label: key, min: 0, max: 1, step: 0, value: kind === "text" ? "" : 0.5, text: "0.50" });
    const mount = (...list) => {
      const errors = [];
      const box = document.createElement("div");
      const view = mountControls(box, JSON.stringify({ controls: list }),
        { edit() {}, changes() {}, onError: (error) => errors.push(String(error.message)) });
      return { view, errors, rows: box.querySelectorAll(".tc-row").length, ids: [...box.querySelectorAll("[id]")].map((node) => node.id) };
    };
    const continuous = mount(control("sky.a"));
    const input = continuous.view.element("sky.a");
    input.value = 0.37;
    const pairs = {
      underscore: mount(control("sky.a_b"), control("sky_a.b")),
      valueFirst: mount(control("sky.a"), control("sky.a_value", "text")),
      valueSecond: mount(control("sky.a_value", "text"), control("sky.a")),
      page: mount(control("params.status", "text")),
    };
    return { step: input.step, value: input.valueAsNumber, pairs: Object.fromEntries(Object.entries(pairs).map(([name, { errors, rows, ids }]) => [name, { errors, rows, ids }])) };
  })()`);
  if (view.step !== 'any' || Math.abs(view.value - 0.37) > 1e-9) throw new Error(`A step-0 slider is not continuous: ${JSON.stringify(view)}`);
  const refused = (name, rows, fragment) => {
    const pair = view.pairs[name];
    if (pair.rows !== rows || pair.errors.length !== 1 || !pair.errors[0].includes(fragment) || new Set(pair.ids).size !== pair.ids.length) throw new Error(`Control ids (${name}) were not reserved: ${JSON.stringify(pair)}`);
  };
  refused('underscore', 1, '"sky-a-b" is already control sky.a_b');
  refused('valueFirst', 1, 'control sky.a_value cannot be drawn: its element id "sky-a-value" is already control sky.a');
  refused('valueSecond', 1, 'control sky.a cannot be drawn: its element id "sky-a-value" is already control sky.a_value');
  refused('page', 0, '"params-status" is already an element');
  return { snapshots, rangeKeyboard: { before, after }, view };
}

// Activate another real tab. No visibility overrides or direct lifecycle calls.
async function qualifyTabs(cdp, sessionId, targetId, poll, timeout, snapshots) {
  const deadline = Date.now() + timeout;
  const until = (check, description) => waitFor(check, Math.max(0, deadline - Date.now()), description);
  await until(async () => /^Running/.test((await poll()).status ?? ''), 'tab lifecycle startup');
  const game = await existingGame(cdp, sessionId);
  const observation = `{...${gameState},hidden:document.hidden,now:performance.now()}`;
  await game.invoke(`
    document.addEventListener('visibilitychange', () => {
      globalThis.__thrumTabBoundary = ${observation};
    }, {capture: true});
  `);
  const observe = async expression => {
    const state = await poll();
    if (!/^Running/.test(state.status ?? '')) throw new Error(`Tab lifecycle stopped: ${state.status}`);
    return game.invoke(`return ${expression};`);
  };
  const read = () => observe(observation);
  const counter = (state, key) => {
    const value = field(state.report, key);
    if (!Number.isFinite(value)) throw new Error(`Missing ${key} in tab lifecycle report`);
    return value;
  };
  const same = (left, right, keys) => keys.every(key => counter(left, key) === counter(right, key));
  const keyD = type => cdp.call('Input.dispatchKeyEvent', { type, key: 'd', code: 'KeyD', windowsVirtualKeyCode: 68 }, sessionId);
  const visible = async () => {
    const value = await read();
    return !value.hidden ? value : undefined;
  };
  const { targetId: other } = await cdp.call('Target.createTarget', { url: 'about:blank', background: true });
  try {
    await cdp.call('Target.activateTarget', { targetId });
    snapshots.initial = await until(visible, 'game tab initially visible');
    await keyD('keyDown');
    snapshots.moving = await until(async () => {
      const value = await visible();
      return value && playerX(value) > playerX(snapshots.initial) + 0.01 ? value : undefined;
    }, 'held movement before switching tabs');
    const switchTab = async (target, hidden, description) => {
      const before = await read();
      await cdp.call('Target.activateTarget', { targetId: target });
      return until(async () => {
        const { value, edge } = await observe(`{value:${observation},edge:globalThis.__thrumTabBoundary}`);
        return value.hidden === hidden && edge?.hidden === hidden && edge.now >= before.now ?
          { value, edge } : undefined;
      }, description);
    };
    const hiddenCounters = ['ticks', 'callbacks', 'presented', 'refused_ticks'];
    const unchangedWhileHidden = (left, right) => left.paused === right.paused &&
      left.time === right.time && same(left, right, hiddenCounters);
    const hide = async name => {
      const { value: start, edge } = await switchTab(other, true, `${name} document becomes hidden`);
      snapshots[`${name}Start`] = start;
      snapshots[`${name}Boundary`] = edge;
      if (!unchangedWhileHidden(edge, start)) {
        throw new Error(`Game advanced during its hidden visibility event: ${name}`);
      }
      await sleep(900);
      const end = await read();
      snapshots[`${name}End`] = end;
      if (!end.hidden || !unchangedWhileHidden(start, end)) {
        throw new Error(`Game advanced while its tab was hidden: ${name}`);
      }
      return end;
    };
    const show = async (hidden, name) => {
      const { value, edge } = await switchTab(targetId, false, 'game tab becomes visible again');
      snapshots[name] = value;
      snapshots[`${name}Boundary`] = edge;
      if (!unchangedWhileHidden(hidden, edge)) throw new Error(`Game advanced before its visible return event: ${name}`);
      return edge;
    };
    const hidden = await hide('runningHidden');
    if (hidden.paused) throw new Error('Hiding the tab changed manual pause preference');
    const shownEdge = await show(hidden, 'shown');
    const resumed = snapshots.resumed = await until(async () => {
      const value = await visible();
      return value && counter(value, 'ticks') > counter(hidden, 'ticks') + 8 &&
        counter(value, 'presented') >= counter(hidden, 'presented') + 3 ? value : undefined;
    }, 'simulation and presentation resume');
    const admitted = counter(resumed, 'ticks') - counter(hidden, 'ticks');
    const refused = counter(resumed, 'refused_ticks') - counter(hidden, 'refused_ticks');
    // Allow the eight ticks already accepted before the tab was hidden.
    const visibleTickBound = Math.ceil((resumed.now - shownEdge.now) * 60 / 1000) + 8;
    if (admitted + refused > visibleTickBound) throw new Error('Tab return replayed hidden time');
    if (!(Math.abs(playerX(resumed) - playerX(hidden)) <= 0.001)) throw new Error('Tab return left held movement latched');
    if (resumed.paused) throw new Error('Tab return changed manual pause preference');
    await game.invoke('document.querySelector("#pause-world").click();');
    const requested = await read();
    snapshots.paused = await until(async () => {
      const value = await visible();
      // Each callback can drain only one accepted tick under the work slice.
      // Drain the full eight-tick queue before measuring the paused endpoint.
      return value?.paused && counter(value, 'callbacks') >= counter(requested, 'callbacks') + 8 &&
        counter(value, 'presented') >= counter(requested, 'presented') + 3 ? value : undefined;
    }, 'manual pause settles before another tab switch');
    const pausedHidden = await hide('pausedHidden');
    if (!pausedHidden.paused || !same(snapshots.paused, pausedHidden, ['ticks'])) {
      throw new Error('Hiding a manually paused tab changed authority or pause preference');
    }
    await show(pausedHidden, 'pausedShown');
    snapshots.pausedReturn = await until(async () => {
      const value = await visible();
      return value && counter(value, 'presented') >= counter(pausedHidden, 'presented') + 3 ? value : undefined;
    }, 'paused tab presents after return');
    if (!snapshots.pausedReturn.paused || snapshots.pausedReturn.time !== pausedHidden.time ||
        !same(pausedHidden, snapshots.pausedReturn, ['ticks', 'refused_ticks'])) {
      throw new Error('Returning to a manually paused tab advanced authority');
    }
  } finally {
    try {
      await cdp.call('Target.activateTarget', { targetId });
      await keyD('keyUp');
    } finally {
      await cdp.call('Target.closeTarget', { targetId: other });
    }
  }
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

// Decode an 8-bit, non-interlaced RGB or RGBA PNG (Chrome's screenshots) to RGBA rows.
function decodePng(bytes) {
  let offset = 8, width, height, channels;
  const data = [];
  while (offset < bytes.length) {
    const length = bytes.readUInt32BE(offset), type = bytes.toString('latin1', offset + 4, offset + 8);
    const body = bytes.subarray(offset + 8, offset + 8 + length);
    if (type === 'IHDR') {
      width = body.readUInt32BE(0); height = body.readUInt32BE(4);
      if (body[8] !== 8 || ![2, 6].includes(body[9]) || body[12] !== 0) throw new Error('Unsupported screenshot PNG');
      channels = body[9] === 6 ? 4 : 3;
    } else if (type === 'IDAT') data.push(body);
    offset += length + 12;
  }
  const raw = zlib.inflateSync(Buffer.concat(data)), stride = width * channels, rgba = Buffer.alloc(width * height * 4);
  let previous = Buffer.alloc(stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)], line = Buffer.from(raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1)));
    for (let i = 0; i < stride; i++) {
      const a = i >= channels ? line[i - channels] : 0, b = previous[i], c = i >= channels ? previous[i - channels] : 0;
      const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
      line[i] = (line[i] + [0, a, b, (a + b) >> 1, pa <= pb && pa <= pc ? a : pb <= pc ? b : c][filter]) & 255;
    }
    for (let x = 0; x < width; x++) {
      for (let k = 0; k < 3; k++) rgba[(y * width + x) * 4 + k] = line[x * channels + k];
      rgba[(y * width + x) * 4 + 3] = channels === 4 ? line[x * channels + 3] : 255;
    }
    previous = line;
  }
  return { width, height, rgba };
}

// Fraction of pixels whose channels differ by more than 12 between two same-size images.
function changedFraction(first, second) {
  if (first.width !== second.width || first.height !== second.height) throw new Error('Compared captures differ in size');
  let changed = 0;
  for (let i = 0; i < first.rgba.length; i += 4) {
    if (Math.max(...[0, 1, 2].map(k => Math.abs(first.rgba[i + k] - second.rgba[i + k]))) > 12) changed++;
  }
  return changed / (first.width * first.height);
}

const materialHooks = {
  identity: 'fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {\n    return surface;\n}\n',
  // A misspelt identifier on line 3, column 21.
  misspelt: 'fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {\n    var out = surface;\n    out.roughness = roughnes * 0.5;\n    return out;\n}\n',
  green: 'fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {\n    var out = surface;\n    out.base_colour = vec4(0.2, 1.0, 0.3, 1.0);\n    out.roughness = 0.3;\n    return out;\n}\n',
};

// Explicit material reloads on the cooked valley: a visible valid reload, a refused hook that
// leaves the last pixels with its file and line, and a device replaced while a reload is
// outstanding. Animation-frame intervals around the valid reload are recorded; headless Chrome
// on a shared GPU, so they are contended observations, not a performance claim.
// The default page with ?material: the reload is compiled out, so the page exports no material
// methods, shows no material controls and never fetches the hook.
async function qualifyMaterialDisabled(cdp, sessionId, poll, timeout, files) {
  const deadline = Date.now() + timeout;
  const left = () => Math.max(0, deadline - Date.now());
  await waitFor(async () => /^Running/.test((await poll()).status ?? ''), left(), 'default page startup');
  const game = await existingGame(cdp, sessionId);
  const drawn = await waitFor(async () => {
    await poll();
    const text = await game.invoke('return game.cooked_status();');
    const value = text && JSON.parse(text);
    return value?.drawn != null && value.submittedFrames > 120 ? value : undefined;
  }, left(), 'cooked valley drawn on the default page');
  const evidence = { cooked: drawn };
  evidence.exports = await game.invoke('return ["reload_material", "material_report"].filter(name => typeof game[name] === "function");');
  evidence.materialHidden = await game.invoke('return document.querySelector("#material").hidden;');
  evidence.materialRequests = files.requested?.get('/case.material.wgsl') ?? 0;
  if (evidence.exports.length || !evidence.materialHidden || evidence.materialRequests) throw new Error(`The default page started material work: ${JSON.stringify(evidence)}`);
  evidence.final = await game.read();
  evidence.scope = 'Default page loaded with ?material=: no material exports, controls or hook fetch';
  return evidence;
}

async function qualifyMaterial(cdp, sessionId, poll, timeout, files, output, evidence, onDestroy) {
  const deadline = Date.now() + timeout;
  const until = (check, description) => waitFor(check, Math.max(0, deadline - Date.now()), description);
  const evaluate = async expression => {
    const result = await cdp.call('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, sessionId);
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  const materialStatus = () => evaluate('document.querySelector("#material-status").textContent');
  const serve = source => { const bytes = Buffer.from(source); files.set('/case.material.wgsl', { bytes, type: 'text/plain; charset=utf-8', sha256: sha(bytes) }); };
  const awaitStatus = (pattern, description) => until(async () => { await poll(); const text = await materialStatus(); return pattern.test(text) ? text : undefined; }, description);
  await until(async () => /^Running/.test((await poll()).status ?? ''), 'material demo startup');
  const game = await existingGame(cdp, sessionId);
  if (!(await game.invoke('return typeof game.reload_material === "function";'))) throw new Error('The material case needs the development page: build it with examples/v1-game/web/build.sh --dev and pass its v1-game-web-dev folder as --web-root');
  evidence.statuses = {};
  evidence.statuses.initial = await awaitStatus(/material 1 \(case\.material\.wgsl\) drawn/, 'initial identity hook drawn');
  const cooked = JSON.parse(await game.invoke('return game.cooked_status();') ?? 'null');
  if (!cooked || !(cooked.drawn >= 1) || cooked.errors.length) throw new Error(`Cooked valley not drawn: ${JSON.stringify(cooked)}`);
  await game.invoke('document.querySelector("[data-sky=noon]").click();document.querySelector("#scene-view").click();if(!game.paused())document.querySelector("#pause-world").click();return true;');
  await until(async () => (await game.read()).paused, 'paused world');
  const clip = await evaluate('(()=>{const r=document.querySelector("canvas").getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,scale:1};})()');
  const settle = async () => {
    const start = field((await game.read()).report, 'presented');
    await until(async () => { await poll(); return field((await game.read()).report, 'presented') >= start + 8; }, 'eight presents');
  };
  const capture = async name => {
    await settle();
    const shot = await screenshot(cdp, sessionId, output, name, { clip, file: `material-${name}.png` });
    (evidence.captures ??= []).push(shot);
    return decodePng(await fs.readFile(path.join(output, shot.file)));
  };
  const identity = await capture('identity');

  // A visible valid reload through the real button, with the page's animation-frame intervals.
  await evaluate(`(()=>{const log=window.__materialFrames={intervals:[],marks:{}};let last;const tick=t=>{if(last!==undefined)log.intervals.push(t-last);last=t;if(!log.stop)requestAnimationFrame(tick);};requestAnimationFrame(tick);return true;})()`);
  await until(async () => { await poll(); return (await evaluate('window.__materialFrames.intervals.length')) >= 90; }, 'frames before the reload');
  // The shipped moss hook, made unique per run (a sub-visible change to the moss colour) so the
  // browser's and the system's shader caches cannot serve a compile from an earlier run.
  const nonce = 1 + Math.floor(Math.random() * 9000);
  const mossSource = (await fs.readFile(path.join(repo, 'examples/v1-game/materials/moss.material.wgsl'), 'utf8')).replace('vec3(0.10, 0.22, 0.05)', `vec3(0.10, 0.22, ${(0.05 + nonce * 1e-6).toFixed(6)})`);
  if (!mossSource.includes(`${(0.05 + nonce * 1e-6).toFixed(6)}`)) throw new Error('Moss hook no longer has the colour the case varies');
  serve(mossSource);
  await evaluate('window.__materialFrames.marks.request=window.__materialFrames.intervals.length;document.querySelector("#reload-material").click();true');
  evidence.statuses.moss = await awaitStatus(/material 2 \(case\.material\.wgsl\) drawn/, 'moss hook drawn');
  await evaluate('window.__materialFrames.marks.promoted=window.__materialFrames.intervals.length;true');
  await until(async () => { await poll(); return (await evaluate('window.__materialFrames.intervals.length-window.__materialFrames.marks.promoted')) >= 90; }, 'frames after the reload');
  const frames = await evaluate('(()=>{window.__materialFrames.stop=true;return window.__materialFrames;})()');
  const span = (from, to) => { const values = frames.intervals.slice(from, to); const sorted = [...values].sort((a, b) => a - b); return { frames: values.length, medianMs: sorted[sorted.length >> 1], worstMs: sorted[sorted.length - 1] }; };
  const timing = /call ([\d.]+) ms, prepare ([\d.]+) ms, eligible ([\d.]+) ms, drawn ([\d.]+) ms/.exec(evidence.statuses.moss);
  evidence.reload = {
    engineMs: timing && { call: Number(timing[1]), prepare: Number(timing[2]), eligible: Number(timing[3]), drawn: Number(timing[4]) },
    before: span(0, frames.marks.request), during: span(frames.marks.request, frames.marks.promoted + 1), after: span(frames.marks.promoted + 1),
    scope: 'Headless Chrome, shared contended GPU, one run; requestAnimationFrame intervals of the page, including the fetch of the hook',
  };
  const moss = await capture('moss');
  evidence.mossChanged = changedFraction(identity, moss);
  if (!(evidence.mossChanged > 0.02)) throw new Error(`Valid reload was not visible: ${evidence.mossChanged}`);

  // A refused hook: the diagnostic names the hook's file and line, and the last pixels stay.
  serve(materialHooks.misspelt);
  await evaluate('document.querySelector("#reload-material").click();true');
  evidence.statuses.refused = await awaitStatus(/material 3 refused/, 'misspelt hook refused');
  if (!evidence.statuses.refused.includes('case.material.wgsl:3:21:')) throw new Error(`Refusal did not name the hook's file, line and column: ${evidence.statuses.refused}`);
  const refused = await capture('refused');
  evidence.refusedChanged = changedFraction(moss, refused);
  if (evidence.refusedChanged > 0.002) throw new Error(`Refused hook changed the pixels: ${evidence.refusedChanged}`);

  // A device replaced while a reload is outstanding: the request and the destruction run in one
  // task, so the candidate cannot publish; the replacement draws the last accepted hook.
  const prototype = await cdp.call('Runtime.evaluate', { expression: 'GPUDevice.prototype' }, sessionId);
  const { objects } = await cdp.call('Runtime.queryObjects', { prototypeObjectId: prototype.result.objectId }, sessionId);
  await cdp.call('Runtime.releaseObject', { objectId: prototype.result.objectId }, sessionId);
  const selected = await cdp.call('Runtime.callFunctionOn', { objectId: objects.objectId,
    functionDeclaration: 'function(){if(this.length!==1)throw new Error(`Expected one actual GPUDevice, found ${this.length}`);return this[0];}' }, sessionId);
  if (selected.exceptionDetails || !selected.result.objectId) throw new Error('Unique current GPUDevice unavailable');
  onDestroy();
  const outstanding = await cdp.call('Runtime.callFunctionOn', { objectId: selected.result.objectId, returnByValue: true,
    functionDeclaration: 'function(games,source){const serial=games[0].reload_material("case.material.wgsl",source);this.destroy();return serial;}',
    arguments: [{ objectId: game.objectId }, { value: materialHooks.green }] }, sessionId);
  if (outstanding.exceptionDetails || outstanding.result.value !== 4) throw new Error(`Outstanding reload not requested: ${JSON.stringify(outstanding)}`);
  await until(async () => (await game.read()).fault?.startsWith('Destroyed:') && await game.invoke('const button=document.querySelector("#restore-graphics");return !button.hidden&&!button.disabled;'), 'device destroyed with a reload outstanding');
  await game.invoke('document.querySelector("#restore-graphics").click();return true;');
  evidence.statuses.retired = await awaitStatus(/material 4 was prepared for a replaced device/, 'outstanding reload retired');
  await until(async () => { const value = await game.read(); return !value.fault && !value.recovering && /^Running/.test((await poll()).status ?? ''); }, 'replacement device drawing');
  // The accepted hook draws again only once it has validated on the new device.
  evidence.statuses.restored = await awaitStatus(/case\.material\.wgsl: the accepted material draws again/, 'accepted hook restored after validation');
  const restored = await capture('restored');
  evidence.restoredChanged = changedFraction(moss, restored);
  if (evidence.restoredChanged > 0.002) throw new Error(`Replacement device did not draw the accepted hook: ${evidence.restoredChanged}`);
  await destroyRetainedDevice(cdp, sessionId, selected.result.objectId);
  await cdp.call('Runtime.releaseObject', { objectId: objects.objectId }, sessionId);

  // The replacement device takes new reloads.
  serve(materialHooks.green);
  await evaluate('document.querySelector("#reload-material").click();true');
  evidence.statuses.green = await awaitStatus(/material 5 \(case\.material\.wgsl\) drawn/, 'green hook drawn on the replacement device');
  const green = await capture('green');
  evidence.greenChanged = changedFraction(moss, green);
  if (!(evidence.greenChanged > 0.02)) throw new Error(`Reload on the replacement device was not visible: ${evidence.greenChanged}`);
  evidence.final = await game.read();
  evidence.scope = 'Explicit material reloads on the cooked valley through the page; one actual GPUDevice.destroy with a reload outstanding; frame intervals are contended headless observations';
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
  for (const [name, pitch] of [['phase-scene-midnight', 0], ['phase-upslope-midnight', 7]]) {
    await held(name, `
      document.querySelector('#scene-view').click();
      for (const [selector, value] of [['#solar-time', 0], ['#camera-pitch', ${pitch}]]) {
        const control=document.querySelector(selector);
        control.value=String(value);control.dispatchEvent(new Event('input',{bubbles:true}));
      }`, { hours: 0, yaw: 0, pitch }, true);
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
  evidence.scope = 'One continuous built BrowserGame: nine paused held sky/scene phases and 24 range-driven camera/Sun samples, three present calls after every edit; healthy generation 1 and fixed authoritative tick. Canvas PNGs are compositor captures, not physical display cadence, GPU timing or transport-accuracy qualification.';
}

async function qualify(cdp, mode, origin, output, directory, wasm, timeout, assetsOrigin, assetsSource, files) {
  const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
  let sessionId, state, response, input, game, pausedAt, pause, controls, params, tabs;
  let failed = false;
  const lossCase = mode === 'device-loss' || mode === 'late-device-loss';
  const retryCase = mode === 'device-recovery-retry';
  const valleyRecovery = mode === 'valley-recovery';
  const recoveryCase = mode === 'device-recovery' || retryCase || valleyRecovery;
  const destroyCase = lossCase || recoveryCase;
  const valleyCase = mode === 'valley';
  const materialCase = mode === 'material';
  const materialDisabled = mode === 'material-disabled';
  const tabCase = mode === 'tabs';
  const oracleCase = mode.startsWith('field-oracle-');
  const routeCase = mode.startsWith('route-');
  const routeFile = routeCase ? `showcase-${mode.slice('route-'.length)}.params.json` : undefined;
  const skyCase = mode === 'sky-reflections' || valleyCase;
  const faults = [], timeline = [];
  let loss, recovery, sky, material, oracle, route, intentionalDestroys = 0;
  const onDestroy = () => { intentionalDestroys++; };
  const url = routeCase ? `${origin}/?assets=${encodeURIComponent(assetsOrigin)}&params=${routeFile}` : materialCase || materialDisabled ? `${origin}/?assets=${encodeURIComponent(assetsOrigin)}&material=case.material.wgsl` : valleyCase || valleyRecovery ? `${origin}/?assets=${encodeURIComponent(assetsOrigin)}` : mode === 'params' ? `${origin}/?assets=${encodeURIComponent(assetsOrigin)}&params=case.params.json` : mode === 'controls' || tabCase || destroyCase || skyCase ? `${origin}/` : mode === 'field' ? `${origin}/?qualify&field` : mode === 'field-direct' ? `${origin}/?qualify&field&culling=direct` : mode === 'field-occlusion' ? `${origin}/?qualify&field&culling=occlusion` : oracleCase ? `${origin}/?field&culling=${mode.slice('field-oracle-'.length)}&params=field-oracle.params.json` : `${origin}/?qualify${['normal', 'input'].includes(mode) ? '' : `&${mode}-probe`}`;
  const log = line => { timeline.push(line); console.log(`${mode}: ${line}`); };
  try {
    ({ sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true }));
    cdp.event = packet => {
      if (packet.sessionId !== sessionId) return;
      if (packet.method === 'Runtime.exceptionThrown') faults.push(packet.params.exceptionDetails);
      if (packet.method === 'Runtime.consoleAPICalled' && packet.params.type === 'error') {
        const messages = packet.params.args.map(arg => arg.value ?? arg.description);
        const expectedDestroyed = (destroyCase || materialCase || oracleCase) && intentionalDestroys > 0 && messages.length === 1 && /^Graphics fault: Destroyed:/.test(messages[0] ?? '');
        const expectedConfigure = retryCase && intentionalDestroys > 0 && messages.length === 1 && /^Surface configuration failed:/.test(messages[0] ?? '') && messages[0].includes(configureFailureSentinel) && messages[0].includes('The surface will report as lost until it is successfully reconfigured.');
        if (!expectedDestroyed && !expectedConfigure) faults.push(messages);
      }
      if (packet.method === 'Network.responseReceived' && packet.params.response.url === `${origin}/v1_game_bg.wasm`) response = packet.params;
    };
    for (const domain of ['Runtime', 'Page']) await cdp.call(`${domain}.enable`, {}, sessionId);
    await cdp.call('Network.enable', { maxResourceBufferSize: wasm.bytes.length * 2, maxTotalBufferSize: wasm.bytes.length * 3 + (valleyCase || valleyRecovery || materialCase || materialDisabled || routeCase || mode === 'params' ? 256 << 20 : 0) }, sessionId);
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
      if (faults.length || (/fault:/i.test(state.status ?? '') && !((destroyCase || materialCase || oracleCase) && intentionalDestroys > 0 && /^Graphics fault: Destroyed:/.test(state.status ?? '')))) throw new Error(`Browser fault: ${JSON.stringify(faults)} ${state.status}`);
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
    else if (materialCase) {
      material = {};
      await qualifyMaterial(cdp, sessionId, poll, timeout, files, output, material, onDestroy);
    }
    else if (materialDisabled) material = await qualifyMaterialDisabled(cdp, sessionId, poll, timeout, files);
    else if (mode === 'controls') controls = await qualifyControls(cdp, sessionId, poll, timeout);
    else if (tabCase) {
      tabs = {};
      await qualifyTabs(cdp, sessionId, targetId, poll, timeout, tabs);
    }
    else if (oracleCase) oracle = await captureFieldOracle(cdp, sessionId, poll, timeout, output, mode, onDestroy);
    else if (routeCase) {
      route = await qualifyRoute(cdp, sessionId, poll, timeout, output, mode, routeFile, files);
      if (timeline.some(line => /params|refused/.test(line))) throw new Error(`Parameter problem reached #status: ${JSON.stringify(timeline)}`);
    }
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
    const report = material?.final.report ?? sky?.final.report ?? recovery?.snapshots.final.report ?? loss?.snapshots.final.report ?? controls?.snapshots.complete.report ?? state.status;
    if (!destroyCase && !skyCase && !materialCase && !materialDisabled && !oracleCase && !routeCase && !tabCase && mode !== 'params' && (field(report, 'samples') !== 600 || !(field(report, 'presented') >= 600))) throw new Error('Qualification requires 600 GPU-complete samples and at least 600 present calls');
    if (state.hidden) throw new Error('Qualification target finished hidden');
    // The field pages must run the mode they were asked for, and Occlusion must hide: Frustum draws at least 21.6%
    // of the field at every tick of the native flythrough, so Occlusion's median stays under a fifth of active.
    const fieldMode = { field: 'Frustum', 'field-direct': 'Direct', 'field-occlusion': 'Occlusion' }[mode];
    if (fieldMode && /culling=(\w+)/.exec(report)?.[1] !== fieldMode) throw new Error(`${mode} page reports ${/culling=(\w+)/.exec(report)?.[1]}, not ${fieldMode}`);
    if (mode === 'field-occlusion' && !(field(report, 'culling_drawn_p50') * 5 < field(report, 'culling_active'))) throw new Error(`Occlusion hid too little: drawn p50 ${field(report, 'culling_drawn_p50')} of ${field(report, 'culling_active')}`);
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
    const scope = route?.scope ?? material?.scope ?? sky?.scope ?? recovery?.scope ?? (lossCase ? loss.scope : tabCase ? 'Actual Chrome tab activation, document visibility, input release and manual pause; no display cadence claim' : mode === 'params' ? 'Parameter file applied and reloaded through the page; no timing claim' : mode === 'controls' ? 'Manual pause with ongoing RAF and explicit visibility-hook composition; not actual tab visibility' : mode === 'pause' ? 'Explicit set_visible(false/true) hook; not actual tab visibility' : 'Headless WebGPU function; not display/cadence qualification');
    const result = { case: mode, url, state, actualWasmSha256, responseStatus: 200, input, pause, controls, tabs, params, loss, recovery, sky, material, oracle, route, timeline, scope: oracle?.scope ?? scope };
    await fs.writeFile(path.join(output, `${mode}.json`), JSON.stringify(result, null, 2));
    log(`PASS actualWasmSha256=${actualWasmSha256}`);
    return result;
  } catch (error) {
    failed = true;
    log(`FAIL ${error.message}`);
    await fs.writeFile(path.join(output, `${mode}-failed.json`), JSON.stringify({ url, state, error: String(error), faults, timeline, tabs, recovery, sky, material, route, wasmResponse: response?.response }, null, 2));
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

// One fresh page per checked-in showcase preset, as a person launches each stop. The checked-in
// files are the expectation: the web build must ship each one byte for byte and nothing else, so a
// missing or stale built preset fails before any page loads.
const routeSource = path.join(repo, 'examples/v1-game/params');
const routePreset = /^showcase-([a-z-]+)\.params\.json$/;
async function routeStops(files, directory) {
  const presets = async folder => (await fs.readdir(folder)).filter(name => routePreset.test(name)).sort();
  const source = await presets(routeSource);
  if (!source.length) throw new Error(`No showcase-*.params.json presets in ${routeSource}`);
  const built = await presets(directory);
  const missing = source.filter(name => !built.includes(name));
  const extra = built.filter(name => !source.includes(name));
  if (missing.length || extra.length) throw new Error(`Web build route presets differ from the checked-in ones: missing ${JSON.stringify(missing)}, extra ${JSON.stringify(extra)}`);
  files.routeExpected = new Map();
  for (const name of source) {
    const expected = await fs.readFile(path.join(routeSource, name));
    const bytes = await fs.readFile(path.join(directory, name));
    if (sha(bytes) !== sha(expected)) throw new Error(`Web build ships a stale ${name}: ${sha(bytes)}, checked in ${sha(expected)}`);
    files.set(`/${name}`, { bytes, type: 'application/json', sha256: sha(bytes) });
    files.routeExpected.set(name, { bytes: expected, sha256: sha(expected) });
  }
  return source.map(name => `route-${routePreset.exec(name)[1]}`);
}

// Launch the cooked valley with a showcase preset in the URL. The page must report the file
// applied with nothing refused, show its values in the controls, hold the world paused and
// draw the cooked valley; three presents later the canvas is captured at a fixed tick.
async function qualifyRoute(cdp, sessionId, poll, timeout, output, mode, file, files) {
  const deadline = Date.now() + timeout;
  const until = (check, description) => waitFor(check, Math.max(0, deadline - Date.now()), description);
  const source = files.routeExpected.get(file);
  if (!source || files.get(`/${file}`)?.sha256 !== source.sha256) throw new Error(`${mode}: served ${file} is not the checked-in preset`);
  const preset = JSON.parse(source.bytes.toString('utf8'));
  const expected = { hours: preset['sky.solar_time'] ?? 12, yaw: preset['camera.yaw'] ?? 0, pitch: preset['camera.pitch'] ?? 0 };
  if (preset['world.paused'] !== true) throw new Error(`${file} must pause the world`);
  const page = `({status:document.querySelector('#params-status').textContent,hidden:document.querySelector('#params').hidden,hours:document.querySelector('#solar-time').valueAsNumber,yaw:document.querySelector('#camera-yaw').valueAsNumber,pitch:document.querySelector('#camera-pitch').valueAsNumber,paused:document.querySelector('#pause-world').checked})`;
  const { startup, game } = await observeRouteStartup(cdp, sessionId, poll, deadline, mode, file);
  const applied = await until(async () => { await poll(); const value = await game.invoke(`return ${page};`); return value.status ? value : undefined; }, `${mode} parameter report`);
  if (applied.hidden || !new RegExp(`^${file.replaceAll('.', '\\.')}: \\d+ changed, \\d+ unchanged, 0 refused$`).test(applied.status)) throw new Error(`${mode}: parameter report wrong: ${applied.status}`);
  if (!applied.paused || Math.abs(applied.hours - expected.hours) > 0.0051 || Math.abs(applied.yaw - expected.yaw) > 0.5001 || Math.abs(applied.pitch - expected.pitch) > 0.5001) throw new Error(`${mode}: controls do not show the preset: ${JSON.stringify({ applied, expected })}`);
  const cookedStatus = async () => {
    const value = JSON.parse(await game.invoke('return game.cooked_status();') ?? 'null');
    if (value?.errors.length) throw new Error(`${mode}: cooked valley failed: ${JSON.stringify(value)}`);
    return value;
  };
  const drawn = await until(async () => { await poll(); const value = await game.read(); const cooked = await cookedStatus(); return value.paused && cooked?.drawn >= 1 ? { value, cooked } : undefined; }, `${mode} paused with the cooked valley drawn`);
  const read = async () => {
    const value = await game.read();
    if (!value.paused || field(value.report, 'ticks') !== field(drawn.value.report, 'ticks') || value.time !== drawn.value.time) throw new Error(`${mode}: world moved while paused`);
    if (value.fault || value.recovering || value.recoveryError) throw new Error(`${mode}: graphics fault during the route stop`);
    return value;
  };
  // The page may drain already accepted ticks as it pauses; record how far the clock moved.
  if (!(drawn.value.time >= expected.hours / 24 - 1e-6 && drawn.value.time < expected.hours / 24 + 30 / 18000)) throw new Error(`${mode}: game clock ignored the preset: ${drawn.value.time}`);
  const settled = await until(async () => { await poll(); const value = await read(); return field(value.report, 'presented') >= field(drawn.value.report, 'presented') + 3 ? value : undefined; }, `${mode} three presents after the cooked valley drew`);
  const clip = await game.invoke(`const r=document.querySelector('canvas').getBoundingClientRect();return {x:r.x+scrollX,y:r.y+scrollY,width:r.width,height:r.height,scale:1};`);
  if (![clip.x, clip.y, clip.width, clip.height].every(Number.isFinite) || clip.width <= 0 || clip.height <= 0) throw new Error(`${mode}: canvas screenshot clip unavailable`);
  const image = await screenshot(cdp, sessionId, output, mode, { file: `${mode}.png`, clip });
  const after = await read();
  const cooked = await cookedStatus();
  if (!(cooked?.drawn >= drawn.cooked.drawn)) throw new Error(`${mode}: cooked valley stopped drawing: ${JSON.stringify(cooked)}`);
  return { file, preset, sha256: source.sha256, expected, applied, settled, after, cooked, startup, image: { ...image, clip },
    scope: 'Fresh page launched with ?assets and ?params; preset applied with nothing refused, world paused at one tick and the cooked valley drawn before a canvas capture; startup records a polled bracket for navigation to the first cooked draw, not first-paint latency; no frame-time claim' };
}

// Startup proxy for one route page, observed on the page's own clock (performance.now(), milliseconds since its
// navigation started). The page fetches and attaches the cooked valley before it reports Running and starts its frame
// loop, so its two pre-loop labels ('Starting WebGPU…', 'Fetching the cooked valley…') prove nothing cooked is drawn.
// From the moment the page starts fetching the valley, the clock, status and cooked_status() are read in one call. The
// first draw lies between the last observation that establishes no draw (a pre-loop label, or a present
// cooked_status() with no generation drawn) and the first with one: the latter is an upper bound, and the gap is the
// observation granularity. A status-only read with any other label establishes nothing. If no observation establishes
// the lower bound, it is recorded as absent and navigation start (0) is the conservative bound, with no granularity.
// THRUM_ROUTE_OBSERVE_DELAY_MS, for testing that case only, delays the first observation of the route page and is
// recorded. This is a CDP polling proxy, not physical first-paint latency.
async function observeRouteStartup(cdp, sessionId, poll, deadline, mode, file) {
  // Only the page's two exact pre-loop labels prove the frame loop has not started; any other status proves nothing.
  const classify = status => status === 'Starting WebGPU…' ? 'loading' : status === 'Fetching the cooked valley…' ? 'fetching'
    : /^Running/.test(status ?? '') ? 'running' : 'other';
  const observe = `JSON.stringify({now:performance.now(),origin:performance.timeOrigin,href:location.href,status:document.querySelector('#status')?.textContent ?? null})`;
  const delayMs = Number(process.env.THRUM_ROUTE_OBSERVE_DELAY_MS ?? 0);
  if (!Number.isSafeInteger(delayMs) || delayMs < 0 || delayMs > 10000) throw new Error('THRUM_ROUTE_OBSERVE_DELAY_MS must be 0..10000');
  const rows = [];
  let game, origin, lastWithout, firstWith, final, delayed = delayMs === 0, phases = {};
  while (!firstWith) {
    if (Date.now() > deadline) throw new Error(`Timed out: ${mode} first cooked draw`);
    await poll();
    if (!game) {
      const evaluated = await cdp.call('Runtime.evaluate', { expression: observe, returnByValue: true }, sessionId);
      if (evaluated.exceptionDetails) throw new Error(JSON.stringify(evaluated.exceptionDetails));
      const value = JSON.parse(evaluated.result.value);
      // Until the route page commits, the old document answers; its clock is not this page's.
      if (!value.href.includes(`params=${file}`)) { await sleep(5); continue; }
      if (!delayed) { delayed = true; await sleep(delayMs); continue; }
      if (origin === undefined) origin = value.origin;
      if (value.origin !== origin) throw new Error(`${mode}: page clock origin changed during startup`);
      const phase = classify(value.status);
      const preLoop = phase === 'loading' || phase === 'fetching';
      phases[phase] ??= value.now;
      // Only a pre-loop label establishes that nothing is drawn; a Running or other status may follow a draw.
      if (preLoop) lastWithout = value.now;
      rows.push({ now: value.now, phase, drawn: preLoop ? 0 : null });
      // The game exists once the page starts fetching the valley; find it then, well before the first draw.
      if (phase === 'loading') { await sleep(5); continue; }
      game = await existingGame(cdp, sessionId);
    }
    const value = await game.invoke(`return {now:performance.now(),origin:performance.timeOrigin,status:document.querySelector('#status')?.textContent ?? null,cooked:game.cooked_status()};`);
    if (origin === undefined) origin = value.origin;
    if (value.origin !== origin) throw new Error(`${mode}: page clock origin changed during startup`);
    const cooked = JSON.parse(value.cooked ?? 'null');
    if (cooked?.errors.length) throw new Error(`${mode}: cooked valley failed during startup: ${JSON.stringify(cooked)}`);
    const phase = classify(value.status);
    phases[phase] ??= value.now;
    // cooked_status().drawn is the generation drawn, null in a present report until the first is drawn: that report
    // establishes no draw. A pre-loop label proves it too; otherwise an absent report is unknown.
    // Rows use 0 for an established no-draw, the generation once drawn, and null for unknown.
    const drawn = cooked ? cooked.drawn ?? 0 : phase === 'loading' || phase === 'fetching' ? 0 : null;
    rows.push({ now: value.now, phase, drawn });
    if (drawn >= 1) { firstWith = value.now; final = cooked; } else { if (drawn === 0) lastWithout = value.now; await sleep(5); }
  }
  // Where the time went: the navigation, the Wasm and the cooked products, from the page's own timing entries.
  const timing = await game.invoke(`const nav=performance.getEntriesByType('navigation')[0];
    const pick=e=>({name:e.name,start:e.startTime,end:e.responseEnd,transfer:e.transferSize,decoded:e.decodedBodySize});
    const resources=performance.getEntriesByType('resource');
    const cooked=resources.filter(e=>!e.name.startsWith(location.origin));
    return {navigation:nav&&{responseEnd:nav.responseEnd,domContentLoaded:nav.domContentLoadedEventEnd,load:nav.loadEventEnd,transfer:nav.transferSize},
      wasm:resources.filter(e=>e.name.endsWith('/v1_game_bg.wasm')).map(pick),
      cooked:{count:cooked.length,start:Math.min(...cooked.map(e=>e.startTime)),end:Math.max(...cooked.map(e=>e.responseEnd)),
        sizes:'not exposed: the cooked server is cross-origin and sends no Timing-Allow-Origin; see uploadedBytes'}};`);
  const gaps = rows.slice(1).map((row, index) => row.now - rows[index].now);
  return {
    game,
    startup: {
      clock: 'page performance.now(), ms since navigation start; timeOrigin recorded', timeOrigin: origin,
      lowerObserved: lastWithout !== undefined, lowerMs: lastWithout ?? 0, upperMs: firstWith,
      granularityMs: lastWithout === undefined ? null : firstWith - lastWithout, observeDelayMs: delayMs,
      medianPollMs: gaps.sort((a, b) => a - b)[Math.floor(gaps.length / 2)] ?? null, polls: rows.length,
      phases, timing, uploadedBytes: final.uploadedBytes, rows,
      cache: 'HTTP cache disabled for the page (Network.setCacheDisabled); one fresh Chrome profile per checker run',
      scope: 'Headless CDP polling proxy for navigation to the first observed cooked draw, which lies between lowerMs and upperMs: the gap is the granularity when a lower observation exists; otherwise only the navigation-start bound holds. Not physical first-paint latency.',
    },
  };
}

// Field oracle views: headings all round, level and tilted down, at the paused tick.
const oracleViews = [-180, -135, -90, -45, 0, 45, 90, 135].flatMap(yaw => [[yaw, 0], [yaw, -25]]);

async function captureFieldOracle(cdp, sessionId, poll, timeout, output, mode, onDestroy) {
  await waitFor(async () => /^Running/.test((await poll()).status ?? '') || undefined, timeout, `${mode} startup`);
  const game = await existingGame(cdp, sessionId);
  const first = await game.read();
  if (!first.paused) throw new Error(`${mode}: world not paused before the first frame`);
  const captures = [];
  for (const [yaw, pitch] of oracleViews) {
    const identity = await game.invoke('game.set_camera_yaw(args[0]);game.set_camera_pitch(args[1]);return game.begin_capture();',
      [yaw * Math.PI / 180, pitch * Math.PI / 180]);
    // Keep the finished pixels in the page and move them in bounded chunks;
    // one multi-megabyte DevTools message closes the connection.
    const length = await waitFor(async () => {
      await poll();
      return await game.invoke('const pixels=game.try_finish_capture();if(!pixels)return undefined;globalThis.__fieldOracle=pixels;return pixels.length;') ?? undefined;
    }, timeout, `${mode} capture yaw ${yaw} pitch ${pitch}`);
    const chunks = [];
    for (let offset = 0; offset < length; offset += 1 << 18) {
      chunks.push(Buffer.from(await game.invoke(`const part=globalThis.__fieldOracle.subarray(args[0],args[0]+${1 << 18});
        let text='';for(let i=0;i<part.length;i+=0x8000)text+=String.fromCharCode.apply(null,part.subarray(i,i+0x8000));return btoa(text);`, [offset]), 'base64'));
    }
    await game.invoke('delete globalThis.__fieldOracle;return true;');
    const pixels = Buffer.concat(chunks);
    if (pixels.length !== length) throw new Error(`${mode}: capture transfer lost bytes`);
    const name = `${mode}-yaw${yaw}-pitch${pitch}.rgba`;
    await fs.writeFile(path.join(output, name), pixels);
    captures.push({ yaw, pitch, identity, bytes: pixels.length, sha256: sha(pixels), file: name });
  }
  const last = await game.read();
  if (!last.paused || field(last.report, 'ticks') !== field(first.report, 'ticks')) throw new Error(`${mode}: world advanced during capture`);
  const ticks = new Set(captures.map(capture => /tick=(\d+)/.exec(capture.identity)?.[1]));
  if (ticks.size !== 1) throw new Error(`${mode}: captures span ticks ${[...ticks]}`);
  // The prepared renderer's mode and its read-back draw counts, so the oracle cannot compare Direct with Direct.
  const culling = /culling=(\w+)/.exec(last.report)?.[1];
  const drawn = /culling_drawn_p50=(\d+)/.exec(last.report)?.[1];
  const active = /culling_active=(\d+)/.exec(last.report)?.[1];
  const loss = mode === 'field-oracle-frustum' ? await fieldOracleLoss(cdp, sessionId, poll, timeout, game, onDestroy) : undefined;
  return { captures, culling, drawn: drawn && Number(drawn), active: active && Number(active), loss, scope: 'Paused field captured through the page at fixed views; equality is judged against the other culling mode' };
}

// A capture outstanding across an actual GPUDevice.destroy: frames run first so its readback can
// complete, then the loss signal refuses both finish and begin before replacement, and the
// replacement device captures afresh under a new generation.
async function fieldOracleLoss(cdp, sessionId, poll, timeout, game, onDestroy) {
  const thrown = async source => game.invoke(`try{${source};return null;}catch(error){return String(error?.message??error);}`);
  const before = await game.invoke('return game.begin_capture();');
  // Animation frames keep polling the healthy device meanwhile, so the readback can complete.
  const presented = field((await game.read()).report, 'presented');
  await waitFor(async () => field((await game.read()).report, 'presented') >= presented + 4 || undefined, timeout, 'frames before oracle loss');
  const devices = await destroyActualDevice(cdp, sessionId, undefined, onDestroy);
  await waitFor(async () => (await game.read()).fault?.startsWith('Destroyed:') || undefined, timeout, 'oracle device destroyed');
  const finish = await thrown('game.try_finish_capture()');
  if (!/lost before the capture finished/.test(finish ?? '')) throw new Error(`Capture finished after loss: ${finish}`);
  const begin = await thrown('game.begin_capture()');
  if (!/device is lost/.test(begin ?? '')) throw new Error(`Capture began after loss: ${begin}`);
  const empty = await game.invoke('return game.try_finish_capture()===undefined;');
  if (!empty) throw new Error('A refused capture stayed outstanding');
  await game.invoke('document.querySelector("#restore-graphics").click();return true;');
  await waitFor(async () => { const value = await game.read(); return (!value.fault && !value.recovering && /^Running/.test((await poll()).status ?? '')) || undefined; }, timeout, 'oracle replacement device');
  const after = await game.invoke('return game.begin_capture();');
  const length = await waitFor(async () => { await poll(); return await game.invoke('const pixels=game.try_finish_capture();return pixels?pixels.length:undefined;') ?? undefined; }, timeout, 'replacement capture');
  const generation = identity => /generation=(\d+)/.exec(identity)?.[1];
  const tick = identity => /tick=(\d+)/.exec(identity)?.[1];
  if (generation(after) === generation(before) || tick(after) !== tick(before)) throw new Error(`Replacement capture identity: ${before} -> ${after}`);
  for (const objectId of [devices.device, devices.allDevices]) await cdp.call('Runtime.releaseObject', { objectId }, sessionId);
  return { before, finish, begin, after, bytes: length };
}

async function compareFieldOracle(output, results) {
  const direct = results.find(result => result.case === 'field-oracle-direct')?.oracle;
  const culled = { Frustum: results.find(result => result.case === 'field-oracle-frustum')?.oracle, Occlusion: results.find(result => result.case === 'field-oracle-occlusion')?.oracle };
  if (!direct || !culled.Frustum || !culled.Occlusion) throw new Error('Field oracle needs all three culling modes');
  if (direct.culling !== 'Direct' || direct.drawn !== undefined) throw new Error(`Direct page reports ${direct.culling} with drawn ${direct.drawn}`);
  for (const [mode, page] of Object.entries(culled)) if (page.culling !== mode || !(page.drawn > 0 && page.drawn < page.active)) throw new Error(`${mode} page reports ${page.culling}, drawn ${page.drawn} of ${page.active}`);
  if (!(culled.Occlusion.drawn < culled.Frustum.drawn)) throw new Error(`Occlusion drew ${culled.Occlusion.drawn}, not fewer than Frustum's ${culled.Frustum.drawn}`);
  const views = [];
  for (const [index, reference] of direct.captures.entries()) {
    const view = { yaw: reference.yaw, pitch: reference.pitch, identity: reference.identity };
    for (const [mode, page] of Object.entries(culled)) {
      const capture = page.captures[index];
      const strip = identity => identity.replace(/;generation=\d+/, '');
      if (strip(reference.identity) !== strip(capture.identity) || reference.yaw !== capture.yaw || reference.pitch !== capture.pitch) throw new Error(`Field oracle views differ: ${JSON.stringify({ reference, capture })}`);
      let differing = 0;
      if (reference.sha256 !== capture.sha256) {
        const [a, b] = await Promise.all([reference.file, capture.file].map(file => fs.readFile(path.join(output, file))));
        for (let offset = 0; offset < Math.max(a.length, b.length); offset += 4) if (a.readUInt32LE(offset) !== b.readUInt32LE(offset)) differing++;
      }
      view[mode] = { equal: differing === 0, differing };
    }
    views.push(view);
  }
  const counts = mode => ({ drawn: culled[mode].drawn, active: culled[mode].active });
  const result = { case: 'field-oracle', views, culling: { direct: direct.culling, frustum: counts('Frustum'), occlusion: counts('Occlusion') }, scope: 'Chrome Frustum and Occlusion each against Chrome Direct, byte-equal RGBA8 captures at the same committed tick, view and extent; each view change leaves Occlusion one frame of stale history' };
  await fs.writeFile(path.join(output, 'field-oracle.json'), JSON.stringify(result, null, 2));
  const failing = views.filter(view => !view.Frustum.equal || !view.Occlusion.equal);
  if (failing.length) throw new Error(`Field oracle: a culled mode differs from Direct at ${JSON.stringify(failing)}`);
  console.log(`field-oracle: PASS ${views.length} views byte-equal for Frustum and Occlusion at ${views[0].identity}`);
  return result;
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
  if (!args) { console.log('node scripts/check_v1_browser.mjs [--case all|normal|pause|resize|overload|input|controls|tabs|params|device-loss|late-device-loss|device-recovery|device-recovery-retry|sky-reflections|valley|valley-recovery|material|material-disabled|field|field-direct|field-occlusion|field-oracle|route] [--web-root target/v1-game-web] [--cook target/debug/thrum-cook] [--assets-source examples/v1-game/assets/showcase] [--chrome /path/to/Chrome] [--output tmp/new-run] [--timeout-ms 90000]\nRequires Node 24+, an already-built game, and this Mac\'s Apple M3 Max/Metal Chrome. Controls checks manual pause with ongoing RAF; tabs switches real Chrome tabs and checks authority, input release and manual pause; params applies, explains and reloads a parameter file; visibility checks use the explicit hook, not tab hiding. Recovery uses the real restore button; retry injects exactly one standard canvas configure failure. Sky reflections saves nine paused phases and 24 camera/Sun samples; valley repeats them on the cooked valley served by thrum-cook; valley-recovery runs device recovery with that valley attached; material reloads a WGSL hook on that valley (valid, refused, and outstanding across a destroyed device) and records contended frame intervals around the valid reload; it needs the development page (web/build.sh --dev), so all leaves it out. material-disabled loads the default page with ?material and proves it starts no material work. Field qualifies the 100,513-instance flythrough with Frustum camera culling; field-direct and field-occlusion qualify it with Direct and Occlusion for a paired comparison; field-oracle pauses three fresh pages and compares Frustum and Occlusion captures with Direct byte for byte. Route launches one fresh page per checked-in examples/v1-game/params/showcase-*.params.json preset with the cooked valley, after requiring the web root to ship each byte for byte and no others; it proves the preset applied and the valley drew, brackets navigation to the first observed cooked draw on the page clock, and captures the paused canvas. The other cases make no timing claim.'); return; }
  if (Number(process.versions.node.split('.')[0]) < 24 || typeof WebSocket !== 'function') throw new Error('Node 24+ with builtin WebSocket is required');
  const directory = await fs.realpath(path.resolve(repo, args['web-root']));
  const files = await assets(directory);
  // The params case's file: two valid entries and one typo; the case rewrites it to test reload.
  const caseParams = Buffer.from('{"world.paused": true, "sky.solar_time": 17.9, "camera.yw": 10}');
  files.set('/case.params.json', { bytes: caseParams, type: 'application/json', sha256: sha(caseParams) });
  // The field oracle's pages pause the world before their first animation frame.
  const oracleParams = Buffer.from('{"world.paused": true}');
  files.set('/field-oracle.params.json', { bytes: oracleParams, type: 'application/json', sha256: sha(oracleParams) });
  // The material case starts from the identity hook and rewrites the file for each reload.
  const caseMaterial = Buffer.from(materialHooks.identity);
  files.set('/case.material.wgsl', { bytes: caseMaterial, type: 'text/plain; charset=utf-8', sha256: sha(caseMaterial) });
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
    // `all` checks the default page; `material` needs the development page, so it runs on its own.
    const modes = [];
    for (const mode of args.case === 'all' ? cases.filter(mode => mode !== 'material') : [args.case]) {
      modes.push(...(mode === 'field-oracle' ? ['field-oracle-direct', 'field-oracle-frustum', 'field-oracle-occlusion'] : mode === 'route' ? await routeStops(files, directory) : [mode]));
    }
    let assetsOrigin;
    let assetsSource;
    if (modes.some(mode => ['valley', 'valley-recovery', 'params', 'material', 'material-disabled'].includes(mode) || mode.startsWith('route-'))) ({ cook, origin: assetsOrigin, source: assetsSource } = await serveCooked(args, output));
    for (const mode of modes) results.push(await qualify(cdp, mode, serving.origin, output, directory, files.get('/v1_game_bg.wasm'), args['timeout-ms'], assetsOrigin, assetsSource, files));
    if (modes.includes('field-oracle-direct')) results.push(await compareFieldOracle(output, results));
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
