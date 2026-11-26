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
    for (const dpr of [1, 2]) {
      const [row, marks] = await traceRoutes(cdp, options.timeout,
        () => arm(options, call, sessionId, dpr));
      row.routeCpu = routeCpu(row.final, row.routeSamples, marks);
      rows.push(row);
    }
  } finally {
    await cdp.call('Target.closeTarget', { targetId });
  }
  const record = { case: 'interface-modal', target: 'chrome-webgpu', commit, release: true, rows };
  await write(path.join(output, 'interface-modal.json'), record);
  return { case: record.case, target: record.target, commit, runs: rows.length };
}

async function traceRoutes(cdp, timeout, run) {
  const previousEvents = cdp.event;
  const marks = [];
  let complete;
  cdp.event = packet => {
    previousEvents(packet);
    if (packet.method === 'Tracing.dataCollected') {
      for (const event of packet.params.value) {
        if (event.name?.startsWith('thrum-menu-route-')) marks.push(event);
      }
    } else if (packet.method === 'Tracing.tracingComplete') {
      complete = packet.params;
    }
  };
  try {
    await cdp.call('Tracing.start', { categories: 'blink.user_timing' });
    let result;
    try {
      result = await run();
    } finally {
      await cdp.call('Tracing.end');
      const deadline = Date.now() + timeout;
      while (!complete) {
        if (Date.now() >= deadline) throw new Error('Chrome route trace did not complete');
        await sleep(20);
      }
    }
    assert(!complete.dataLossOccurred, 'Chrome route trace lost events');
    return [result, marks];
  } finally {
    cdp.event = previousEvents;
  }
}

function routeCpu(report, samples, marks) {
  const byName = new Map(marks.map(mark => [mark.name, mark]));
  assert.equal(byName.size, marks.length, 'duplicate route marks');
  assert.equal(marks.length, 3 * Object.keys(samples).length);
  const byKind = {};
  for (const kind of ['key press', 'pointer press', 'touch press', 'presentation edit']) {
    const { event_id, count } = samples[kind];
    const row = report.timing.find(row => row.id === event_id);
    assert(row?.kind === kind && row.frames >= 1, `routing sample did not come from submitted ${kind}`);
    assert(Number.isSafeInteger(count) && count > 0);
    const [empty, start, end] = ['empty', 'start', 'end'].map(phase =>
      byName.get(`thrum-menu-route-${event_id}-${phase}`));
    assert(empty && start && end, `missing route marks for ${row.id}`);
    for (const mark of [empty, start, end]) {
      assert.equal(mark.ph, 'I');
      assert.equal(mark.pid, start.pid);
      assert.equal(mark.tid, start.tid);
      assert(Number.isSafeInteger(mark.tts), 'thread clock missing');
    }
    const cpu_us = end.tts - start.tts;
    const empty_us = start.tts - empty.tts;
    assert(cpu_us >= 0 && empty_us >= 0, 'nonmonotonic renderer clock');
    byKind[kind] = { event_id, count, pid: start.pid, tid: start.tid,
      thread_ticks_us: [empty.tts, start.tts, end.tts], cpu_us, empty_us,
      net_us_per_route: (cpu_us - empty_us) / count };
  }
  return { clock: 'Chromium trace tts, renderer thread CPU microseconds',
    scope: 'Batches replay the actual browser event against independent copies of its pre-event router state. The control loop measures event copying and traversal.',
    byKind };
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
    return until(state => state.drawn === 'Choice' && !state.drawn_disarmed && state.boxes.length === 4, 'choice armed');
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
  const focused = await until(state => state.drawn_cards[1] === 'Focused', 'arrow focused Guard');
  assertRetained(opened.menu_work, focused.menu_work);
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
    if (id === 110) {
      const [left, top] = await canvas();
      // A pointer can focus a card while a finger is on the scrim.
      const idle = { x: Math.round(left + 24), y: Math.round(top + 24), id: 9 };
      await touch('touchStart', idle);
      const guard = await at(120, state);
      await mouse('mouseMoved', guard);
      await until(state => state.drawn_cards[1] === 'Focused', 'Guard focused while scrim is held');
      await touch('touchMove', { ...idle, x: idle.x + 12 });
      await until(state => state.drawn_cards.every(card => card === 'Idle'), 'touch motion hid focus');
      await mouse('mouseMoved', [guard[0] + 2, guard[1]]);
      await until(state => state.drawn_cards[1] === 'Focused', 'pointer restored focus');
      await touch('touchEnd');
      const touchIdle = await until(state => state.drawn_cards.every(card => card === 'Idle'),
        'touch release hid focus');
      assert.equal(touchIdle.focus, 120);
      await capture('choice-idle', touchIdle);
    }
    await mouse('mouseMoved', point);
    const focusedCard = await until(state => state.focus === id &&
      state.drawn_cards[cards.indexOf(id)] === 'Focused', 'mouse focused card');
    if (id === 120) await capture('choice-focused', focusedCard);
    const beforePress = focusedCard.timing.length;
    await mouse('mousePressed', point, { button: 'left', buttons: 1, clickCount: 1 });
    const pressedCard = await until(state => state.timing.slice(beforePress).some(row => row.kind === 'pointer press' && row.frames), 'pressed card submitted');
    assertRetained(focusedCard.menu_work, pressedCard.menu_work);
    if (id === 120) await capture('choice-pressed', pressedCard);
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
  const beforeTouch = (await report()).timing.length;
  await touch('touchStart', finger);
  await until(state => state.timing.slice(beforeTouch).some(row => row.kind === 'touch press' && row.frames), 'touch press submitted');
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
  assert(!final.inputs.some(row => /Arrow|Enter|Pointer/.test(row.input)));
  assert.equal(final.inputs.filter(row => row.input.includes('Touch') && row.input.startsWith('Press')).length, 1);
  assert(holds[1].contacts.some(contact => /Touch\(\d+\).*Some\(\[/.test(contact)));
  assert(final.effects.some(row => row.effect.source === 'Link' && row.effect.text === ticket));
  assert(final.effects.some(row => row.effect.source === 'Paste' && row.effect.text === expired));
  for (const kind of ['key press', 'pointer press', 'touch press', 'presentation edit']) {
    assert(final.timing.some(row => row.kind === kind && row.frames >= 1), kind);
  }

  const routeSamples = await game.invoke('return JSON.parse(game.profile_menu_routing());');
  const devices = await releaseGameGraphics(cdp, sessionId, game);
  await unloadGamePage(cdp, sessionId, timeout);
  return { dpr, build, loadBefore, loadAfter: os.loadavg(), stills, holds, final, routeSamples,
    beforeRecovery: beforeRecovery.focus, afterRecovery: recovered.focus, devices };
}

function assertRetained(before, after) {
  assert.equal(after.constructions, before.constructions, 'style edit rebuilt the choice tree');
  assert.equal(after.layouts, before.layouts, 'style edit laid out the choice again');
  assert(after.lists > before.lists, 'style edit did not rebuild the paint list');
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
