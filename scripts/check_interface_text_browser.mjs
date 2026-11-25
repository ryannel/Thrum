// Chrome's real page, fetched font products and the engine's submitted-output tap.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import { installWasmReceipt, readWasmReceipt } from './wasm_receipt.mjs';

function sections(bytes, magic) {
  assert.equal(bytes.toString('ascii', 0, 8), magic);
  const result = new Map();
  for (let i = 0; i < bytes.readUInt32LE(12); i++) {
    const at = 64 + 32 * i;
    const start = Number(bytes.readBigUInt64LE(at + 8));
    const size = bytes.readUInt32LE(at + 4);
    const count = Number(bytes.readBigUInt64LE(at + 16));
    result.set(bytes.toString('ascii', at, at + 4), { size, count, bytes: bytes.subarray(start, start + size * count) });
  }
  return result;
}

async function loadFonts(origin) {
  const get = async relative => {
    const response = await fetch(new URL(relative, origin));
    assert(response.ok, `${relative} returned ${response.status}`);
    return Buffer.from(await response.arrayBuffer());
  };
  const head = await get('/head');
  assert.equal(head.toString('ascii', 0, 8), 'THRMHEAD');
  const generation = Number(head.readBigUInt64LE(16));
  const manifest = sections(await get('/cas/' + head.toString('hex', 24, 56)), 'THRMMANI');
  const { bytes, count, size } = manifest.get('ENTR');
  const names = manifest.get('STRS').bytes;
  const fonts = {};
  for (let index = 0; index < count; index++) {
    const row = bytes.subarray(index * size, (index + 1) * size);
    assert.equal(row[54], 5);
    const start = row.readUInt32LE(48);
    const name = names.toString('utf8', start, start + row.readUInt16LE(52));
    fonts[name] = sections(await get('/cas/' + row.toString('hex', 8, 40)), 'THRFONT1');
  }
  return { generation, fonts };
}

const decode = code => {
  const x = code / 255;
  return x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4;
};
const encode = linear => Math.round(255 * (linear <= 0.0031308
  ? linear * 12.92 : 1.055 * linear ** (1 / 2.4) - 0.055));

function painter(arm, { fonts }, name, pointSize, scale, origin) {
  const pixel = name === 'pixel';
  const font = fonts[name];
  const sizes = font.get('SIZE');
  const strike = Array.from({ length: sizes.count }, (_, index) => sizes.bytes.subarray(index * sizes.size))
    .find(size => size.readFloatLE(0) === pointSize && size.readUInt32LE(4) === (pixel ? 1 : scale));
  assert(strike, 'original scale metrics must be present');
  const table = font.get('GLYP');
  const kerning = font.get('KERN');
  const glyphs = new Map();
  const pairs = new Map();
  for (let i = 0; i < strike.readUInt32LE(28); i++) {
    const at = (strike.readUInt32LE(24) + i) * table.size;
    glyphs.set(String.fromCodePoint(table.bytes.readUInt32LE(at)), {
      frame: table.bytes.readUInt32LE(at + 4), advance: table.bytes.readInt32LE(at + 12),
    });
  }
  for (let i = 0; i < strike.readUInt32LE(36); i++) {
    const at = (strike.readUInt32LE(32) + i) * kerning.size;
    const pair = String.fromCodePoint(kerning.bytes.readUInt32LE(at), kerning.bytes.readUInt32LE(at + 4));
    pairs.set(pair, kerning.bytes.readInt32LE(at + 8));
  }
  const sheet = sections(font.get('SHT' + (pixel ? 1 : scale)).bytes, 'THRMSHET');
  const frames = sheet.get('FRAM');
  const pages = sheet.get('PAGE').bytes;
  const pageWidth = pages.readUInt32LE(20);
  const pageHeight = pages.readUInt32LE(24);
  const texels = pages.subarray(Number(pages.readBigUInt64LE(80)));
  const block = pixel ? scale : 1;

  return (text, y) => {
    let pen = origin * scale;
    let previous = '';
    const baseline = y * scale + Math.ceil(strike.readFloatLE(8)) * block;
    for (const ch of text) {
      pen += (pairs.get(previous + ch) ?? 0) * block;
      const { frame, advance } = glyphs.get(ch);
      const row = frames.bytes.subarray(frame * frames.size);
      const [page, sourceX, sourceY, width, height] = [8, 12, 16, 20, 24].map(at => row.readUInt32LE(at));
      const left = pen + (row.readUInt32LE(28) - row.readFloatLE(44)) * block;
      const top = baseline + (row.readUInt32LE(32) - row.readFloatLE(48)) * block;
      for (let sy = 0; sy < height; sy++) {
        for (let sx = 0; sx < width; sx++) {
          const at = 4 * ((page * pageHeight + sourceY + sy) * pageWidth + sourceX + sx);
          const alpha = texels[at + 3] / 255;
          for (let dy = 0; dy < block; dy++) {
            for (let dx = 0; dx < block; dx++) {
              const x = left + sx * block + dx, y = top + sy * block + dy;
              assert(x >= 0 && x < arm.width && y >= 0 && y < arm.height);
              const offset = 4 * (y * arm.width + x);
              for (let channel = 0; channel < 3; channel++) {
                arm.rgba[offset + channel] = encode(decode(texels[at + channel])
                  + (1 - alpha) * decode(arm.rgba[offset + channel]));
              }
            }
          }
        }
      }
      pen += advance * block;
      previous = ch;
    }
  };
}

