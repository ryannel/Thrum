// Qualification of the same submitted-output take that is retained for inspection.
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';

const colours = [[25, 183, 238], [240, 114, 31]];

function paletteCounts(rgba) {
  const hits = [0, 0];
  for (let i = 0; i < rgba.length; i += 4) {
    const red = rgba[i], green = rgba[i + 1], blue = rgba[i + 2];
    if (Math.abs(red - colours[0][0]) <= 2 && Math.abs(green - colours[0][1]) <= 2 && Math.abs(blue - colours[0][2]) <= 2) hits[0]++;
    if (Math.abs(red - colours[1][0]) <= 2 && Math.abs(green - colours[1][1]) <= 2 && Math.abs(blue - colours[1][2]) <= 2) hits[1]++;
  }
  return hits;
}

export function checkSheetFrame(raw, frame) {
  assert([1, 2].includes(frame.sheet_generation), 'one of the two cooked generations must draw');
  assert.equal(frame.sprite_world_rows, 500, 'every cooked row draws');
  assert.equal(frame.pending_sheet_rows, 0);
  assert.equal(frame.missing_frame_rows, 0);
  const hits = paletteCounts(raw);
  const active = frame.sheet_generation - 1;
  assert(hits[active] > 64 && hits[1 - active] === 0,
    `submission ${frame.submission_id} shows wrong, missing or mixed art: ${hits}`);
  frame.sheet_palette_pixels = hits;
}

async function products(output, head) {
  const bytes = await fs.readFile(path.join(output, 'store', 'cas', head.root.slice(0, 2), head.root));
  assert.equal(bytes.subarray(0, 8).toString(), 'THRMMANI');
  const sections = new Map();
  for (let i = 0; i < bytes.readUInt32LE(12); i++) {
    const at = 64 + i * 32;
    sections.set(bytes.subarray(at, at + 4).toString(), {
      start: Number(bytes.readBigUInt64LE(at + 8)), count: Number(bytes.readBigUInt64LE(at + 16)),
    });
  }
  const { start, count } = sections.get('ENTR');
  const names = sections.get('STRS').start;
  const result = {};
  for (let index = 0; index < count; index++) {
    const at = start + index * 64;
    if (bytes[at + 55] !== 0) continue;
    assert.equal(bytes[at + 54], 4, 'portable sprite sheet kind');
    const name = names + bytes.readUInt32LE(at + 48);
    result[bytes.subarray(name, name + bytes.readUInt16LE(at + 52)).toString()] = bytes.subarray(at + 8, at + 40).toString('hex');
  }
  return result;
}

export async function checkSheetSwitch(output, record, frames, initial, final, requests) {
  const requested = record.actions.find(action => action.kind === 'sheet-save');
  const fetched = record.actions.find(action => action.kind === 'sheet-fetched');
  assert.equal(requested.kind, 'sheet-save');
  assert.equal(fetched.kind, 'sheet-fetched');
  const switches = frames.filter((row, i) => i > 0 && row.sheet_generation !== frames[i - 1].sheet_generation);
  assert.equal(switches.length, 1, 'one submitted publication switch');
  const switched = switches[0];
  const index = frames.indexOf(switched);
  assert.equal(frames[0].sheet_generation, 1);
  assert.equal(switched.sheet_generation, 2);
  assert(index >= 8 && frames.length - index >= 8, 'frames on both sides of the switch');
  const before = frames[index - 1];
  const save = requested.requested_ms - record.start_after_ms;
  assert(switched.submit_ms > save, 'edited art cannot precede the save');
  assert(switched.tick >= before.tick && frames[index + 2].tick > frames[index - 2].tick, 'world advances across the switch');
  assert.notDeepEqual(frames[index - 2].sprite_view_centre, frames[index + 2].sprite_view_centre);
  assert.equal(initial.generation, 1);
  assert.equal(final.generation, 2);
  const oldProducts = await products(output, initial);
  const newProducts = await products(output, final);
  assert.deepEqual(Object.keys(newProducts).sort(), ['field', 'unrelated']);
  assert.equal(oldProducts.unrelated, newProducts.unrelated);
  assert.notEqual(oldProducts.field, newProducts.field);
  assert.deepEqual(requests.map(row => row.hash).sort(),
    [initial.root, final.root, oldProducts.field, oldProducts.unrelated, newProducts.field].sort(),
    'Chrome fetched each initial product once and only the changed sheet after the save');
  return { initial, final, oldProducts, newProducts, requests, save_ms: save,
    fetch_settled_ms: fetched.at_ms - record.start_after_ms, submission: switched.submission_id,
    tick: switched.tick, switch_ms: switched.submit_ms, preceding_gap_ms: switched.submit_ms - before.submit_ms };
}
