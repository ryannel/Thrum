#!/usr/bin/env node
// Exact submitted canvas records. Packaging and elapsed-time inspection use the saved timeline.
import fs from 'node:fs/promises';
import { loadavg } from 'node:os';
import path from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { promisify } from 'node:util';
import { gunzip, gzip } from 'node:zlib';
import { checkPublicSheetPage, checkSheetFrame, checkSheetRequests, checkSheetSwitch } from './check_sprite_sheets_browser.mjs';
import { captureReceiver } from './browser_capture_receiver.mjs';
import { installWasmReceipt } from './wasm_receipt.mjs';
import { finalizeBrowserCapture, openLauncherControl } from './browser_capture_shutdown.mjs';
import { Cdp, assets, serve, serveCooked, hardware, waitFor, closeBrowserConnections, cookedHead, movingRouteFiles, destroyActualDevice, existingGame, releaseGameGraphics, unloadGamePage } from './check_v1_browser.mjs';
const decompress = promisify(gunzip);
const compress = promisify(gzip);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const args = { mode: 'realtime', source: 'sdr', scene: 'day', reconstruction: 'off', witness: 'off', 'diagnostic-clouds': 'off', overlay: 'route', 'route-file': '', control: 'ordinary', output: '', 'web-root': '', cook: 'target/debug/thrum-cook', 'assets-source': 'examples/v1-game/assets/showcase', chrome: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' };
args.effect = 'off';
args.clouds = 'off';
args['cloud-view'] = '';
args['retain-raw'] = 'off';
args.atmosphere = 'off';
args.haze = '';
args['haze-take'] = '';
args['renderer-signature'] = 'off';
args.params = '';
args['sprite-scale'] = '1.5';
args['sprite-speed'] = '0.05';
args.dpr = '1';
args['wasm-receipt'] = 'off';
for (let i = 2; i < process.argv.length; i += 2) {
  const key = process.argv[i].slice(2);
  if (!Object.hasOwn(args, key) || !process.argv[i + 1]) throw new Error(`Unknown or incomplete option ${process.argv[i]}`);
  args[key] = process.argv[i + 1];
}
if (!['fixed', 'realtime'].includes(args.mode) || !['ordinary', 'stall', 'saturation', 'resize', 'loss', 'slow', 'off'].includes(args.control)) throw new Error('Invalid mode/control');
if (!['sdr', 'scene-linear'].includes(args.source) || (args.source === 'scene-linear' && args.control !== 'ordinary')) throw new Error('Unsupported capture source/control');
if (!['day', 'night', 'noon', 'exit', 'sprites-field', 'sprites-scroll', 'sprites-sheets'].includes(args.scene)) throw new Error('Unknown route scene');
if (!['off', 'on'].includes(args['wasm-receipt'])) throw new Error('Invalid Wasm receipt option');
const spriteScroll = args.scene === 'sprites-scroll';
const spriteSheets = args.scene === 'sprites-sheets';
const spriteField = args.scene === 'sprites-field' || spriteScroll || spriteSheets;
const spriteDpr = Number(args.dpr);
if (spriteScroll && (!['1.5', '2', '2.37'].includes(args['sprite-scale']) ||
    !['0.05', '0.3', '3'].includes(args['sprite-speed']) || ![1, 2].includes(spriteDpr))) {
  throw new Error('Invalid sprite scroll scale, speed or DPR');
}
if (spriteField && (args.mode !== 'realtime' || args.control !== 'ordinary' || args.source !== 'sdr' ||
    args.clouds !== 'off' || args.reconstruction !== 'off' || args.effect !== 'off' || args.witness !== 'off')) {
  throw new Error('Sprite field requires ordinary realtime SDR without 3D selections');
}
if (!['route','off'].includes(args.overlay)) throw new Error('Invalid overlay option');
if (!['off','on'].includes(args['diagnostic-clouds'])) throw new Error('Invalid diagnostic-clouds option');
if (!['off','shade','sky'].includes(args.clouds) || (args.clouds !== 'off' && args['diagnostic-clouds'] === 'on')) throw new Error('Invalid cloud selection');
if (args['cloud-view'] && (args.clouds === 'off' || args.control !== 'ordinary' || args.source !== 'sdr')) throw new Error('A cloud view requires an ordinary SDR cloud capture');
if (!['off','taau'].includes(args.reconstruction)) throw new Error('Invalid reconstruction option');
if (!['off','pan','reveal','spawn','stars','scrub'].includes(args.witness) || (args.witness !== 'off' && args.mode !== 'realtime')) throw new Error('Invalid reconstruction witness/mode');
if (!['off','slab','depth-only'].includes(args.effect)) throw new Error('Invalid effect diagnostic');
if (!['off','checkpoint','scrub','ground-pan','cost'].includes(args.atmosphere) ||
    (args.atmosphere !== 'off' && (!args.params || args.witness !== 'off' ||
       args.control !== (args.atmosphere === 'cost' ? 'off' : 'ordinary') ||
       args.mode !== (['checkpoint','cost'].includes(args.atmosphere) ? 'fixed' : 'realtime')))) throw new Error('Invalid atmosphere capture options');
if (!['off', 'on'].includes(args['retain-raw']) || (args['retain-raw'] === 'on' &&
    ((args.scene !== 'exit' && args.atmosphere === 'off') ||
      args.mode !== 'realtime' || args.control !== 'ordinary'))) throw new Error('Raw retention requires a realtime exit or atmosphere capture');
const output = path.resolve(args.output);
const tmp = await fs.realpath('tmp');
if (!output.startsWith(tmp + path.sep) || !args.output) throw new Error('Use a new output directory under tmp/');
const parent = await fs.realpath(path.dirname(output));
if (parent !== tmp && !parent.startsWith(tmp + path.sep)) throw new Error('Output parent escapes repository tmp/');
const space = await fs.statfs(parent);
if (space.bavail * space.bsize < 16 * 1024 ** 3) throw new Error('Less than the declared 16 GiB free-space allowance');
if (!args['web-root']) {
  args['web-root'] = 'target/v1-game-web-proving';
  execFileSync('sh', ['examples/v1-game/web/build.sh', '--proving'], {
    env: { ...process.env, CARGO_TARGET_DIR: 'target', THRUM_V1_WEB_OUTPUT: args['web-root'], RUSTC_WRAPPER: '' },
    stdio: 'inherit',
  });
}
await fs.mkdir(output);
await fs.mkdir(path.join(output, 'frames'));
const files = await assets(path.resolve(args['web-root']));
await movingRouteFiles(files, path.resolve(args['web-root']));
let captureRoute;
if (args.params) {
  const bytes = await fs.readFile(path.resolve(args.params));
  files.set('/capture-params.json', { bytes, type: 'application/json', sha256: sha(bytes) });
}
if (args['haze-take']) {
  const bytes = await fs.readFile(path.resolve(args['haze-take']));
  files.set('/haze-take.json', { bytes, type: 'application/json', sha256: sha(bytes) });
}
if (args['route-file']) {
  captureRoute = await fs.readFile(path.resolve(args['route-file']));
  files.set('/capture-route.json', { bytes: captureRoute, type: 'application/json', sha256: sha(captureRoute) });
}
const cloudView = args['cloud-view'] ? await fs.readFile(path.resolve(args['cloud-view'])) : undefined;
const cloudMotion = cloudView && args.mode === 'realtime';
if (cloudView) files.set('/capture-view.json', { bytes: cloudView, type: 'application/json', sha256: sha(cloudView) });
for (const [name, type] of [['capture.html', 'text/html'], ['capture.mjs', 'text/javascript'], ['capture_upload.mjs', 'text/javascript']]) {
  const bytes = await fs.readFile(path.join(args['web-root'], name));
  const source = await fs.readFile(path.join('examples/v1-game/web', name));
  if (!bytes.equals(source)) throw new Error(`Stale capture driver ${name}`);
  files.set(`/${name}`, { bytes, type, sha256: sha(bytes) });
}
// Ask Git only for the paths hashed below; retained evidence can exceed Node's output buffer.
const sourceFiles = execFileSync('git', ['ls-files', '-co', '--exclude-standard', '--', 'crates', 'examples', 'scripts', 'Cargo.lock', '.config/architecture.toml'], { encoding: 'utf8' }).trim().split('\n');
const sourceHashes = {};
for (const file of sourceFiles) {
  if (/^(crates|examples|scripts)\//.test(file) || ['Cargo.lock', '.config/architecture.toml'].includes(file)) sourceHashes[file] = sha(await fs.readFile(file));
}
const route = JSON.parse(captureRoute ?? files.movingRoute);
const extent = spriteField ? [1920, 1080] : cloudView ? JSON.parse(cloudView).extent : route.render_extent;
const frameBytes = extent[0] * extent[1] * (args.source === 'scene-linear' ? 8 : 4);
const spoolRaw = args.mode === 'realtime' && args.control === 'ordinary' &&
  ( args.scene === 'exit' || args.witness !== 'off' || args.clouds !== 'off' || args.atmosphere !== 'off');
const manifest = { complete: false, target: 'chrome-webgpu', source: args.source, scene: args.scene, reconstruction: args.reconstruction, witness: args.witness, diagnostic_clouds: args['diagnostic-clouds'], overlay: args.overlay, mode: args.mode, control: args.control, records: [], export_peak: 0, export_working_byte_cap: 32 * frameBytes, output_allowance: 16 * 1024 ** 3,
  source_revision: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), dirty_diff_sha256: sha(execFileSync('git', ['diff', 'HEAD'])),
  route_file: args['route-file'] || null, route_sha256: sha(captureRoute ?? files.movingRoute),
  source_files: sourceHashes, artifacts: Object.fromEntries([...files].map(([name, value]) => [name, value.sha256])) };
if (spriteField || args.scene === 'exit' || spoolRaw) {
  manifest.record_limit = cloudMotion ? 4096 : spriteField || args.scene === 'exit' || args.clouds !== 'off' ? 1800 : args.atmosphere !== 'off' ? 1024 : 270;
  // A suspended request still owns its image. Give the 720p motion take a
  // bounded backlog without increasing the larger or fixed capture profiles.
  manifest.export_slots = spriteField ? 32 : spoolRaw && frameBytes <= 1280 * 720 * 4
    ? (args.atmosphere !== 'off' || cloudMotion ? 128 : 64) : 24;
  if (spriteField) manifest.admission_policy = 'wait';
  manifest.export_working_byte_cap = 4 * manifest.export_slots * frameBytes;
}
if (args.clouds !== 'off') manifest.clouds = args.clouds;
if (cloudView) manifest.cloud_view = JSON.parse(cloudView);
manifest.effect = args.effect;
manifest.atmosphere = args.atmosphere;
if (args.haze) manifest.haze = args.haze;
manifest.console = execFileSync('/usr/sbin/ioreg', ['-n', 'Root', '-d1'], { encoding: 'utf8' }).split('\n').filter(line => /IOConsoleLocked|CGSSessionScreenIsLocked/.test(line)).join('\n');
const bundle = path.join(output, 'ThrumCapture.app');
const launcher = path.join(bundle, 'Contents/MacOS/Launcher');
await fs.mkdir(path.dirname(launcher), { recursive: true });
await fs.writeFile(path.join(bundle, 'Contents/Info.plist'), `<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>org.thrum.capture</string><key>CFBundleName</key><string>Thrum Capture</string><key>CFBundleExecutable</key><string>Launcher</string><key>CFBundlePackageType</key><string>APPL</string><key>LSUIElement</key><true/></dict></plist>`);
execFileSync('/usr/bin/swiftc', ['-module-cache-path', path.join(output, 'swift-cache'), 'scripts/capture_browser_launcher.swift', '-o', launcher]);
execFileSync('/usr/bin/codesign', ['--force', '--sign', '-', bundle]);
manifest.launcher = { sha256: sha(await fs.readFile(launcher)), scope: 'LaunchServices app identity, preflight must be false before starting Chrome; no permission request' };
let control, launchError, socket, cdp, server, cook, failure;
const retainRaw = args['retain-raw'] === 'on';
const spoolPath = path.join(output, 'frames.rgba');
const rawSpool = spoolRaw ? await fs.open(spoolPath, 'wx+') : undefined;
const { receive, pending } = captureReceiver(output, frameBytes, manifest, error => { failure ??= error; }, { slow: args.control === 'slow', rawSpool });
try {
  let cookedOrigin, spriteSource, sheetEdit;
  const requests = [];
  const serveCapture = (request, response) => {
    if (!spriteSheets || request.method !== 'POST' || request.url !== '/sprite-sheet-edit') {
      return receive(request, response);
    }
    if (sheetEdit || !spriteSource) { response.writeHead(409).end(); return true; }
    sheetEdit = fs.rename(path.join(output, 'edits/recolour.png'), path.join(spriteSource, 'field/walk.png'))
      .then(() => response.writeHead(200, { 'Content-Type': 'application/json' })
        .end(JSON.stringify({ rename_monotonic_ns: String(process.hrtime.bigint()) })))
      .catch(error => { failure ??= error; response.writeHead(500).end(); });
    return true;
  };
  ({ server, origin: manifest.origin } = await serve(files, serveCapture));
  if (spriteSheets) {
    const source = path.join(output, 'fixture-source');
    execFileSync('python3', ['scripts/sprite_sheet_fixture.py', source, path.join(output, 'edits')]);
    const serving = await serveCooked({ ...args, 'assets-source': source }, output);
    cook = serving.cook;
    cookedOrigin = serving.origin;
    spriteSource = serving.source;
    manifest.cooked = await cookedHead(output);
  }
  if (!spriteField) {
    const serving = await serveCooked(args, output); cook = serving.cook;
    cookedOrigin = serving.origin;
    const head = await cookedHead(output);
    if (head.root !== route.cooked_root || head.generation !== route.cooked_generation) throw new Error(`Cooked root mismatch ${JSON.stringify(head)}`);
    manifest.cooked = head;
  }
  const profile = path.join(output, `chrome-profile-${randomUUID()}`);
  manifest.flags = [`--user-data-dir=${profile}`, '--remote-debugging-port=0', '--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-crashpad-for-testing', '--disable-component-update', '--window-size=1280,1100', 'about:blank'];
  if (spriteField) manifest.flags.splice(-1, 0, '--disable-renderer-backgrounding', '--disable-background-timer-throttling');
  control = await openLauncherControl();
  const wrapper = spawn('/usr/bin/open', ['-n', '-W', bundle, '--args', output, '--owned',
    String(control.port), control.token, profile, args.chrome, ...manifest.flags], { stdio: 'ignore' });
  wrapper.once('error', error => { launchError = error; });
  wrapper.unref();
  const browserReady = async () => {
    if (launchError) throw launchError;
    const receipt = control.receipt;
    if (receipt?.error || receipt?.chrome_exited || receipt?.chrome_final) {
      throw new Error(receipt.error ?? 'Chrome exited before DevTools was ready');
    }
    if (control.startOffered && !control.connected) throw new Error('Chrome launcher control disconnected');
    return receipt?.chrome_pid ? receipt : undefined;
  };
  const endpoint = await waitFor(async () => {
    if (!await browserReady()) return;
    try { return (await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n'); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
  }, 15000, 'Chrome DevTools');
  socket = new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);
  let socketError;
  socket.addEventListener('error', () => { socketError = new Error('Chrome DevTools connection failed'); }, { once: true });
  manifest.permission_preflight = await waitFor(async () => {
    if (socketError) throw socketError;
    const receipt = await browserReady();
    if (socket.readyState === WebSocket.OPEN) return receipt;
  }, 15000, 'Chrome DevTools connection');
  cdp = new Cdp(socket);
  if (manifest.permission_preflight.screen_recording_before !== false) throw new Error('Launcher permission preflight was not denied');
  manifest.browser = await cdp.call('Browser.getVersion');
  manifest.gpu = await cdp.call('SystemInfo.getInfo'); hardware(manifest.gpu);
  const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
  await cdp.call('Page.enable', {}, sessionId);
  if (args['wasm-receipt'] === 'on') await installWasmReceipt(cdp, sessionId);
  if (spriteField) await cdp.call('Browser.setDownloadBehavior', { behavior: 'allow', downloadPath: output });
  const completionSignal = (spriteField || args.scene === 'exit' || args.clouds !== 'off' || args.atmosphere !== 'off') && args.mode === 'realtime' &&
    (args.control === 'ordinary' || args.control === 'off');
  if (spriteSheets) await cdp.call('Network.enable', {}, sessionId);
  let captureDone = false;
  if (completionSignal) {
    await cdp.call('Target.activateTarget', { targetId });
    await cdp.call('Runtime.enable', {}, sessionId);
    await cdp.call('Runtime.addBinding', { name: 'thrumCaptureDone' }, sessionId);
    cdp.event = event => {
      if (event.sessionId === sessionId && event.method === 'Network.requestWillBeSent' &&
          event.params.request.method === 'GET' && event.params.request.url.startsWith(`${cookedOrigin}/cas/`)) {
        requests.push({ hash: event.params.request.url.split('/').at(-1), timestamp_s: event.params.timestamp });
      }
      if (event.sessionId === sessionId && event.method === 'Runtime.bindingCalled'  &&
          event.params.name === 'thrumCaptureDone') captureDone = true;
    };
  }
  let spriteQuery = '';
  if (spriteScroll) {
    await cdp.call('Emulation.setDeviceMetricsOverride', {
      width: extent[0] / spriteDpr, height: extent[1] / spriteDpr,
      deviceScaleFactor: spriteDpr, mobile: false,
    }, sessionId);
    spriteQuery = `&sprite-probe=scroll:${args['sprite-scale']}:${args['sprite-speed']}`;
    if (process.env.THRUM_SPRITE_PASS_TIMING === '1') spriteQuery += '&sprite-pass-timing';

    manifest.sprite_scroll = { scale: Number(args['sprite-scale']), speed: Number(args['sprite-speed']), dpr: spriteDpr };
  }
  manifest.load_before = loadavg();
  await cdp.call('Page.navigate', { url: `${manifest.origin}/capture.html?mode=${args.mode}&control=${args.control}&source=${args.source}&scene=${args.scene}&reconstruction=${args.reconstruction}&witness=${args.witness}&atmosphere=${args.atmosphere}&diagnostic-clouds=${args['diagnostic-clouds']}&overlay=${args.overlay}&effect=${args.effect}&route=${captureRoute ? 'capture-route.json' : 'moving-v1.json'}&assets=${encodeURIComponent(cookedOrigin ?? '')}${manifest.export_slots ? `&export-slots=${manifest.export_slots}` : ''}&clouds=${args.clouds}${args.haze ? `&haze=${encodeURIComponent(args.haze)}` : ''}${args['haze-take'] ? '&haze-take' : ''}${args['renderer-signature'] === 'on' ? '&renderer-signature' : ''}${cloudView ? '&cloud-view' : ''}${spriteQuery}` }, sessionId);
  let destroyed = false;
  const record = await waitFor(async () => {
    if (completionSignal && !captureDone) return undefined;
    if (args.control === 'loss' && !destroyed) {
      const state = await cdp.call('Runtime.evaluate', { expression: 'window.thrumCapture?.waiting', returnByValue: true }, sessionId);
      if (state.result.value === 'destroy-pending') { await destroyActualDevice(cdp, sessionId, undefined, () => {}); destroyed = true; }
    }
    const r = await cdp.call('Runtime.evaluate', { expression: 'window.thrumCapture?.done ? JSON.stringify(window.thrumCapture) : null', returnByValue: true }, sessionId);
    return r.result.value ? JSON.parse(r.result.value) : undefined;
  }, 180000, 'submitted-output route');
  manifest.load_after = loadavg();
  if (args['wasm-receipt'] === 'on') {
    const receipt = await cdp.call('Runtime.evaluate', {
      // The capture page instantiates one module. An absent or ambiguous receipt cannot prove its identity.
      expression: 'globalThis.__thrumWasmReceipts?.length === 1 ? globalThis.__thrumWasmReceipts[0].sha256 : null',
      returnByValue: true,
    }, sessionId);
    if (receipt.result.value) manifest.instantiated_wasm_sha256 = receipt.result.value;
  }
  await fs.writeFile(path.join(output, 'runtime.json'), JSON.stringify(record, null, 2));
  if (spriteScroll && (record.device_pixel_ratio !== spriteDpr ||
      JSON.stringify(record.css_extent.map(side => side * spriteDpr)) !== JSON.stringify(extent))) {
    throw new Error('Scroll did not run at one canvas backing pixel per device pixel');
  }
  if ((args.witness !== 'off' || args.atmosphere !== 'off') && (record.initial_report?.reconstruction !== (args.reconstruction === 'taau' ? 'Taau' : 'Off') || record.final_report?.reconstruction !== record.initial_report?.reconstruction)) throw new Error('Prepared reconstruction provider did not match the capture arm');
  await Promise.all(pending);
  if (spriteField && record.complete) {
    const file = path.join(output, 'sprites-field.rgba');
    await waitFor(async () => (await fs.stat(file).catch(error => {
      if (error.code !== 'ENOENT') throw error;
    }))?.size === record.spool_bytes, 600000, 'sealed sprite download');
    await fs.rename(file, spoolPath);
    manifest.records = record.images.map((row, ordinal) => ({
      ...row, ordinal, file: path.basename(spoolPath), offset: ordinal * frameBytes, bytes: frameBytes,
    }));
  }
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
      if (args.scene === 'exit' && args.mode === 'fixed') {
        const first = route.origin_tick;
        const last = first + route.keys.at(-1).tick;
        const active = manifest.records.filter(row => row.source_tick >= first && row.source_tick <= last);
        const phase = route.phase[0] / route.phase[1];
        if (active[0]?.source_tick !== first || active.at(-1)?.source_tick !== last) throw new Error('Fixed exit capture missed a route endpoint');
        const wrongPhase = active.find(row => !Number.isFinite(row.alpha) || Math.abs(row.alpha - phase) > 1e-6);
        if (wrongPhase) throw new Error(`Fixed exit tick ${wrongPhase.source_tick} submitted phase ${wrongPhase.alpha}, expected ${phase}`);
      }
      if (record.final_report.timestamp_clamps || (record.final_report.overload_events && args.control !== 'stall')) throw new Error('Runtime clamped/overloaded');
      if (args.control === 'resize' && new Set(manifest.records.map(row => row.presenter_generation)).size < 2) throw new Error('Resize did not change presenter generation');
      const spriteSpool = spriteField ? await fs.open(spoolPath, 'r') : undefined;
      const rawSource = rawSpool ?? spriteSpool;
      const spoolFrame = rawSource && Buffer.allocUnsafe(frameBytes);
      try {
        for (const [i, row] of manifest.records.entries()) {
          if (row.submission_id !== record.images[i].submission_id || !row.surface_presented) throw new Error('Image join/surface mismatch');
          if (i > 0 && row.submit_ms <= manifest.records[i - 1].submit_ms) throw new Error('Nonmonotonic submit time');
          let raw;
          if (rawSource) {
            let offset = 0;
            while (offset < frameBytes) {
              const { bytesRead } = await rawSource.read(spoolFrame, offset, frameBytes - offset, i * frameBytes + offset);
              if (bytesRead === 0) throw new Error('Short raw spool frame');
              offset += bytesRead;
            }
            raw = spoolFrame;
          } else {
            raw = await decompress(await fs.readFile(path.join(output, row.file)));
          }
          if (spriteSheets) checkSheetFrame(raw, row);
          const digest = sha(raw);
          if (raw.length !== frameBytes || digest !== row.rgba_sha256) {
            throw new Error(`Lossless round-trip failed for submission ${row.submission_id}`);
          }
          if (retainRaw) {
            row.file = path.basename(spoolPath);
            row.offset = i * frameBytes;
            row.bytes = frameBytes;
          } else if (spoolRaw) {
            const encoded = await compress(raw, { level: 1 });
            const packed = path.join(output, row.file);
            await fs.writeFile(packed, encoded, { flag: 'wx' });
            if (sha(await decompress(await fs.readFile(packed))) !== row.rgba_sha256) throw new Error('Lossless archive verification failed');
            row.bytes = encoded.length;
          }
        }
      } finally {
        await spriteSpool?.close();
      }
      if (spriteSheets) {
        await sheetEdit;
        const recoloured = await cookedHead(output);
        manifest.sprite_sheets = await checkSheetSwitch(output, record, manifest.records,
          manifest.cooked, recoloured, requests.splice(0));
        await unloadGamePage(cdp, sessionId, 30000);
        const publicUrl = `${manifest.origin}/?scene=sprites&rows=500&assets=${encodeURIComponent(cookedOrigin)}`;
        const call = (method, params) => cdp.call(method, params, sessionId);
        let publicGame;
        const gameOnPage = async () => {
          publicGame = await existingGame(cdp, sessionId);
          return publicGame;
        };
        const restoreArt = async () => {
          const replacement = path.join(output, 'edits/restore.png');
          await fs.copyFile(path.join(output, 'fixture-source/field/walk.png'), replacement);
          await fs.rename(replacement, path.join(spriteSource, 'field/walk.png'));
          return String(process.hrtime.bigint());
        };
        try {
          const proof = await checkPublicSheetPage(call, publicUrl, gameOnPage, waitFor, restoreArt);
          proof.fetches = await checkSheetRequests(output, recoloured, await cookedHead(output), requests.splice(0));
          await fs.writeFile(path.join(output, 'public-sheet-page.json'), JSON.stringify(proof, null, 2) + '\n');
        } finally {
          if (publicGame) await releaseGameGraphics(cdp, sessionId, publicGame);
          await unloadGamePage(cdp, sessionId, 30000);
        }
      }
      const syncStart = performance.now();
      if (spriteField) {
        const file = await fs.open(spoolPath, 'r+');
        try { await file.sync(); } finally { await file.close(); }
      } else if (retainRaw) await rawSpool.sync();
      else for (const row of manifest.records) {
        const file = await fs.open(path.join(output, row.file), 'r+');
        try { await file.sync(); } finally { await file.close(); }
      }
      manifest.final_sync_ms = performance.now() - syncStart;
      if (spoolRaw && !retainRaw) await fs.unlink(spoolPath);
      manifest.complete = true;
    }
  }
} catch (error) { failure ??= error; }
finally {
  cook?.kill();
  // Realtime atmosphere takes can keep large readback pools alive until the
  // owned Chrome process exits. Allow it to drain those pools under shared load.
  const drainLargeCapture = spriteField || Boolean(args.haze) || (spoolRaw && args.atmosphere !== 'off');
  manifest.encoding = { stage: 'final SDR engine output after history/tone/engine overlay, before canvas presentation', channel_order: 'RGBA8', colour: 'sRGB', row_stride: extent[0] * 4, extent };
  manifest.spool_encoding = spriteField ? 'browser File download, raw RGBA8' : retainRaw ? 'raw' : spoolRaw ? 'raw with deferred gzip' : 'live gzip';
  await finalizeBrowserCapture({
    output, control, cdp, manifest,
    graceMs: drainLargeCapture ? 45000 : undefined,
    closeConnections: async () => {
      try { await closeBrowserConnections(socket, server); }
      finally {
        await Promise.all(pending);
        await rawSpool?.close();
      }
      manifest.encoded_bytes = manifest.records.reduce((total, row) => total + row.bytes, 0);
      manifest.console_after = execFileSync('/usr/sbin/ioreg', ['-n', 'Root', '-d1'], { encoding: 'utf8' }).split('\n').filter(line => /IOConsoleLocked|CGSSessionScreenIsLocked/.test(line)).join('\n');
    },
    captureError: () => failure,
  });
}
console.log(`PASS ${manifest.observer_only ? "capture-off observer arm;" : manifest.negative_control_pass ? "negative control, incomplete capture;" : "complete capture;"} ${manifest.records.length} exact submitted images, ${manifest.encoded_bytes} encoded bytes`);
