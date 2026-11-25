// Run the ordinary sprite page through its submitted-image tap and WebGPU recovery.
import fs from 'node:fs/promises';
import path from 'node:path';
import { installWasmReceipt, readWasmReceipt } from './wasm_receipt.mjs';

const graphPasses = ['sprites', 'display-referred tone encode', 'V1 surface present'];

// Uses authored sheet bytes and inverse pixel coordinates, independently of WGSL.
function compareCard(image, sheet, width, height, scale) {
  const left = Math.floor(width / 2) - 128 * scale;
  const top = Math.floor(height / 2) - 64 * scale;
  if (image.length !== width * height * 4 || sheet.length !== 256 * 128 * 4) {
    throw new Error('Unexpected source or submitted output extent');
  }
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const u = Math.floor((x - left) / scale);
      const v = Math.floor((y - top) / scale);
      const i = (y * width + x) * 4;
      const inside = u >= 0 && u < 256 && v >= 0 && v < 128;
      for (let component = 0; component < 4; component++) {
        const expected = inside ? sheet[(v * 256 + u) * 4 + component] : component === 3 ? 255 : 0;
        if (image[i + component] !== expected) {
          return { x, y, component, expected, actual: image[i + component] };
        }
      }
    }
  }
  return null;
}

async function checkStartupRefusals(cdp, origin, timeout, waitFor) {
  const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
  try {
    const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
    const rows = [];
    for (const [query, expected] of [
      ['assets=/cooked', 'cooked placements (?assets)'],
      ['field', 'the flythrough field (?field)'],
      ['meshes=on', 'valley meshes (?meshes)'],
      ['field-meshes', 'field meshes (?field-meshes)'],
      ['diagnostic-clouds', 'diagnostic clouds (?diagnostic-clouds)'],
      ['material=/shader.wgsl', 'a cooked material hook (?material)'],
      ['stars=on', 'valley stars (?stars)'],
      ['culling=direct', 'camera culling mode direct'],
      ['culling=frustum', 'camera culling mode frustum'],
      ['culling=occlusion', 'camera culling mode occlusion'],
      ['reconstruction=taau', 'reconstructor Taau'],
      ['reconstruction=reference', 'reconstructor Reference'],
    ]) {
      const url = `${origin}/?scene=sprites&${query}`;
      await cdp.call('Page.navigate', { url }, sessionId);
      const status = await waitFor(async () => {
        const result = await cdp.call('Runtime.evaluate', {
          expression: `location.href === ${JSON.stringify(url)} && document.querySelector("#status")?.textContent`,
          returnByValue: true,
        }, sessionId).catch(error => {
          if (/context.*destroyed|cannot find .*context/i.test(error.message)) return undefined;
          throw error;
        });
        const text = result?.result.value;
        return typeof text === 'string' && /^(Startup fault:|Running)/.test(text) && text;
      }, timeout, `sprite launch with ${query}`);
      if (!status.startsWith('Startup fault:') || !status.includes(`2D scene refuses ${expected}`)) {
        throw new Error(`Sprite launch with ${query}: ${status}`);
      }
      rows.push({ query, status });
    }
    return rows;
  } finally {
    await cdp.call('Target.closeTarget', { targetId });
  }
}

