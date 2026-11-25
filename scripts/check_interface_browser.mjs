// Chrome's ordinary game and submitted-image tap qualify both interface paths.
import fs from 'node:fs/promises';
import path from 'node:path';
import { installWasmReceipt, readWasmReceipt } from './wasm_receipt.mjs';

const PASS = 'interface stage';
const marks = [
  [255, 134, 61], [105, 247, 185], [223, 229, 255],
  [48, 70, 192], [185, 70, 237], [6, 117, 110],
];

function colour(plain, actual, width, height, scale) {
  const left = Math.floor(width / (4 * scale));
  const top = Math.floor(height / (4 * scale));
  const boxes = [
    [1, 8, 8, 9, 4], [2, 111, 8, 9, 6], [4, 8, 59, 4, 7], [5, 114, 62, 6, 4],
    [3, 22, 19, 40, 2], [2, 22, 28, 54, 6], [6, 76, 28, 24, 6],
    ...marks.map((_, index) => [index + 1, 22 + index * 13, 45, 11, 8]),
  ].map(([frame, x, y, w, h]) => [frame, x + left, y + top, w, h]);
  boxes.push([3, width / scale - 10, height / scale - 10, 4, 4]);
  boxes.reverse();
  if (plain.length !== width * height * 4 || plain.length !== actual.length) {
    throw new Error('unexpected submitted image extent');
  }
  const half = Uint8Array.from({ length: 256 }, (_, value) => {
    const code = value / 255;
    const linear = code <= 0.04045 ? code / 12.92 : ((code + 0.055) / 1.055) ** 2.4;
    return Math.round(255 * (linear * 0.5 <= 0.0031308
      ? linear * 0.5 * 12.92 : 1.055 * (linear * 0.5) ** (1 / 2.4) - 0.055));
  });
  const covered = (x, y, left, top, w, h) =>
    x >= left * scale && x < (left + w) * scale && y >= top * scale && y < (top + h) * scale;
  let opaque = 0, blended = 0, uncovered = 0;
  for (let i = 0; i < actual.length; i += 4) {
    const x = (i / 4) % width;
    const y = Math.floor(i / 4 / width);
    const box = boxes.find(([, left, top, w, h]) => covered(x, y, left, top, w, h));
    const scrim = !box && covered(x, y, left + 8, top + 8, 112, 58);
    if (box) opaque++;
    else if (scrim) blended++;
    else uncovered++;
    for (let channel = 0; channel < 4; channel++) {
      const expected = box ? (marks[box[0] - 1][channel] ?? 255)
        : scrim && channel < 3 ? half[plain[i + channel]] : plain[i + channel];
      if (Math.abs(actual[i + channel] - expected) > Number(scrim && channel < 3)) {
        throw new Error(`interface pixel ${x},${y} channel ${channel}: ${actual[i + channel]} expected ${expected}`);
      }
    }
  }
  if (!opaque || !blended || !uncovered) throw new Error('CPU panel proof omitted an arm');
  return { opaque, blended, uncovered };
}

