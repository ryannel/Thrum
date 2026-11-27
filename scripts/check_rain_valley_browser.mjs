// Exercise the production V1 page and read its committed engine output on Chrome WebGPU.
import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';

function hash(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

async function capture(game, waitFor, timeout, output, name) {
  const identity = await game.invoke('return game.begin_capture();');
  const length = await waitFor(() => game.invoke(`
    const pixels = game.try_finish_capture();
    if (!pixels) return;
    globalThis.rainCapture = pixels;
    return pixels.length;
  `), timeout, `${name} engine readback`);
  const chunks = [];
  for (let offset = 0; offset < length; offset += 0x40000) {
    const encoded = await game.invoke(`
      const slice = globalThis.rainCapture.subarray(args[0], args[0] + 0x40000);
      let bytes = "";
      for (let i = 0; i < slice.length; i += 0x8000) {
        bytes += String.fromCharCode.apply(null, slice.subarray(i, i + 0x8000));
      }
      return btoa(bytes);
    `, [offset]);
    chunks.push(Buffer.from(encoded, 'base64'));
  }
  await game.invoke('delete globalThis.rainCapture;');
  const pixels = Buffer.concat(chunks);
  const [, width, height] = /extent=(\d+)x(\d+)/.exec(identity);
  if (+width * +height * 4 !== length || pixels.length !== length) {
    throw new Error(`${name} engine image has an invalid extent`);
  }
  await fs.writeFile(path.join(output, `${name}.rgba`), pixels);
  return { identity, width: +width, height: +height, sha256: hash(pixels), pixels };
}

function groundLuminance({ width, height, pixels }) {
  let sum = 0;
  let count = 0;
  // Terrain away from the patch and the two cooked props at the lookout.
  for (let y = Math.floor(height * 0.82); y < height * 0.97; y++) {
    for (let x = Math.floor(width * 0.07); x < width * 0.27; x++) {
      const i = (y * width + x) * 4;
      sum += 0.2126 * pixels[i] + 0.7152 * pixels[i + 1] + 0.0722 * pixels[i + 2];
      count++;
    }
  }
  return sum / count;
}

export async function qualifyRainValley({ cdp, origin, assetsOrigin, selectedAssetsOrigin, output, timeout, waitFor, existingGame }) {
  async function run(selection, body) {
    const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
    const failures = [];
    cdp.event = packet => {
      if (packet.sessionId !== sessionId) return;
      if (packet.method === 'Runtime.exceptionThrown' ||
          (packet.method === 'Runtime.consoleAPICalled' && packet.params.type === 'error')) {
        failures.push(packet.params);
      }
    };
    try {
      await cdp.call('Runtime.enable', {}, sessionId);
      if (selection !== 'shower') {
        // Freeze the page clock for the endpoint control so distinct load times
        // cannot advance the original mover loops by different tick counts.
        await cdp.call('Page.addScriptToEvaluateOnNewDocument', { source: `(() => {
          const fixed = 1000;
          Object.defineProperty(performance, 'now', { value: () => fixed });
          const raf = window.requestAnimationFrame.bind(window);
          window.requestAnimationFrame = callback => raf(() => callback(fixed));
        })()` }, sessionId);
      }
      const query = new URLSearchParams({
        scene: 'exit',
        assets: selection === 'shower' ? selectedAssetsOrigin : assetsOrigin,
        'route-check': '', params: 'showcase-noon.params.json',
      });
      if (selection) query.set('rain', selection);
      await cdp.call('Page.navigate', { url: `${origin}/?${query}` }, sessionId);
      await waitFor(async () => {
        if (failures.length) throw new Error(JSON.stringify(failures));
        const { result } = await cdp.call('Runtime.evaluate', {
          expression: 'document.querySelector("#status")?.textContent', returnByValue: true,
        }, sessionId);
        return /^Running/.test(result.value ?? '');
      }, timeout, `${selection ?? 'default'} ready on cooked rows`);
      const game = await existingGame(cdp, sessionId);
      await waitFor(async () => {
        if (failures.length) throw new Error(JSON.stringify(failures));
        const rows = await game.invoke('return JSON.parse(game.exit_status());');
        return rows.row_generation === 1 && rows.active === 10000;
      }, timeout, `${selection ?? 'default'} cooked row submission`);
      const result = await body(game);
      if (failures.length || (await game.read()).fault) throw new Error(JSON.stringify(failures));
      return result;
    } finally {
      cdp.event = () => {};
      await cdp.call('Target.closeTarget', { targetId });
    }
  }

  const set = (game, values) => game.invoke(`
    const result = JSON.parse(game.apply_parameters(JSON.stringify(args[0]), performance.now()));
    if (result.problems.length) throw new Error(result.problems.join(", "));
    return true;
  `, [values]);
  const observe = game => game.invoke('return JSON.parse(game.rain_observation() ?? "null");');
  const snap = (game, name) => capture(game, waitFor, timeout, output, name);
  const dryExit = (selection, name) => run(selection, async game => {
    if (await observe(game) !== null) throw new Error('Exit prepared a rain projection by default');
    await set(game, { 'world.paused': true, 'sky.solar_time': 12, 'camera.pitch': -21, 'camera.yaw': 0 });
    const rows = await game.invoke('return JSON.parse(game.exit_status());');
    if (rows.active !== 10000 || rows.row_generation !== 1) throw new Error(`Missing original exit rows: ${JSON.stringify(rows)}`);
    return snap(game, name);
  });
  const original = await dryExit(null, 'exit-default');
  const off = await dryExit('off', 'rain-off');
  const endpoint = identity => identity.replace(/^tick=\d+;/, '');
  if (endpoint(original.identity) !== endpoint(off.identity)) {
    throw new Error(`Exit extent or device generation differs: ${original.identity} and ${off.identity}`);
  }
  // Store attachment can admit a few startup ticks before the pause applies.
  // Bound that mover difference across fresh pages. Metal also compares exact
  // frame bytes with both arms held at the same world endpoint.
  let totalDelta = 0;
  for (let i = 0; i < original.pixels.length; i++) {
    totalDelta += Math.abs(original.pixels[i] - off.pixels[i]);
  }
  const offMeanByteDelta = totalDelta / original.pixels.length;
  if (offMeanByteDelta > 0.1) throw new Error(`Rain off changed the exit image by ${offMeanByteDelta} bytes per channel`);

  const rainy = await run('shower', async game => {
    await set(game, { 'camera.pitch': -21 });
    const dry = await snap(game, 'rain-dry');
    const before = await observe(game);
    if (before.wet_depths_m.some(depth => depth > 0) || before.submitted_hollow_vertices !== 0) {
      throw new Error(`Dry patch submitted water ${JSON.stringify(before)}`);
    }
    await set(game, { 'world.paused': false });
    const falling = await waitFor(async () => {
      const row = await observe(game);
      return row.tick >= 270 && row.wet_depths_m.every(depth => depth > 0.012) &&
        row.submitted_hollow_vertices > 0 && row.rain_drops?.some(Boolean) ? row : undefined;
    }, timeout, 'committed rain, submitted live drops and two standing puddles');
    await set(game, { 'world.paused': true });
    const soaked = await snap(game, 'rain-soaked');
    const dryLuminance = groundLuminance(dry);
    const soakedLuminance = groundLuminance(soaked);
    if (!(soakedLuminance < dryLuminance * 0.94)) {
      throw new Error(`Cooked terrain did not darken enough: ${dryLuminance} to ${soakedLuminance}`);
    }
    return { dry, soaked, dryLuminance, soakedLuminance, before, falling };
  });
  const metadata = ({ pixels, ...rest }) => rest;
  const record = { case: 'rain', offMeanByteDelta, original: metadata(original), off: metadata(off),
    ...rainy, dry: metadata(rainy.dry), soaked: metadata(rainy.soaked) };
  await fs.writeFile(path.join(output, 'rain.json'), JSON.stringify(record, null, 2));
  return record;
}
