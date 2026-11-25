// Qualify the actual kit scene on hardware Chrome and save submitted output.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import { Cdp, hardware, waitFor, cleanup, serve } from './check_v1_browser.mjs';
import { captureReceiver } from './browser_capture_receiver.mjs';

const mode = process.argv[3];
assert(process.argv[2] === '--case' && ['view', 'inspection'].includes(mode));
const output = path.resolve(process.env.THRUM_MIRROR_OUTPUT ?? `tmp/scene-views-m3/${mode}-${Date.now()}`);
await fs.mkdir(output, { recursive: true });
const probeFiles = new Map();
if (mode === 'view') {
  const rustc = execFileSync('rustup', ['which', '--toolchain', 'stable', 'rustc'], { encoding: 'utf8' }).trim();
  const env = { ...process.env, PATH: `${path.dirname(rustc)}:${process.env.PATH}` };
  const messages = execFileSync('cargo', ['test', '--no-run', '--locked', '--offline', '--release',
    '-p', 'mirror-scene', '--test', 'pool_gpu', '--target', 'wasm32-unknown-unknown',
    '--message-format=json-render-diagnostics'], { encoding: 'utf8', maxBuffer: 4 * 1024 ** 2, env });
  const artifact = messages.trim().split('\n').map(line => JSON.parse(line))
    .find(row => row.reason === 'compiler-artifact' && row.target.name === 'pool_gpu' && row.executable);
  assert(artifact?.executable, 'browser proof artifact was not built');
  execFileSync('wasm-bindgen', ['--target', 'web', '--out-name', 'browser_probe',
    '--out-dir', output, artifact.executable], { stdio: 'inherit', env });
  for (const [name, type] of [['browser_probe.js', 'text/javascript'], ['browser_probe_bg.wasm', 'application/wasm']]) {
    probeFiles.set(`/${name}`, { bytes: await fs.readFile(path.join(output, name)), type });
  }
}
const log = await fs.open(path.join(output, 'scene.log'), 'w+');
const served = process.env.THRUM_MIRROR_URL;
const scene = served ? null : spawn('./dev', ['scene', 'examples/mirror', '--web'], {
  env: { ...process.env, THRUM_SCENE_OUTPUT: output, THRUM_SCENE_WEB_CAPTURE: '1' },
  detached: true,
  stdio: ['ignore', log.fd, log.fd],
});
let chrome, cdp, profile, receiver, ended = false;
const results = {
  commit: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
  extent: [2560, 1440],
  records: [],
  export_slots: 4,
  export_peak: 0,
  output_allowance: 4 * 1024 ** 3,
};
let captureError;
try {
  const url = served ?? await waitFor(async () => {
    const records = await fs.readFile(path.join(output, 'scene.log'), 'utf8');
    if (scene.exitCode !== null) throw Error(records);
    return /SCENE_URL=(\S+)/.exec(records)?.[1];
  }, 600000, 'mirror scene URL');
  const sha = bytes => createHash('sha256').update(bytes).digest('hex');
  const config = await fetch(new URL('config.json', url)).then(response => response.json());
  results.inputs = {
    scene_wasm: sha(Buffer.from(await fetch(new URL('scene_bg.wasm', url)).then(response => response.arrayBuffer()))),
    store_head: sha(Buffer.from(await fetch(new URL('/head', config.store)).then(response => response.arrayBuffer()))),
  };
  if (mode === 'view') results.inputs.probe_wasm = sha(probeFiles.get('/browser_probe_bg.wasm').bytes);
  profile = await fs.mkdtemp(path.join(output, 'chrome-'));
  const chromeLog = await fs.open(path.join(output, 'chrome.log'), 'w');
  chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
    `--user-data-dir=${profile}`, '--headless=new', '--remote-debugging-port=0',
    '--no-first-run', '--no-default-browser-check', 'about:blank',
  ], { stdio: ['ignore', chromeLog.fd, chromeLog.fd] });
  await chromeLog.close();
  chrome.on('exit', () => { ended = true; });
  const [port, endpoint] = await waitFor(async () => {
    if (ended) throw Error('Chrome exited');
    try {
      return (await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n');
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
  }, 20000, 'Chrome DevTools');
  const socket = new WebSocket(`ws://127.0.0.1:${port}${endpoint}`);
  await new Promise((resolve, reject) => {
    socket.onopen = resolve;
    socket.onerror = reject;
  });
  cdp = new Cdp(socket);
  results.browser = await cdp.call('Browser.getVersion');
  results.gpu = await cdp.call('SystemInfo.getInfo');
  hardware(results.gpu);
  const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
  results.consoleErrors = [];
  cdp.event = packet => {
    if (packet.sessionId === sessionId && packet.method === 'Runtime.consoleAPICalled' && packet.params.type === 'error') {
      results.consoleErrors.push(packet.params.args.map(item => item.value ?? item.description).join(' '));
    }
  };
  await cdp.call('Runtime.enable', {}, sessionId);
  await cdp.call('Page.enable', {}, sessionId);
  await cdp.call('Page.addScriptToEvaluateOnNewDocument', { source: `
    globalThis.mirrorErrors = [];
    globalThis.mirrorSubmits = 0;
    const submit = GPUQueue.prototype.submit;
    GPUQueue.prototype.submit = function(...args) {
      mirrorSubmits++;
      return submit.apply(this, args);
    };
    const request = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = async function(...args) {
      const device = await request.apply(this, args);
      device.addEventListener('uncapturederror', event => mirrorErrors.push(event.error.message));
      const info = this.info;
      globalThis.mirrorAdapter = {vendor: info.vendor, architecture: info.architecture, fallback: info.isFallbackAdapter};
      return device;
    };
    // A probe owns the opportunities after startup; the ordinary kit still
    // prepares them and uses its normal presenter and submitted capture tap.
    const raf = requestAnimationFrame;
    globalThis.requestAnimationFrame = callback => globalThis.mirrorManual ? 0 : raf(callback);
    document.addEventListener('DOMContentLoaded', () => {
      const canvas = document.querySelector('canvas');
      canvas.width = 2560;
      canvas.height = 1440;
    });
  ` }, sessionId);
  const evaluate = async expression => {
    const response = await cdp.call('Runtime.evaluate', {
      expression, returnByValue: true, awaitPromise: true,
    }, sessionId);
    if (response.exceptionDetails) throw Error(response.result.description);
    return response.result.value;
  };
  await cdp.call('Page.navigate', { url }, sessionId);
  const drawn = await waitFor(async () => {
    const status = await evaluate(`[document.querySelector('#status')?.textContent,
      document.querySelector('#problems')?.textContent, globalThis.mirrorErrors]`);
    assert(!status[1] && !status[2]?.length && !/could not start|failed|unavailable/i.test(status[0]), JSON.stringify(status));
    return /generation \d+ drawn/i.test(status[0]) && status[0];
  }, 120000, 'mirror cooked placements');
  results.drawn = drawn;
  results.adapter = await evaluate('mirrorAdapter');
  assert.match(results.adapter.vendor, /apple/i);
  assert.match(results.adapter.architecture, /metal/i);
  assert.equal(results.adapter.fallback, false);
  results.frame = await evaluate('game.frame_info()');
  await fs.mkdir(path.join(output, 'frames'));
  const exporter = captureReceiver(output, 2560 * 1440 * 4, results, error => { captureError = error; });
  receiver = await serve(probeFiles, (request, response) => {
    response.setHeader('Access-Control-Allow-Origin', new URL(url).origin);
    response.setHeader('Access-Control-Allow-Headers', 'x-frame, content-type');
    if (request.method === 'OPTIONS') {
      response.writeHead(204).end();
      return true;
    }
    return exporter.receive(request, response);
  });
  const runArm = async (name, initial, motion) => {
    const plan = { name, initial, motion, upload: receiver.origin, ordinal: results.records.length };
    return evaluate(`(async ({name, initial, motion, upload, ordinal}) => {
      mirrorManual = true;
      const sleep = ms => new Promise(resolve => setTimeout(resolve,ms));
      await sleep(100);
      while (!game.frame_ready()) await sleep(4);
      const params = values => {
        const refusal = game.apply_params(JSON.stringify(values));
        if (refusal) throw Error(refusal);
      };
      const save = async (capture, time, opportunity) => {
        const response = await fetch(upload + '/capture?ordinal=' + ordinal++, {
          method: 'POST', body: capture.rgba, headers: { 'x-frame': JSON.stringify({
            name, time, opportunity, mode: motion ? 'realtime' : 'fixed',
            identity: capture.identity, frame: capture.frame,
          }) },
        });
        if (!response.ok) throw Error(await response.text());
      };
      game.begin_capture();
      const start = performance.now();
      let last = -1, count = 0, uploadError;
      const pending = new Set();
      try {
        do {
          const now = performance.now(), time = now - start;
          const opportunity = Math.floor(time / 50);
          if (opportunity === last || pending.size >= 3) {
            await sleep(2);
            continue;
          }
          last = opportunity;
          const t = Math.max(0, Math.min(1, (time - 350) / 2000));
          const phase = t*t*(3 - 2*t);
          const values = { ...initial };
          if (motion === 'pan') values['camera.pan'] = 25 - 50*phase;
          if (motion === 'tilt') values['camera.tilt'] += 22*phase;
          if (motion === 'orbit') values['camera.orbit'] = -25 + 50*phase;
          if (motion === 'sun') {
            values['sun.elevation'] += 10*phase - 5;
            values['sun.azimuth'] += 12*phase - 6;
          }
          params(values);
          const before = mirrorSubmits;
          game.frame(now);
          if (mirrorSubmits === before) continue;
          const deadline = performance.now() + 3500;
          let capture;
          do {
            game.frame_ready();
            capture = game.capture();
            if (!capture) await sleep(2);
          } while (!capture && performance.now() < deadline);
          if (!capture) throw Error('missing submitted image');
          if (!capture.identity.includes('presented=true')) throw Error(capture.identity);
          count++;
          const work = save(capture, time, opportunity);
          pending.add(work);
          work.then(() => pending.delete(work), error => { uploadError = error; pending.delete(work); });
        } while (motion ? performance.now() - start < 2850 : count === 0);
        await Promise.all(pending);
        if (uploadError) throw uploadError;
        if (game.capture_omissions()) throw Error('submitted tap omitted frames');
        return count;
      } finally {
        game.end_capture();
      }
    })(${JSON.stringify(plan)})`);
  };
  for (const [hour, elevation, azimuth] of [['noon', 51, 130], ['dusk', -5, 262]]) {
    const pose = (height, tilt) => ({
      'camera.height': height, 'camera.tilt': tilt, 'camera.pan': 0,
      'camera.orbit': 0, 'sun.elevation': elevation, 'sun.azimuth': azimuth,
    });
    if (mode === 'view') {
      await runArm(`low-${hour}`, pose(1.5, 9));
      continue;
    }
    for (const [name, height, tilt] of [['low', 1.5, 9], ['low-down', 1.5, 31], ['high', 10, 16], ['high-down', 10, 42]]) {
      assert.equal(await runArm(`${name}-${hour}`, pose(height, tilt)), 1);
    }
    for (const motion of ['pan', 'tilt', 'orbit', 'sun', 'mover']) {
      assert(await runArm(`${motion}-${hour}`, pose(1.5, 9), motion) >= 12, `incomplete ${hour} ${motion}`);
    }
  }
  if (mode === 'view') {
    await evaluate(`globalThis.viewProof = { pending: true };
      (async () => {
        const store = (await fetch('config.json').then(response => response.json())).store;
        const probe = await import(${JSON.stringify(receiver.origin + '/browser_probe.js')});
        await probe.default();
        viewProof = { report: await probe.prove_views(store) };
      })().catch(error => { viewProof = { error: String(error), gpu: mirrorErrors }; }); true`);
    results.proof = await waitFor(async () => {
      assert.deepEqual(results.consoleErrors, []);
      const state = await evaluate('viewProof');
      if (state.error) throw Error(JSON.stringify(state));
      return state.report;
    }, 180000, 'stored-format view and aerial comparisons');
    console.log(results.proof);
  }
  if (captureError) throw captureError;
  results.errors = await evaluate('mirrorErrors');
  assert.deepEqual(results.errors, []);
  assert.deepEqual(results.consoleErrors, []);
} catch (error) {
  results.failure = String(error);
  throw error;
} finally {
  await fs.writeFile(path.join(output, 'browser.json'), JSON.stringify(results, null, 2) + '\n');
  if (scene && scene.exitCode === null) process.kill(-scene.pid, 'SIGTERM');
  await log.close();
  await cleanup(chrome, () => ended, cdp, receiver?.server, profile);
}
console.log(path.join(output, 'browser.json'));