export async function qualifyInterface({ cdp, origin, output, mode, timeout, wasm, existingGame,
  destroyActualDevice, waitFor, commit }) {
  const cases = mode === 'interface-stage-cost'
    ? [{ dpr: 1, grid: [1920, 1080] }, { dpr: 2, grid: [3456, 2234] }]
    : [{ dpr: 1, grid: [384, 264] }, { dpr: 2, grid: [768, 528] }];
  const exact = mode === 'interface-stage';
  const rows = [];
  for (const { dpr, grid } of cases) {
    const [width, height] = grid;
    const opens = [];
    const open = async (iface, paintRows) => {
      const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
      opens.push(targetId);
      const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
      const call = (method, args = {}) => cdp.call(method, args, sessionId);
      await call('Page.enable');
      await call('Runtime.enable');
      await installWasmReceipt(cdp, sessionId);
      await call('Emulation.setDeviceMetricsOverride', {
        width: width / dpr, height: height / dpr, deviceScaleFactor: dpr, mobile: false,
      });
      await call('Page.addScriptToEvaluateOnNewDocument', { source: `{
        const createView = GPUTexture.prototype.createView;
        window.interfaceSrgbViews = 0;
        GPUTexture.prototype.createView = function(desc) {
          const view = createView.apply(this, arguments);
          if (this.format === 'rgba8unorm' && desc?.format === 'rgba8unorm-srgb') {
            ++window.interfaceSrgbViews;
          }
          return view;
        };
        ${exact ? 'requestAnimationFrame = callback => setTimeout(() => callback(1000), 1); cancelAnimationFrame = clearTimeout;' : ''}
      }` });
      await cdp.call('Target.activateTarget', { targetId });
      const setting = paintRows === undefined ? '' : `&paint-rows=${paintRows}`;
      await call('Page.navigate', { url: `${origin}/?scene=${iface ? 'interface' : 'sprites'}&rows=10000&backing=${width}x${height}${setting}` });
      await waitFor(async () => {
        const reply = await call('Runtime.evaluate', {
          expression: 'document.querySelector("#status")?.textContent', returnByValue: true,
        });
        const status = reply.result.value;
        if (/fault:/i.test(status ?? '')) throw new Error(status);
        return status?.startsWith('Running');
      }, timeout, 'interface page startup');
      const game = await existingGame(cdp, sessionId);
      const report = () => game.invoke('return JSON.parse(game.sprite_scene_report());');
      const graph = await report();
      if (graph.passes.includes(PASS) !== iface || graph.reconstructor !== null) {
        throw new Error(`interface graph mismatch: ${JSON.stringify(graph.passes)}`);
      }
      const build = await readWasmReceipt(cdp, sessionId, `${origin}/v1_game_bg.wasm`, wasm.sha256, wasm.bytes.length);
      const srgbViews = await game.invoke('return window.interfaceSrgbViews;');
      if ((srgbViews > 0) !== iface) throw new Error('unexpected WebGPU sRGB display views');
      return { game, report, build, sessionId, targetId, graph, srgbViews };
    };
    let session = 0;
    const image = async (arm, grid, generation) => {
      await cdp.call('Target.activateTarget', { targetId: arm.targetId });
      await waitFor(async () => (await arm.report()).storage.includes(`bytes=${grid[0] * grid[1] * 8}\n`),
        timeout, `${grid.join('×')} prepared interface attachments`);
      await arm.game.invoke('game.begin_submitted_capture(args[0]);return true;', [++session]);
      try {
        return await waitFor(async () => {
          const captured = await arm.game.invoke(`
            const frame = game.next_submitted_image();
            if (!frame) return null;
            const metadata = JSON.parse(frame.metadata);
            if (!metadata.surface_presented || frame.pixels.length !== args[0]) return null;
            return { metadata, rgba: frame.pixels.toBase64() };
          `, [grid[0] * grid[1] * 4]);
          if (captured && (captured.metadata.pixel_source !== 'Sdr' || captured.metadata.device_generation !== generation)) {
            throw new Error(JSON.stringify(captured.metadata));
          }
          return captured;
        }, timeout, `submitted ${grid.join('×')} interface comparison frame`);
      } finally {
        await arm.game.invoke('game.end_submitted_capture();return true;');
      }
    };
    try {
      if (exact) {
        const plain = await open(false);
        const painted = await open(true);
        const compare = async (grid, generation) => {
          const before = await image(plain, grid, 1);
          const after = await image(painted, grid, generation);
          if (before.metadata.source_tick !== after.metadata.source_tick) throw new Error('different scene source ticks');
          return {
            grid, plain: before.metadata, painted: after.metadata,
            ...colour(Buffer.from(before.rgba, 'base64'), Buffer.from(after.rgba, 'base64'), ...grid, dpr),
          };
        };
        const initial = await compare(grid, 1);
        const resizedGrid = [width + 3, height + 5];
        for (const arm of [plain, painted]) {
          await arm.game.invoke('const canvas = document.querySelector("canvas"); [canvas.width, canvas.height] = args; return true;', resizedGrid);
        }
        const resized = await compare(resizedGrid, 1);
        await destroyActualDevice(cdp, painted.sessionId, undefined, () => {});
        await waitFor(() => painted.game.invoke('return !!game.graphics_fault();'), timeout, 'WebGPU device loss');
        await painted.game.invoke('document.querySelector("#restore-graphics").click();return true;');
        await waitFor(async () => {
          const state = await painted.game.read();
          if (state.recoveryError) throw new Error(state.recoveryError);
          return !state.fault && !state.recovering && (await painted.report()).generation === 2;
        }, timeout, 'interface replacement device');
        const recovered = await compare(resizedGrid, 2);
        rows.push({
          dpr, build: painted.build, control_passes: plain.graph.passes, interface_passes: painted.graph.passes,
          srgb_views: painted.srgbViews, initial, resized, recovered,
        });
      } else {
        for (const [name, iface, paintRows] of [['absent', false], ['empty', true, 0], ['loaded', true, 2000]]) {
          const arm = await open(iface, paintRows);
          const report = await waitFor(async () => {
            const measured = await arm.report();
            return measured.samples.length >= 48 && measured;
          }, timeout, '48 completed interface cost samples');
          const samples = report.samples.slice(-32);
          if (iface && samples.some(sample => sample.interface?.gpu_ns == null)) throw new Error('missing interface GPU timestamp');
          rows.push({
            name, grid, dpr, build: arm.build, srgb_views: arm.srgbViews, passes: report.passes, samples,
            interface_load_bytes: iface ? width * height * 4 : 0,
            interface_store_bytes: iface ? width * height * 4 : 0,
          });
          await cdp.call('Target.closeTarget', { targetId: opens.pop() });
        }
      }
    } finally {
      for (const targetId of opens.reverse()) await cdp.call('Target.closeTarget', { targetId });
    }
  }
  const record = { case: mode, target: 'chrome-webgpu', commit, rows };
  await fs.writeFile(path.join(output, `${mode}.json`), JSON.stringify(record, null, 2));
  return record;
}
