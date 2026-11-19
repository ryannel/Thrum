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
