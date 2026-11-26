// Qualify the live WebGPU page through Chrome's actual keyboard, mouse and touch dispatch.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { installWasmReceipt, readWasmReceipt } from './wasm_receipt.mjs';

const cards = [110, 120, 130, 140];
const ticket = 'THRUM-ALPHA-29406-A82EXACL79FTG2OXQ4X65SPPDLX2OUF6KR18PJH5902C01W64NFQVE82NHXQ33DM0-COAST-8432';
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const write = (file, data) => fs.writeFile(file, JSON.stringify(data, null, 2) + '\n');

export async function qualifyInterfaceModal(options) {
  const { cdp, output, commit } = options;
  const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
  const rows = [];
  try {
    const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
    const call = (method, args = {}) => cdp.call(method, args, sessionId);
    await call('Page.enable');
    await call('Runtime.enable');
    await installWasmReceipt(cdp, sessionId);
    await cdp.call('Target.activateTarget', { targetId });
    for (const dpr of [1, 2]) rows.push(await arm(options, call, sessionId, dpr));
  } finally {
    await cdp.call('Target.closeTarget', { targetId });
  }
  const record = { case: 'interface-modal', target: 'chrome-webgpu', commit, release: true, rows };
  await write(path.join(output, 'interface-modal.json'), record);
  return { case: record.case, target: record.target, commit, runs: rows.length };
}