function reference(fonts, scale, tick, edited) {
  const width = 128 * scale, height = 80 * scale;
  const rgba = Buffer.alloc(width * height * 4);
  for (let offset = 3; offset < rgba.length; offset += 4) rgba[offset] = 255;
  const arm = { width, height, rgba };
  const digit = Math.floor(tick / 4) % 10;
  painter(arm, fonts, 'type', 12, scale, 12)('AV To r. Éž', 12);
  const grid = painter(arm, fonts, 'pixel', 8, scale, 12);
  // The two A's fill the first line by advance in the edited size. The middle
  // elision has room for one more prefix glyph with the original advances.
  grid('AA', 40);
  grid(digit + 'B', 48);
  grid(edited ? 'A…B' : 'AB…B', 65);
  return arm;
}

function compare(frame, width, reference) {
  for (let y = 0; y < reference.height; y++) {
    for (let x = 0; x < reference.width; x++) {
      for (let channel = 0; channel < 4; channel++) {
        const actual = frame[4 * (y * width + x) + channel];
        const expected = reference.rgba[4 * (y * reference.width + x) + channel];
        const tolerance = Number(channel < 3 && expected !== 0 && expected !== 255);
        if (Math.abs(actual - expected) > tolerance) return { x, y, channel, actual, expected };
      }
    }
  }
  return null;
}