export async function qualifySprites({ cdp, origin, output, mode, timeout, wasm, existingGame,
  destroyActualDevice, waitFor }) {
  const cost = mode === 'sprites-2d-cost';
  const refusals = cost ? undefined : await checkStartupRefusals(cdp, origin, timeout, waitFor);
  const cases = cost ? [[2, 1920, 1080], [2, 3456, 2234]]
    : [1, 2, 3, 4, 6].map(scale => [scale, 1280, 720]);
  const rows = [];
  for (const [scale, width, height] of cases) {
    const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
    const call = (method, params = {}) => cdp.call(method, params, sessionId);
    const faults = [];
    let destroyed = false;
    cdp.event = packet => {
      if (packet.sessionId !== sessionId) return;
      if (packet.method === 'Runtime.exceptionThrown') faults.push(packet.params.exceptionDetails);
      if (packet.method === 'Runtime.consoleAPICalled' && packet.params.type === 'error') {
        const message = packet.params.args.map(arg => arg.value ?? arg.description).join(' ');
        if (!destroyed || !message.startsWith('Graphics fault: Destroyed:')) faults.push(message);
      }
    };
    try {
      await call('Page.enable');
      await call('Runtime.enable');
      await installWasmReceipt(cdp, sessionId);
      await call('Emulation.setDeviceMetricsOverride', {
        width: Math.floor(width / (cost && width > 1920 ? 2 : 1)),
        height: Math.floor(height / (cost && width > 1920 ? 2 : 1)),
        deviceScaleFactor: cost && width > 1920 ? 2 : 1, mobile: false,
      });
      await cdp.call('Target.activateTarget', { targetId });
      await call('Page.navigate', { url: `${origin}/?scene=sprites&scale=${scale}&backing=${width}x${height}` });
      const evaluate = async expression => {
        const result = await call('Runtime.evaluate', { expression, returnByValue: true });
        if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
        return result.result.value;
      };
      const health = async () => {
        const status = await evaluate('document.querySelector("#status")?.textContent');
        if (faults.length || (/fault:/i.test(status ?? '') && !(destroyed && /^Graphics fault: Destroyed:/.test(status)))) {
          throw new Error(`Chrome sprite scene: ${status} ${JSON.stringify(faults)}`);
        }
        return status;
      };
      await waitFor(async () => (await health())?.startsWith('Running'), timeout, 'sprite page startup');
      const game = await existingGame(cdp, sessionId);
      const report = () => game.invoke('return JSON.parse(game.sprite_scene_report());');
      const row = { scale, extent: [width, height], submitted: [] };
      const checkGraph = result => {
        if (result.kind !== 'TwoD' || result.reconstructor !== null ||
            JSON.stringify(result.passes) !== JSON.stringify(graphPasses)) throw new Error(JSON.stringify(result));
      };
      const readCard = async (w, h, generation) => {
        await waitFor(async () => (await report()).storage.includes(`bytes=${w * h * 8}\n`),
          timeout, `${w}×${h} sprite targets`);
        await game.invoke('game.begin_submitted_capture(args[0]);return true;', [row.submitted.length + 1]);
        const frame = await waitFor(async () => {
          await health();
          return game.invoke(`
            const image = game.next_submitted_image();
            if (!image) return null;
            const metadata = JSON.parse(image.metadata);
            if (!metadata.surface_presented) return null;
            const compare = ${compareCard.toString()};
            const mismatch = compare(image.pixels, game.sprite_card_texels(), ...args);
            return {metadata, mismatch, pixels: image.pixels.length / 4};
          `, [w, h, scale]);
        }, timeout, 'ordinary submitted sprite frame');
        await game.invoke('game.end_submitted_capture();return true;');
        if (frame.mismatch || frame.metadata.device_generation !== generation ||
            frame.metadata.pixel_source !== 'Sdr') throw new Error(JSON.stringify(frame));
        const plan = await report();
        checkGraph(plan);
        row.submitted.push({ extent: [w, h], passes: plan.passes, ...frame });
      };
      if (cost) {
        const measured = await waitFor(async () => {
          await health();
          const result = await report();
          return result.samples.length >= 110 ? result : undefined;
        }, timeout, '110 completed sprite timing samples');
        checkGraph(measured);
        row.measurement = { ...measured, samples: measured.samples.slice(20, 110) };
      } else {
        await readCard(width, height, 1);
        if (scale === 4) {
          await game.invoke('const canvas=document.querySelector("canvas");canvas.width=1121;canvas.height=657;return true;');
          await readCard(1121, 657, 1);
          await destroyActualDevice(cdp, sessionId, undefined, () => { destroyed = true; });
          await waitFor(() => game.invoke('return !!game.graphics_fault();'), timeout, 'actual WebGPU device loss');
          await game.invoke('document.querySelector("#restore-graphics").click();return true;');
          await waitFor(async () => {
            const state = await game.read();
            if (state.recoveryError) throw new Error(state.recoveryError);
            return !state.fault && !state.recovering && (await report()).generation === 2;
          }, timeout, 'replacement device presented');
          await readCard(1121, 657, 2);
        }
      }
      row.wasm = await readWasmReceipt(cdp, sessionId, `${origin}/v1_game_bg.wasm`, wasm.sha256, wasm.bytes.length);
      await health();
      rows.push(row);
      console.log(`${mode}: scale=${scale} ${width}×${height} passed`);
    } finally {
      cdp.event = () => {};
      await cdp.call('Target.closeTarget', { targetId });
    }
  }
  const record = { case: mode, target: 'chrome-webgpu', refusals, rows };
  await fs.writeFile(path.join(output, `${mode}.json`), JSON.stringify(record, null, 2));
  return record;
}