async function arm(opts, call, sessionId, dpr) {
  const { cdp, origin, fontOrigin, output, timeout, wasm, existingGame, waitFor,
    destroyActualDevice, releaseGameGraphics, unloadGamePage, encodePng } = opts;
  const loadBefore = os.loadavg();
  await call('Emulation.setDeviceMetricsOverride', { width: 1100, height: 1000, deviceScaleFactor: dpr, mobile: false });
  await call('Page.navigate', { url: `${origin}/?scene=interface&text-card=menu&menu-record=1&assets=${encodeURIComponent(fontOrigin)}` });
  await waitFor(async () => {
    const { result } = await call('Runtime.evaluate', { expression: 'document.querySelector("#status")?.textContent', returnByValue: true });
    if (/fault:/i.test(result.value ?? '')) throw new Error(result.value);
    return result.value?.startsWith('Running');
  }, timeout, 'menu page startup');
  const game = await existingGame(cdp, sessionId);
  const build = await readWasmReceipt(cdp, sessionId, `${origin}/v1_game_bg.wasm`, wasm.sha256, wasm.bytes.length);
  await game.invoke('document.querySelector("canvas").style.cssText += ";width:680px;height:400px";');
  const report = () => game.invoke('return JSON.parse(game.menu_report());');
  const until = async (predicate, name) => {
    let state;
    try {
      return await waitFor(async () => {
        state = await report();
        return predicate(state) && state;
      }, Math.min(timeout, 15000), name);
    } catch (error) {
      await write(path.join(output, `menu-${dpr}-failed.json`), { name, state });
      throw error;
    }
  };
  await until(state => state.drawn === 'Play', 'menu resources submitted');

  const effect = (key, state) => state.effects.filter(row => Object.hasOwn(row.effect, key)).map(row => row.effect[key]);
  const code = { KeyD: ['d', 68], KeyE: ['e', 69], KeyM: ['m', 77], Enter: ['Enter', 13], Escape: ['Escape', 27],
    ArrowRight: ['ArrowRight', 39], ArrowDown: ['ArrowDown', 40] };
  const key = (name, down) => call('Input.dispatchKeyEvent', { type: down ? 'keyDown' : 'keyUp',
    code: name, key: code[name][0], windowsVirtualKeyCode: code[name][1] });
  const tap = async name => { await key(name, true); await key(name, false); };
  const choice = async () => {
    await tap('KeyM');
    return until(state => state.drawn === 'Choice' && !state.disarmed && state.boxes.length === 4, 'choice armed');
  };
  const canvas = () => game.invoke(`const c = document.querySelector('canvas'), rect = c.getBoundingClientRect();
    return [rect.left, rect.top, rect.width / c.width, rect.height / c.height];`);
  const at = async (id, state) => {
    const [left, top, sx, sy] = await canvas();
    const [, origin, size] = state.boxes.find(box => box[0] === id);
    return [Math.round(left + (origin[0] + size[0] / 2) * state.scale * sx),
      Math.round(top + (origin[1] + size[1] / 2) * state.scale * sy)];
  };
  const mouse = (type, [x, y], extra = {}) => call('Input.dispatchMouseEvent', { type, x, y, ...extra });
  const touch = (type, ...touchPoints) => call('Input.dispatchTouchEvent', { type, touchPoints });
  const play = () => until(state => state.drawn === 'Play', 'choice closed');
  const holds = [];
  const stills = [];

  await tap('KeyE');
  await until(state => state.timing.some(row => row.kind === 'presentation edit' && row.frames), 'game edit drawn');
  const start = await report();
  await key('KeyD', true);
  await until(state => state.position[0] > start.position[0], 'key moved player');
  const opened = await choice();
  await sleep(130);
  assert.deepEqual((await report()).position, opened.position);
  await tap('ArrowRight');
  const focused = await until(state => state.focus === 120, 'arrow focused Guard');
  await tap('Enter');
  await play();
  const resumed = await until(state => state.position[0] > opened.position[0], 'held key resumed');
  await key('KeyD', false);
  holds.push({ device: 'keyboard', opened, focused, resumed: resumed.position });

  for (const [index, id] of cards.entries()) {
    await choice();
    if (index & 1) await tap('ArrowRight');
    if (index & 2) await tap('ArrowDown');
    assert.equal((await report()).focus, id);
    await tap('Enter');
    await play();
  }

  let session = 100 * dpr;
  const capture = async (name, state) => {
    const filename = `menu-${dpr}-${name}.png`;
    const metadata = await takeFixed(game, waitFor, timeout, ++session, path.join(output, filename), encodePng);
    assert.equal(metadata.paint_scale, dpr);
    stills.push({ filename, metadata, state });
  };
  for (const id of cards) {
    const state = await choice();
    const point = await at(id, state);
    if (id === 110) await capture('choice-open', state);
    await mouse('mouseMoved', point);
    await until(state => state.focus === id, 'mouse focused card');
    if (id === 120) await capture('choice-focused', await report());
    await mouse('mousePressed', point, { button: 'left', buttons: 1, clickCount: 1 });
    await until(state => state.timing.some(row => row.kind === 'pointer press' && row.frames), 'pressed card submitted');
    if (id === 120) await capture('choice-pressed', await report());
    await mouse('mouseReleased', point, { button: 'left', buttons: 0, clickCount: 1 });
    await play();
  }

  const [left, top] = await canvas();
  const stick = { x: Math.round(left + 80), y: Math.round(top + 320), id: 1 };
  await touch('touchStart', stick);
  const stopped = await choice();
  await sleep(130);
  assert.deepEqual((await report()).position, stopped.position);
  const [x, y] = await at(140, stopped);
  const finger = { x, y, id: 2 };
  await touch('touchStart', finger);
  await until(state => state.timing.some(row => row.kind === 'touch press' && row.frames), 'touch press submitted');
  // Chromium's native DevTools dispatch releases the IDs in a nonempty touchEnd.
  await touch('touchEnd', finger);
  await play();
  const touchResumed = await until(state => state.position[0] > stopped.position[0], 'first touch resumed');
  await touch('touchEnd');
  holds.push({ device: 'touch', stopped: stopped.position, resumed: touchResumed.position,
    contacts: effect('resumed', touchResumed).at(-1) });

  for (const id of cards.slice(0, 3)) {
    const state = await choice();
    const [x, y] = await at(id, state);
    await touch('touchStart', { x, y, id: 2 });
    await touch('touchEnd');
    await play();
  }

  await choice();
  await tap('ArrowRight');
  const beforeRecovery = await until(state => state.focus === 120, 'focus before replacement');
  await destroyActualDevice(cdp, sessionId, null, () => {});
  await waitFor(() => game.invoke('return game.graphics_fault();'), timeout, 'device reported lost');
  await game.invoke('game.request_graphics_recovery();');
  const recovered = await until(state => state.submitted > beforeRecovery.submitted && state.drawn === 'Choice', 'replacement submission');
  assert.equal(recovered.focus, beforeRecovery.focus);
  await tap('Escape');
  await play();

  await game.invoke('location.hash = args[0];', [ticket]);
  const joinedTicket = await until(state => state.drawn === 'Joining' && state.ticket === ticket && state.boxes.some(box => box[0] === 220), 'fragment delivered whole');
  await capture('joining', joinedTicket);
  await tap('Enter');
  const joined = await until(state => state.drawn === 'Joined', 'joined roster drawn');
  await capture('joined', joined);
  await tap('Escape');
  await play();
  const expired = `${ticket}-EXPIRED`;
  await game.invoke(`const data = new DataTransfer(); data.setData("text/plain", args[0]);
    window.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));`, [expired]);
  await until(state => state.drawn === 'Joining' && state.ticket === expired && state.boxes.some(box => box[0] === 220), 'paste delivered whole');
  await tap('Enter');
  const refused = await until(state => state.drawn === 'Refused', 'join refusal drawn');
  await capture('refused', refused);

  const final = await report();
  assert.deepEqual(final.activations, [3, 4, 3, 3]);
  assert(!final.inputs.some(row => /Arrow|Enter|Pointer|Touch\(\d+\).*\[.*\]/.test(row.input) && !row.input.includes('Touch')));
  assert.equal(final.inputs.filter(row => row.input.includes('Touch') && row.input.startsWith('Press')).length, 1);
  assert(holds[1].contacts.some(contact => /Touch\(\d+\).*Some\(\[/.test(contact)));
  assert(final.effects.some(row => row.effect.source === 'Link' && row.effect.text === ticket));
  assert(final.effects.some(row => row.effect.source === 'Paste' && row.effect.text === expired));
  for (const kind of ['key press', 'pointer press', 'touch press', 'presentation edit']) {
    assert(final.timing.some(row => row.kind === kind && row.frames >= 1 &&
      Number.isFinite(row.route_ms) && row.route_ms >= 0), kind);
  }

  const devices = await releaseGameGraphics(cdp, sessionId, game);
  await unloadGamePage(cdp, sessionId, timeout);
  return { dpr, build, loadBefore, loadAfter: os.loadavg(), stills, holds, final,
    beforeRecovery: beforeRecovery.focus, afterRecovery: recovered.focus, devices };
}

async function takeFixed(game, waitFor, timeout, session, filename, encodePng) {
  await game.invoke('game.set_paused(true, performance.now());game.begin_fixed_submission(args[0]);', [session]);
  try {
    const metadata = await waitFor(() => game.invoke(`
      const image = game.next_submitted_image();
      if (!image) return null;
      window.menuPixels = image.pixels;
      return JSON.parse(image.metadata);
    `), timeout, 'submitted menu capture');
    assert.equal(metadata.pixel_source, 'Sdr');
    assert.equal(metadata.surface_presented, true);
    const [width, height] = metadata.extent;
    const chunks = [];
    for (let i = 0; i < width * height * 4; i += 256 * 1024) {
      const base64 = await game.invoke('return window.menuPixels.subarray(args[0], args[1]).toBase64();', [i, i + 256 * 1024]);
      chunks.push(Buffer.from(base64, 'base64'));
    }
    const rgba = Buffer.concat(chunks);
    assert.equal(rgba.length, width * height * 4);
    await fs.writeFile(filename, encodePng(width, height, rgba));
    return { ...metadata, sha256: createHash('sha256').update(rgba).digest('hex') };
  } finally {
    await game.invoke('game.end_submitted_capture();delete window.menuPixels;game.set_paused(false, performance.now());');
  }
}