export async function qualifyInterfaceText({ cdp, origin, fontOrigin, source, output, timeout, wasm, existingGame, waitFor, commit, visual }) {
  const pixelFile = path.join(source, 'pixel', 'font.json');
  const original = await fs.readFile(pixelFile);
  const changed = JSON.parse(original);
  for (const cell of changed.characters) {
    if (cell.character === 'A' || cell.character === 'B') cell.advance = 12;
  }
  const save = bytes => fs.writeFile(pixelFile, bytes);
  const awaitFonts = (after = 0) => waitFor(async () => {
    const font = await loadFonts(fontOrigin);
    return font.generation > after ? font : undefined;
  }, timeout, 'font publication');

  const arms = [];
  let before = await awaitFonts();
  for (const dpr of visual ? [1, 2] : [1, 2, 3]) {
    const [width, height] = (visual ? [520, 416] : [144, 96]).map(side => side * dpr);
    const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
    const call = (method, args = {}) => cdp.call(method, args, sessionId);
    try {
      await call('Page.enable');
      await call('Runtime.enable');
      await installWasmReceipt(cdp, sessionId);
      await call('Emulation.setDeviceMetricsOverride', {
        width: width / dpr, height: height / dpr, deviceScaleFactor: dpr, mobile: false,
      });
      await call('Page.addScriptToEvaluateOnNewDocument', { source: `{
        const frames = new Map();
        let serial = 0, time = 0;
        requestAnimationFrame = callback => { frames.set(++serial, callback); return serial; };
        cancelAnimationFrame = id => frames.delete(id);
        window.stepFontFrame = () => {
          const work = [...frames.values()];
          frames.clear();
          time += 18;
          for (const callback of work) callback(time);
          return work.length;
        };
      }` });
      await cdp.call('Target.activateTarget', { targetId });
      const card = visual ? 'type' : 'specimen';
      await call('Page.navigate', { url: `${origin}/?scene=interface&rows=1000&text-card=${card}&backing=${width}x${height}&assets=${encodeURIComponent(fontOrigin)}` });
      await waitFor(async () => {
        const { result } = await call('Runtime.evaluate', { expression: 'document.querySelector("#status")?.textContent', returnByValue: true });
        if (/fault:/i.test(result.value ?? '')) throw new Error(result.value);
        return result.value?.startsWith('Running');
      }, timeout, 'cooked font card opened');
      const game = await existingGame(cdp, sessionId);
      const build = await readWasmReceipt(cdp, sessionId, `${origin}/v1_game_bg.wasm`, wasm.sha256, wasm.bytes.length);
      await game.invoke('game.begin_fixed_submission(args[0]);', [dpr]);
      let previousSubmission = null;
      const frame = async () => {
        assert.equal(await game.invoke('return window.stepFontFrame();'), 1);
        const metadata = await waitFor(() => game.invoke(`
          const take = game.next_submitted_image();
          if (!take) return null;
          window.fontImage = take.pixels;
          return JSON.parse(take.metadata);
        `), timeout, 'engine-submitted font capture');
        assert(metadata.surface_presented && metadata.pixel_source === 'Sdr', JSON.stringify(metadata));
        assert.equal(metadata.paint_stamp_mismatch, false, JSON.stringify(metadata));
        if (previousSubmission !== null) assert.equal(metadata.submission_id, previousSubmission + 1);
        previousSubmission = metadata.submission_id;
        return { metadata };
      };
      const pixels = async take => {
        const size = await game.invoke('return window.fontImage.length;');
        assert.equal(size, width * height * 4);
        const chunks = [];
        for (let offset = 0; offset < size; offset += 256 * 1024) {
          const base64 = await game.invoke('return window.fontImage.subarray(args[0], args[1]).toBase64();',
            [offset, offset + 256 * 1024]);
          chunks.push(Buffer.from(base64, 'base64'));
        }
        await game.invoke('delete window.fontImage;');
        return { ...take, rgba: Buffer.concat(chunks) };
      };
      let initial;
      for (let step = 0; step < 200 && !initial; step++) {
        const take = await frame();
        if (take.metadata.paint_generation === before.generation) initial = await pixels(take);
      }
      assert(initial, 'initial font became resident');
      assert.equal(initial.metadata.paint_scale, dpr);
      const keep = async (take, label) => {
        const filename = `${label}-${dpr}.rgba`;
        await fs.writeFile(path.join(output, filename), take.rgba);
        return { ...take.metadata, filename, sha256: createHash('sha256').update(take.rgba).digest('hex') };
      };
      if (visual) {
        arms.push({ dpr, width, height, build, fixed: await keep(initial, 'type') });
        continue;
      }
      const initialExpected = reference(before, dpr, initial.metadata.source_tick, false);
      assert.equal(compare(initial.rgba, width, initialExpected), null, JSON.stringify(compare(initial.rgba, width, initialExpected)));
      const first = await keep(initial, 'old');
      await save(JSON.stringify(changed));
      const after = await awaitFonts(before.generation);
      const frames = [];
      let last;
      let switched = false;
      for (let step = 0; step < 220 && (!switched || step < switched + 3); step++) {
        const take = await pixels(await frame());
        const tick = take.metadata.source_tick;
        const oldDiff = compare(take.rgba, width, reference(before, dpr, tick, false));
        const newDiff = compare(take.rgba, width, reference(after, dpr, tick, true));
        const old = take.metadata.paint_generation === before.generation;
        assert.equal(take.metadata.paint_scale, dpr);
        assert.equal(old ? oldDiff : newDiff, null, JSON.stringify({ ...take.metadata, oldDiff, newDiff }));
        assert.notEqual(old ? newDiff : oldDiff, null, 'both references must be distinguishable');
        if (!old) {
          assert.equal(take.metadata.paint_generation, after.generation);
          if (switched === false) switched = step + 1;
          last = take;
        } else {
          assert.equal(switched, false, 'old font returned after the switch');
        }
        frames.push({ ...take.metadata, reference: old ? 'old' : 'new' });
      }
      assert(last, 'font edit reached the same page');
      assert(frames.some(frame => frame.reference === 'old') && frames.some(frame => frame.reference === 'new'));
      assert.equal(await game.invoke('return game.submitted_capture_omissions();'), 0);
      arms.push({ dpr, width, height, build, first, frames, last: await keep(last, 'new') });
      if (dpr < 3) {
        await save(original);
        before = await awaitFonts(after.generation);
      }
    } finally {
      await cdp.call('Target.closeTarget', { targetId });
    }
  }
  const result = { case: visual ? 'interface-text-visual' : 'interface-text', target: 'chrome-webgpu', commit, arms };
  await fs.writeFile(path.join(output, result.case + '.json'), JSON.stringify(result, null, 2) + '\n');
  return result;
}
