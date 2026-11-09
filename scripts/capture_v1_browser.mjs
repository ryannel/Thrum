#!/usr/bin/env node
// Exact submitted canvas records. Packaging and elapsed-time inspection use the saved timeline.
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { promisify } from 'node:util';
import { gzip, gunzip } from 'node:zlib';
import { Cdp, assets, serve, serveCooked, hardware, waitFor, cleanup, cookedHead, movingRouteFiles, destroyActualDevice } from './check_v1_browser.mjs';
const compress = promisify(gzip);
const decompress = promisify(gunzip);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const args = { mode: 'realtime', control: 'ordinary', output: '', 'web-root': 'target/v1-game-web', cook: 'target/debug/thrum-cook', 'assets-source': 'examples/v1-game/assets/showcase', chrome: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' };
for (let i = 2; i < process.argv.length; i += 2) {
  const key = process.argv[i].slice(2);
  if (!Object.hasOwn(args, key) || !process.argv[i + 1]) throw new Error(`Unknown or incomplete option ${process.argv[i]}`);
  args[key] = process.argv[i + 1];
}
if (!['fixed', 'realtime'].includes(args.mode) || !['ordinary', 'stall', 'saturation', 'resize', 'loss', 'slow', 'off'].includes(args.control)) throw new Error('Invalid mode/control');
const output = path.resolve(args.output);
const tmp = await fs.realpath('tmp');
if (!output.startsWith(tmp + path.sep) || !args.output) throw new Error('Use a new output directory under tmp/');
const parent = await fs.realpath(path.dirname(output));
if (parent !== tmp && !parent.startsWith(tmp + path.sep)) throw new Error('Output parent escapes repository tmp/');
const space = await fs.statfs(parent);
if (space.bavail * space.bsize < 16 * 1024 ** 3) throw new Error('Less than the declared 16 GiB free-space allowance');
await fs.mkdir(output);
await fs.mkdir(path.join(output, 'frames'));
const profile = await fs.mkdtemp(path.join(output, 'chrome-profile-'));
const files = await assets(path.resolve(args['web-root']));
await movingRouteFiles(files, path.resolve(args['web-root']));
for (const [name, type] of [['capture.html', 'text/html'], ['capture.mjs', 'text/javascript']]) {
  const bytes = await fs.readFile(path.join(args['web-root'], name));
  const source = await fs.readFile(path.join('examples/v1-game/web', name));
  if (!bytes.equals(source)) throw new Error(`Stale capture driver ${name}`);
  files.set(`/${name}`, { bytes, type, sha256: sha(bytes) });
}
const sourceFiles = execFileSync('git', ['ls-files', '-co', '--exclude-standard'], { encoding: 'utf8' }).trim().split('\n');
const sourceHashes = {};
for (const file of sourceFiles) {
  if (/^(crates|examples|scripts)\//.test(file) || ['Cargo.lock', '.config/architecture.toml'].includes(file)) sourceHashes[file] = sha(await fs.readFile(file));
}
const route = JSON.parse(files.movingRoute);
const frameBytes = route.render_extent[0] * route.render_extent[1] * 4;
const manifest = { complete: false, target: 'chrome-webgpu', mode: args.mode, control: args.control, records: [], export_peak: 0, export_working_byte_cap: 32 * frameBytes, output_allowance: 16 * 1024 ** 3,
  source_revision: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), dirty_diff_sha256: sha(execFileSync('git', ['diff', 'HEAD'])),
  source_files: sourceHashes, artifacts: Object.fromEntries([...files].map(([name, value]) => [name, value.sha256])) };
manifest.console = execFileSync('/usr/sbin/ioreg', ['-n', 'Root', '-d1'], { encoding: 'utf8' }).split('\n').filter(line => /IOConsoleLocked|CGSSessionScreenIsLocked/.test(line)).join('\n');
const bundle = path.join(output, 'ThrumCapture.app');
const launcher = path.join(bundle, 'Contents/MacOS/Launcher');
await fs.mkdir(path.dirname(launcher), { recursive: true });
await fs.writeFile(path.join(bundle, 'Contents/Info.plist'), `<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>org.thrum.capture</string><key>CFBundleName</key><string>Thrum Capture</string><key>CFBundleExecutable</key><string>Launcher</string><key>CFBundlePackageType</key><string>APPL</string><key>LSUIElement</key><true/></dict></plist>`);
execFileSync('/usr/bin/swiftc', ['-module-cache-path', path.join(output, 'swift-cache'), 'scripts/capture_browser_launcher.swift', '-o', launcher]);
execFileSync('/usr/bin/codesign', ['--force', '--sign', '-', bundle]);
manifest.launcher = { sha256: sha(await fs.readFile(launcher)), scope: 'LaunchServices app identity, preflight must be false before starting Chrome; no permission request' };
let child, cdp, server, cook, ended = false, failure, active = 0, outputBytes = 0;
const pending = new Set();
function receive(request, response) {
  if (!request.url.startsWith('/capture?')) return false;
  const work = (async () => {
    if (request.method !== 'POST' || active >= 8) { request.resume(); response.writeHead(429).end(); return; }
    active++; manifest.export_peak = Math.max(manifest.export_peak, active);
    try {
      const ordinal = Number(new URL(request.url, 'http://localhost').searchParams.get('ordinal'));
      if (!Number.isSafeInteger(ordinal) || ordinal < 0 || ordinal >= 1024 || manifest.records[ordinal]) throw new Error('Invalid or duplicate record ordinal');
      const metadata = JSON.parse(request.headers['x-frame']);
      const chunks = []; let length = 0;
      for await (const chunk of request) {
        length += chunk.length;
        if (length > frameBytes) throw new Error('Oversized frame');
        chunks.push(chunk);
      }
      if (length !== frameBytes) throw new Error('Truncated frame');
      if (args.control === 'slow') await new Promise(resolve => setTimeout(resolve, 500));
      const raw = Buffer.concat(chunks);
      const encoded = await compress(raw, { level: 1 });
      outputBytes += encoded.length;
      if (outputBytes > manifest.output_allowance) throw new Error('Output allowance exhausted');
      const file = `frames/${String(ordinal).padStart(6, '0')}.rgba.gz`;
      await fs.writeFile(path.join(output, file), encoded, { flag: 'wx' });
      manifest.records[ordinal] = { ...metadata, ordinal, file, rgba_sha256: sha(raw), bytes: encoded.length };
      response.writeHead(200).end();
    } finally { active--; }
  })().catch(error => { failure ??= error; response.writeHead(500).end(String(error)); });
  pending.add(work); work.finally(() => pending.delete(work));
  return true;
}
try {
  ({ server, origin: manifest.origin } = await serve(files, receive));
  const serving = await serveCooked(args, output); cook = serving.cook;
  const head = await cookedHead(output);
  if (head.root !== route.cooked_root || head.generation !== route.cooked_generation) throw new Error(`Cooked root mismatch ${JSON.stringify(head)}`);
  manifest.cooked = head;
  manifest.flags = [`--user-data-dir=${profile}`, '--remote-debugging-port=0', '--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-component-update', '--window-size=1280,1100', 'about:blank'];
  child = spawn('/usr/bin/open', ['-n', '-W', bundle, '--args', output, args.chrome, ...manifest.flags], { stdio: 'ignore' });
  let startupError;
  child.once('error', error => { startupError = error; ended = true; });
  child.once('exit', () => { ended = true; });
  const endpoint = await waitFor(async () => {
    if (startupError || ended) throw startupError ?? new Error('Chrome exited');
    try { return (await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n'); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
  }, 15000, 'Chrome DevTools');
  const socket = new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  cdp = new Cdp(socket);
  manifest.permission_preflight = JSON.parse(await fs.readFile(path.join(output, 'launcher.json'), 'utf8'));
  if (manifest.permission_preflight.screen_recording_before !== false) throw new Error('Launcher permission preflight was not denied');
  manifest.browser = await cdp.call('Browser.getVersion');
  manifest.gpu = await cdp.call('SystemInfo.getInfo'); hardware(manifest.gpu);
  const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
  await cdp.call('Page.enable', {}, sessionId);
  await cdp.call('Page.navigate', { url: `${manifest.origin}/capture.html?mode=${args.mode}&control=${args.control}&assets=${encodeURIComponent(serving.origin)}` }, sessionId);
  let destroyed = false;
  const record = await waitFor(async () => {
    if (args.control === 'loss' && !destroyed) {
      const state = await cdp.call('Runtime.evaluate', { expression: 'window.thrumCapture?.waiting', returnByValue: true }, sessionId);
      if (state.result.value === 'destroy-pending') { await destroyActualDevice(cdp, sessionId, undefined, () => {}); destroyed = true; }
    }
    const r = await cdp.call('Runtime.evaluate', { expression: 'window.thrumCapture?.done ? JSON.stringify(window.thrumCapture) : null', returnByValue: true }, sessionId);
    return r.result.value ? JSON.parse(r.result.value) : undefined;
  }, 180000, 'submitted-output route');
  await fs.writeFile(path.join(output, 'runtime.json'), JSON.stringify(record, null, 2));
  await Promise.all(pending);
  if (!record.complete) {
    if (args.control === 'loss' && record.control_pass) {
      manifest.negative_control_pass = true;
    } else if (['saturation', 'slow'].includes(args.control) && /incomplete capture/.test(record.error)
        && record.final_report.tick > record.opportunities[0].tick + 100
        && (record.renderer_omissions > 0 || record.export_omissions > 0)) {
      manifest.negative_control_pass = true;
    } else throw new Error(record.error ?? 'Incomplete page result');
  }
  if (['saturation', 'slow', 'loss'].includes(args.control) && !manifest.negative_control_pass) throw new Error('Negative control unexpectedly completed');
  if (manifest.negative_control_pass) {
    manifest.refusal = record.error;
  } else {
    if (args.control === 'off') { manifest.observer_only = true; } else {
      if (record.images.length !== manifest.records.length || Array.from(manifest.records).some(row => !row)) throw new Error('Missing exported records');
      if (record.final_report.presented - record.initial_report.presented !== manifest.records.length) throw new Error('Submission tail or head missing');
      const ids = manifest.records.map(row => row.submission_id);
      if (!ids.length || ids.some((id, i) => i > 0 && id !== ids[i - 1] + 1)) throw new Error('Noncontiguous submission IDs');
      const syncStart = performance.now();
      for (const row of manifest.records) {
        const file = await fs.open(path.join(output, row.file), 'r+');
        try { await file.sync(); } finally { await file.close(); }
      }
      manifest.final_sync_ms = performance.now() - syncStart;
      for (const [i, row] of manifest.records.entries()) {
        if (row.submission_id !== record.images[i].submission_id || !row.surface_presented) throw new Error('Image join/surface mismatch');
        if (i > 0 && row.submit_ms <= manifest.records[i - 1].submit_ms) throw new Error('Nonmonotonic submit time');
        const raw = await decompress(await fs.readFile(path.join(output, row.file)));
        if (raw.length !== frameBytes || sha(raw) !== row.rgba_sha256) throw new Error('Lossless round-trip failed');
      }
      if (record.final_report.timestamp_clamps || (record.final_report.overload_events && args.control !== 'stall')) throw new Error('Runtime clamped/overloaded');
      if (args.control === 'resize' && new Set(manifest.records.map(row => row.presenter_generation)).size < 2) throw new Error('Resize did not change presenter generation');
      manifest.complete = true;
    }
  }
} catch (error) { failure ??= error; }
finally {
  await Promise.all(pending);
  cook?.kill();
  try { await cleanup(child, () => ended, cdp, server, profile); } catch (error) { failure ??= error; }
  try {
    manifest.permission_preflight = JSON.parse(await fs.readFile(path.join(output, 'launcher.json'), 'utf8'));
    if (manifest.permission_preflight.screen_recording_after !== false || manifest.permission_preflight.chrome_exit_code !== 0) failure ??= new Error('Launcher final preflight/Chrome exit failed');
  } catch (error) { failure ??= error; }
  manifest.error = failure ? String(failure) : undefined;
  manifest.complete &&= !failure;
  if (manifest.permission_preflight?.screen_recording_before !== false) manifest.complete = false;
  manifest.console_after = execFileSync('/usr/sbin/ioreg', ['-n', 'Root', '-d1'], { encoding: 'utf8' }).split('\n').filter(line => /IOConsoleLocked|CGSSessionScreenIsLocked/.test(line)).join('\n');
  manifest.encoding = { stage: 'final SDR engine output after history/tone/engine overlay, before canvas presentation', channel_order: 'RGBA8', colour: 'sRGB', row_stride: route.render_extent[0] * 4, extent: route.render_extent };
  manifest.encoded_bytes = outputBytes;
  await fs.writeFile(path.join(output, 'manifest.json'), JSON.stringify(manifest, null, 2));
}
if (failure) throw failure;
console.log(`PASS ${manifest.observer_only ? "capture-off observer arm;" : manifest.negative_control_pass ? "negative control, incomplete capture;" : "complete capture;"} ${manifest.records.length} exact submitted images, ${outputBytes} encoded bytes`);