// Qualify the simulation-backed field through the ordinary page and surface.
export async function qualifySpriteField({ cdp, origin, output, mode, timeout, wasm, existingGame, waitFor, commit }) {
  const rows = [];
  const width = 1920, height = 1080;
  for (const count of mode === 'sprites-field' ? [10000] : [2000, 5000, 10000]) {
    const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
    const call = (method, params = {}) => cdp.call(method, params, sessionId);
    const url = `${origin}/?scene=sprites&rows=${count}&backing=${width}x${height}`;
    try {
      await call('Page.enable');
      await call('Runtime.enable');
      await call('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
      // Headless macOS can throttle RAF to 30 Hz. The function check
      // observes elapsed sub-tick samples. The cost take uses fixed input
      // timestamps to keep the same moving-row workload under varying load.
      const fixedClock = mode === 'sprites-field-cost';
      await call('Page.addScriptToEvaluateOnNewDocument', { source: `{
        let base;
        let index = 0;
        let framesRequested = true;
        window.stopSpriteQualification = () => { framesRequested = false; };
        window.requestAnimationFrame = callback => window.setTimeout(() => {
          const now = performance.now();
          base ??= now;
          if (framesRequested) callback(${fixedClock} ? base + index++ * 1000 / 120 : now);
        }, 1000 / 120);
        window.cancelAnimationFrame = id => window.clearTimeout(id);
      }` });
      await installWasmReceipt(cdp, sessionId);
      await cdp.call('Target.activateTarget', { targetId });
      await call('Page.navigate', { url });
      await waitFor(async () => {
        const status = await call('Runtime.evaluate', {
          expression: 'document.querySelector("#status")?.textContent', returnByValue: true,
        });
        const message = status.result.value;
        if (/fault:/i.test(message ?? '')) throw new Error(message);
        return message?.startsWith('Running');
      }, timeout, 'ordinary sprite field startup');
      const game = await existingGame(cdp, sessionId);
      const result = await waitFor(async () => {
        const report = await game.invoke('return JSON.parse(game.sprite_scene_report());');
        if (report.kind !== 'TwoD' || report.reconstructor !== null ||
            JSON.stringify(report.passes) !== JSON.stringify(graphPasses)) throw new Error(JSON.stringify(report));
        if (report.samples.some(({ sprites: s }) => !s?.surface_presented || s.world_rows !== count ||
            s.frame_rows !== 2 || s.draws !== 1 || s.dropped !== 0)) throw new Error('Unexpected WebGPU sprite-field admission');
        const moving = report.samples.filter(({ sprites: s }) => s.moving === count && s.world_upload_rows === count);
        const interpolating = report.samples.filter(({ sprites: s }) => s.moving === count && s.alpha > 0 && s.alpha < 1);
        if (moving.length >= 52 && interpolating.length >= 10) {
          return { ...report, samples: moving.slice(12, 52), fractional_frames: interpolating.length };
        }
        return undefined;
      }, timeout, `complete WebGPU ${count}-row field samples`);
      await call('Runtime.evaluate', { expression: 'window.stopSpriteQualification()' });
      const proof = await readWasmReceipt(cdp, sessionId, `${origin}/v1_game_bg.wasm`, wasm.sha256, wasm.bytes.length);
      rows.push({ world_rows: count, extent: [width, height], measurement: result, wasm: proof });
      console.log(`${mode}: ${count} world rows, 2 frame rows, one draw, ${result.fractional_frames} interpolating frames`);
    } finally {
      await cdp.call('Target.closeTarget', { targetId });
    }
  }
  const record = { case: mode, target: 'chrome-webgpu', commit, rows,
    schedule: mode === 'sprites-field-cost'
      ? 'ordinary page with fixed 120 Hz input timestamps and elapsed CPU/GPU measurements'
      : 'ordinary page callbacks at elapsed timer opportunities, 120 Hz requested' };
  await fs.writeFile(path.join(output, `${mode}.json`), JSON.stringify(record, null, 2));
  return record;
}

function markerCentroid(bytes, width, height) {
  if (bytes.length !== width * height * 8) throw new Error('Invalid scene-linear sweep extent');
  const data = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let mass = 0;
  let mx = 0;
  let my = 0;
  for (let y = height / 2 - 4; y < height / 2 + 6; y++) {
    for (let x = width / 2 - 4; x < width / 2 + 6; x++) {
      const red = data.getFloat16(8 * (y * width + x), true);
      mass += red;
      mx += x * red;
      my += y * red;
    }
  }
  if (!(mass > 0)) throw new Error('Submitted marker is missing');
  return [mx / mass, my / mass];
}

function movesDiagonally(positions) {
  return positions.slice(1).every((point, i) => point.every((axis, j) => axis > positions[i][j]));
}

export async function qualifySpriteCrisp({ cdp, origin, output, timeout, wasm, existingGame, waitFor, releaseGameGraphics, unloadGamePage, commit }) {
  const rows = [];
  for (const dpr of [1, 2]) {
    const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
    const call = (method, params = {}) => cdp.call(method, params, sessionId);
    try {
      await call('Page.enable');
      await call('Runtime.enable');
      await installWasmReceipt(cdp, sessionId);
      await call('Emulation.setDeviceMetricsOverride', { width: 668, height: 550, deviceScaleFactor: dpr, mobile: false });
      await cdp.call('Target.activateTarget', { targetId });
      const visit = async query => {
        const url = `${origin}/?scene=sprites&scale=2${query}`;
        await call('Page.navigate', { url });
        await waitFor(async () => {
          const state = (await call('Runtime.evaluate', {
            expression: `location.href === ${JSON.stringify(url)} && document.querySelector('#status')?.textContent`,
            returnByValue: true,
          })).result.value;
          if (/fault:/i.test(state ?? '')) throw new Error(state);
          return typeof state === 'string' && state.startsWith('Running');
        }, timeout, 'ordinary sprite canvas startup');
        return existingGame(cdp, sessionId);
      };
      const retireCanvas = async game => {
        // Retire the surface and acknowledge device destruction while this
        // document still owns them. Each navigation creates a fresh device.
        const devices = await releaseGameGraphics(cdp, sessionId, game);
        if (!Number.isInteger(devices) || devices < 1) {
          throw new Error(`DPR ${dpr}: no sprite WebGPU device retired`);
        }
        // Give the compositor two callbacks to release the last canvas image.
        await call('Runtime.evaluate', {
          expression: 'new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))',
          awaitPromise: true,
        });
        return devices;
      };
      const surface = async (game, css) => {
        const grid = await waitFor(() => game.invoke(`
          const c = document.querySelector('canvas');
          const r = c.getBoundingClientRect();
          if (args[0] && (r.width !== args[0][0] || r.height !== args[0][1])) return null;
          if (c.width !== Math.round(r.width * devicePixelRatio) ||
              c.height !== Math.round(r.height * devicePixelRatio)) return null;
          const report = JSON.parse(game.sprite_scene_report());
          if (!report.storage.includes(\`bytes=\${c.width*c.height*8}\\n\`)) return null;
          return { css: [r.width, r.height], backing: [c.width, c.height], dpr: devicePixelRatio, passes: report.passes };
        `, [css]), timeout, 'CSS/device pixel and sprite grids match');
        if (grid.dpr !== dpr || JSON.stringify(grid.passes) !== JSON.stringify(graphPasses)) {
          throw new Error(JSON.stringify(grid));
        }
        return grid;
      };
      const capture = async (game, source, grid, body, args) => {
        if (source === 'Sdr') {
          await game.invoke('game.begin_submitted_capture(1);return true;');
        }
        try {
          return await waitFor(() => game.invoke(`
            const image = game.next_submitted_image();
            if (!image) return null;
            const meta = JSON.parse(image.metadata);
            if (!meta.surface_presented || meta.pixel_source !== args[0] ||
                image.pixels.length !== args[1] * args[2] * (args[0] === "Sdr" ? 4 : 8)) return null;
            ${body}
          `, [source, ...grid.backing, ...args]), timeout, `submitted ${source} sprite image`);
        } finally {
          await game.invoke('game.end_submitted_capture();return true;');
        }
      };
      const card = await visit('');
      const cardRows = [];
      for (const css of [null, [517, 293]]) {
        if (css) await card.invoke(`
          const c = document.querySelector('canvas');
          c.style.width = args[0] + 'px'; c.style.height = args[1] + 'px';
          return true;
        `, css);
        const grid = await surface(card, css);
        const frame = await capture(card, 'Sdr', grid, `
          const compare = ${compareCard.toString()};
          return {meta, mismatch: compare(image.pixels, game.sprite_card_texels(), args[1], args[2], 2)};
        `, []);
        if (frame.mismatch) throw new Error(JSON.stringify(frame));
        cardRows.push({ ...grid, submission: frame.meta.submission_id, presenter: frame.meta.presenter_generation });
      }
      if (cardRows[0].presenter === cardRows[1].presenter) throw new Error('Resize did not change the presenter generation');
      const cardDevicesRetired = await retireCanvas(card);
      const sweeps = [];
      for (const scale of [1.25, 1.5, 2.37]) {
        const game = await visit(`&sprite-probe=sweep:${scale}`);
        const grid = await surface(game, [64, 64]);
        const checkpoints = [0, 64, 128, 192, 255];
        const positions = [];
        const frozen = [];
        let last = -1;
        for (let start = 0; start < 256; start += 16) {
          const batch = await game.invoke(`return (async () => {
            const centroid = ${markerCentroid.toString()};
            const [width, height, checkpoints, start, previousSubmission] = args;
            const positions = [];
            const frozen = [];
            let last = previousSubmission;
            const delay = () => new Promise(resolve => setTimeout(resolve, 0));
            for (let step = start; step < start + 16; step++) {
              const deadline = performance.now() + 10000;
              const presentedBefore = game.presented();
              game.set_sprite_sweep_step(step);
              while (game.presented() === presentedBefore) {
                if (performance.now() > deadline) throw new Error("Sprite sweep did not present pose " + step);
                if (game.route_slot_ready()) game.frame(performance.now());
                if (game.presented() === presentedBefore) await delay();
              }
              let image;
              while (!(image = game.next_submitted_image())) {
                if (performance.now() > deadline) throw new Error("Sprite sweep readback did not retire at pose " + step);
                await delay();
              }
              const meta = JSON.parse(image.metadata);
              if (!meta.surface_presented || meta.pixel_source !== "SceneLinear" || meta.submission_id <= last) {
                throw new Error(JSON.stringify({step, last, meta}));
              }
              last = meta.submission_id;
              if (step === 0) window.spriteSweepFirstPixels = image.pixels.slice();
              if (checkpoints.includes(step)) {
                positions.push(centroid(image.pixels, width, height));
                frozen.push(centroid(window.spriteSweepFirstPixels, width, height));
              }
            }
            return {last, positions, frozen};
          })();`, [...grid.backing, checkpoints, start, last]);
          last = batch.last;
          positions.push(...batch.positions);
          frozen.push(...batch.frozen);
        }
        await game.invoke('delete window.spriteSweepFirstPixels;game.end_submitted_capture();return true;');
        const frozenRejected = !movesDiagonally(frozen);
        if (!movesDiagonally(positions) || !frozenRejected) throw new Error(JSON.stringify({positions, frozen}));
        const devicesRetired = await retireCanvas(game);
        sweeps.push({
          scale, ...grid, submissions: 256, checkpoints, centroids: positions, frozen_centroids: frozen,
          frozen_rejected: frozenRejected, last_submission: last, devices_retired: devicesRetired,
        });
      }
      const receipt = await readWasmReceipt(cdp, sessionId, `${origin}/v1_game_bg.wasm`, wasm.sha256, wasm.bytes.length);
      await unloadGamePage(cdp, sessionId, timeout);
      rows.push({ dpr, card: cardRows, card_devices_retired: cardDevicesRetired, sweeps, receipt });
      console.log(`sprites-crisp: DPR ${dpr}, two CSS grids and 3 × 256 submitted camera poses passed`);
    } finally {
      await cdp.call('Target.closeTarget', { targetId });
    }
  }
  const record = { target: 'chrome-webgpu', commit, offsets: 256, rows };
  await fs.writeFile(path.join(output, 'sprites-crisp.json'), JSON.stringify(record, null, 2) + '\n');
  return record;
}
