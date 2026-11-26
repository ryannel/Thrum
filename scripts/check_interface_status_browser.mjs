// The regular page's first submitted pixels at each scale and after a canvas resize.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { installWasmReceipt, readWasmReceipt } from './wasm_receipt.mjs';

function compare(rgba, metadata, metrics, oracle, store) {
  const { paint_scale: scale, source_tick: tick, extent } = metadata;
  assert.deepEqual(extent, metrics.extent);
  assert.equal(scale, metrics.density);
  const insets = metrics.insets.map(inset => inset * metrics.density / scale);
  const proof = execFileSync(oracle, [store], {
    input: JSON.stringify([extent, scale, insets, tick]) + '\n',
    // The 3× panel and its per-pixel blend mask exceed Node's default 1 MiB.
    maxBuffer: 2 * 1024 * 1024,
  });
  const newline = proof.indexOf(10);
  const [region, size, maskSize] = JSON.parse(proof.subarray(0, newline));
  const expected = proof.subarray(newline + 1, newline + 1 + size);
  const tolerance = proof.subarray(newline + 1 + size);
  assert.equal(tolerance.length, maskSize);
  const [[left, top], [right, bottom]] = region;
  for (let y = top; y < bottom; y++) {
    for (let x = left; x < right; x++) {
      const actual = (y * extent[0] + x) * 4;
      const reference = (y - top) * (right - left) + x - left;
      for (let ch = 0; ch < 4; ch++) {
        const a = rgba[actual + ch], b = expected[reference * 4 + ch];
        assert(Math.abs(a - b) <= (ch < 3 ? tolerance[reference] : 0),
          `DPR ${scale} status at ${x},${y} channel ${ch}: ${a} expected ${b}`);
      }
    }
  }
  return region;
}

export async function qualifyInterfaceStatus({ cdp, origin, fontOrigin, output, timeout, wasm,
  existingGame, waitFor, releaseGameGraphics, unloadGamePage, commit, encodePng, oracle, costs }) {
  const rows = [];
  // Keep one tab while each DPR loads a fresh document and device. Retire
  // each page before changing the DPR so Chrome closes only one target.
  const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
  try {
    const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
    const call = (method, args = {}) => cdp.call(method, args, sessionId);
    await call('Page.enable');
    await call('Runtime.enable');
    await installWasmReceipt(cdp, sessionId);
    await call('Page.addScriptToEvaluateOnNewDocument', { source: `{
      const scheduled = new Map();
      let serial = 0, time = 0;
      requestAnimationFrame = callback => { scheduled.set(++serial, callback); return serial; };
      cancelAnimationFrame = id => scheduled.delete(id);
      window.stepStatusFrame = () => {
        const work = [...scheduled.values()];
        scheduled.clear();
        time += 17;
        for (const callback of work) callback(time);
      };
    }` });
    await cdp.call('Target.activateTarget', { targetId });
    for (const dpr of costs ? [2] : [1, 2, 3]) {
      await call('Emulation.setDeviceMetricsOverride', { width: 900, height: 760, deviceScaleFactor: dpr, mobile: false });
      await call('Page.navigate', { url: `${origin}/?scene=interface&text-card=status&assets=${encodeURIComponent(fontOrigin)}` });
      await waitFor(async () => {
        const { result } = await call('Runtime.evaluate', { expression: 'document.querySelector("#status")?.textContent', returnByValue: true });
        if (/fault:/i.test(result.value ?? '')) throw new Error(result.value);
        return result.value?.startsWith('Running');
      }, timeout, 'status display startup');
      const game = await existingGame(cdp, sessionId);
      const build = await readWasmReceipt(cdp, sessionId, `${origin}/v1_game_bg.wasm`, wasm.sha256, wasm.bytes.length);
      const retire = async () => {
        const devices = await releaseGameGraphics(cdp, sessionId, game);
        assert(devices > 0, 'Chrome device was retired');
        await unloadGamePage(cdp, sessionId, timeout);
        return devices;
      };
      if (costs) {
        await game.invoke('document.querySelector("canvas").style.cssText = "width:960px;height:540px";game.set_status_cost_workload();');
        const layout = await game.invoke('return JSON.parse(game.status_layout_costs());');
        const samples = [];
        const report = () => game.invoke('game.route_slot_ready();return JSON.parse(game.status_cost_report());');
        for (let i = 0; i < 180 && samples.length < 40; i++) {
          // A resize, unavailable canvas texture, or occupied frame slot can
          // leave a callback without a presentation. Keep driving the page
          // until it presents, then stop admitting work while its timing retires.
          const before = await waitFor(() => game.invoke(`
            if (!game.route_slot_ready()) return null;
            const before = JSON.parse(game.status_cost_report());
            const presented = game.presented();
            window.stepStatusFrame();
            return game.presented() > presented ? before : null;
          `), timeout, 'status cost frame presentation');
          const arm = await waitFor(async () => {
            const newest = await report();
            return newest.frame?.submission > (before.frame?.submission ?? -1) && newest;
          }, timeout, 'status cost frame retirement');
          if (arm.frame.stamp !== dpr || !before.counts || !arm.counts) continue;
          const [layouts, lists, rows] = arm.counts.map((count, index) => count - before.counts[index]);
          samples.push({ ...arm.frame, layouts, lists, rows });
        }
        assert.equal(samples.length, 40);
        const record = { target: 'chrome-webgpu', commit, build, layout, samples: samples.slice(8),
          devices_retired: await retire() };
        await fs.writeFile(path.join(output, 'interface-status-cost.json'), JSON.stringify(record, null, 2) + '\n');
        return record;
      }
      await game.invoke('document.querySelector("canvas").style.cssText = "width:420px;height:320px";game.begin_fixed_submission(args[0]);', [dpr]);
      const frame = async () => {
        let submitted = false;
        for (let attempt = 0; attempt < 15 && !submitted; attempt++) {
          submitted = await game.invoke(`
            game.route_slot_ready();
            const before = game.presented();
            window.stepStatusFrame();
            return game.presented() > before;
          `);
          if (!submitted) await new Promise(resolve => setTimeout(resolve, 4));
        }
        assert(submitted, 'a manual step presented on the ordinary canvas');
        const take = await waitFor(() => game.invoke(`
          const frame = game.next_submitted_image();
          if (!frame) return null;
          window.statusImage = frame.pixels;
          return { metadata: JSON.parse(frame.metadata), metrics: window.thrumInterfaceOutput() };
        `), timeout, 'a submitted status display frame');
        assert.equal(take.metadata.pixel_source, 'Sdr');
        assert.equal(take.metadata.surface_presented, true);
        assert.equal(take.metadata.paint_stamp_mismatch, false);
        return take;
      };
      const keep = async (take, name) => {
        const [width, height] = take.metrics.extent;
        const chunks = [];
        for (let offset = 0; offset < width * height * 4; offset += 256 * 1024) {
          chunks.push(Buffer.from(await game.invoke('return window.statusImage.subarray(args[0], args[1]).toBase64();',
            [offset, offset + 256 * 1024]), 'base64'));
        }
        const rgba = Buffer.concat(chunks);
        assert.equal(rgba.length, width * height * 4);
        const region = compare(rgba, take.metadata, take.metrics, oracle, path.join(output, 'store'));
        const filename = `status-${dpr}-${name}.png`;
        await fs.writeFile(path.join(output, filename), encodePng(width, height, rgba));
        return { ...take, filename, region, rgba_sha256: createHash('sha256').update(rgba).digest('hex') };
      };
      let initial;
      for (let current = 0; current < 180 && !initial; current++) {
        const take = await frame();
        if (take.metadata.paint_scale === dpr) initial = await keep(take, 'initial');
      }
      assert(initial, 'resident status display');
      await game.invoke('document.querySelector("canvas").style.cssText = "width:512px;height:360px";');
      await waitFor(() => game.invoke('return document.querySelector("canvas").getBoundingClientRect().width === 512;'), timeout, 'new CSS canvas width');
      let resized;
      for (let i = 0; i < 16 && !resized; i++) {
        const take = await frame();
        if (take.metrics.extent[0] === 512 * dpr && take.metadata.extent?.[0] === 512 * dpr) resized = await keep(take, 'resized');
      }
      assert(resized, 'resized resident status display');
      assert(resized.metadata.presenter_generation > initial.metadata.presenter_generation);
      assert.equal(await game.invoke('return game.submitted_capture_omissions();'), 0);
      await game.invoke('game.end_submitted_capture();delete window.statusImage;');
      rows.push({ dpr, build, initial, resized, devices_retired: await retire() });
    }
  } finally {
    await cdp.call('Target.closeTarget', { targetId });
  }
  const record = { case: 'interface-status', target: 'chrome-webgpu', commit, rows };
  await fs.writeFile(path.join(output, 'interface-status.json'), JSON.stringify(record, null, 2) + '\n');
  return record;
}
