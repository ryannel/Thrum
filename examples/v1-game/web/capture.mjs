// A separate capture driver. The deterministic route page keeps its original refusals.
import init, { create_route_game, fetch_cooked_valley } from './v1_game.js';
const record = { done: false, complete: false, opportunities: [], images: [], actions: [], export_peak: 0 };
window.thrumCapture = record;
const query = new URL(location.href).searchParams;
const mode = query.get('mode');
const control = query.get('control');
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
let hidden = document.hidden;
document.addEventListener('visibilitychange', () => { hidden ||= document.hidden; });
let game;
const inflight = new Set();
let exportError;
let withholdUntil = 0;
const fail = message => { throw new Error(message); };
function drain() {
  if (control === 'off' || performance.now() < withholdUntil) return;
  for (;;) {
    const image = game.next_submitted_image();
    if (!image) return;
    const metadata = JSON.parse(image.metadata);
    metadata.available_ms = performance.now();
    if (inflight.size >= 8) {
      record.export_omissions = (record.export_omissions ?? 0) + 1;
      continue;
    }
    record.images.push(metadata);
    const task = fetch(`/capture?ordinal=${record.images.length - 1}`, {
      signal: AbortSignal.timeout(10000), method: 'POST', headers: { 'X-Frame': image.metadata }, body: image.pixels,
    }).then(response => { if (!response.ok) fail(`export HTTP ${response.status}`); })
      .catch(error => { exportError = String(error); }).finally(() => inflight.delete(task));
    inflight.add(task);
    record.export_peak = Math.max(record.export_peak, inflight.size);
  }
}
try {
  if (!['fixed', 'realtime'].includes(mode)) fail('mode must be fixed or realtime');
  if (hidden) fail('capture page started hidden');
  await init();
  const text = await (await fetch('/moving-v1.json')).text();
  const route = JSON.parse(text);
  game = await create_route_game(document.querySelector('canvas'), text);
  window.captureGame = game;
  game.attach_cooked_valley(await fetch_cooked_valley(query.get('assets')));
  game.set_visible(true, 0);
  // Fixed-time preparation stops exactly at the ordinary route installation barrier.
  while (game.committed_tick() < route.origin_tick - 1) {
    if (!game.route_slot_ready()) { await wait(1); continue; }
    game.frame(game.route_step_ms(0.5));
    await wait(1);
  }
  game.install_route(text);
  const base = game.route_step_ms(0.5) - 1000 / 60;
  game.set_paused(true, base);
  record.initial_report = JSON.parse(game.route_report(0));
  record.start_before_ms = performance.now();
  if (control !== 'off') game.begin_submitted_capture(1);
  record.start_after_ms = performance.now();
  record.base_world_ms = base;
  record.mode = mode;
  record.extent = route.render_extent;
  record.ui = { included: 'canvas scene and engine route overlay', excluded: 'DOM, browser and native UI' };
  record.device_pixel_ratio = devicePixelRatio;
  record.scheduling = mode === 'fixed' ? 'visible headless page, synthetic 60 Hz samples with explicit GPU/export waits' : 'visible headless page, timer opportunities at 60 Hz, ordinary lifecycle policy';
  if (control === 'loss') {
    game.set_paused(false, base);
    game.frame(base + 1000 / 60);
    record.waiting = 'destroy-pending';
    const deadline = performance.now() + 10000;
    while (!game.graphics_fault()) {
      if (performance.now() > deadline) fail('device loss was not delivered');
      await wait(4);
    }
    const refused = body => { try { body(); return ''; } catch (e) { return String(e); } };
    record.loss = { tick_before: game.committed_tick(),
      pending_refusal: refused(() => game.next_submitted_image()),
      begin_refusal: refused(() => game.begin_submitted_capture(2)) };
    let time = base + 1000 / 60;
    for (let i = 0; i < 12; i++) { time += 1000 / 60; game.frame(time); await wait(4); }
    record.loss.tick_after = game.committed_tick();
    game.request_graphics_recovery();
    while (game.graphics_fault() || game.recovering()) {
      if (performance.now() > deadline) fail('device recovery timed out');
      time += 1000 / 60; game.frame(time); await wait(4);
    }
    record.loss.retired_refusal = refused(() => game.next_submitted_image());
    game.end_submitted_capture();
    game.begin_submitted_capture(2);
    time += 1000 / 60; game.frame(time);
    let fresh;
    while (!(fresh = game.next_submitted_image())) {
      if (performance.now() > deadline) fail('replacement capture timed out');
      await wait(4);
    }
    record.loss.replacement = JSON.parse(fresh.metadata);
    record.loss.replacement_bytes = fresh.pixels.length;
    const saved = await fetch('/capture?ordinal=0', { method: 'POST', headers: { 'X-Frame': fresh.metadata }, body: fresh.pixels, signal: AbortSignal.timeout(10000) });
    if (!saved.ok || fresh.pixels.length !== route.render_extent[0] * route.render_extent[1] * 4) fail('replacement pixels failed export');
    record.images.push(record.loss.replacement);
    if (!/lost/.test(record.loss.pending_refusal) || !/lost/.test(record.loss.begin_refusal)
        || !record.loss.retired_refusal || record.loss.tick_after <= record.loss.tick_before
        || record.loss.replacement.device_generation < 2 || record.loss.replacement.session !== 2) fail('loss/recovery control failed');
    record.control_pass = true;
    throw new Error('expected incomplete interval after pending-readback device loss');
  }
  const start = performance.now();
  if (control === 'saturation') {
    withholdUntil = start + 2000;
    record.actions.push({ kind: 'withhold readback collection', start_ms: start, end_ms: withholdUntil });
  }
  const duration = (route.keys.at(-1).tick + 121) * 1000 / 60;
  let ordinal = 0;
  let acted = false;
  let released = false;
  while (mode === 'fixed' ? ordinal < route.keys.at(-1).tick + 121 : performance.now() - start < duration || game.committed_tick() < route.origin_tick + route.keys.at(-1).tick + 59) {
    if (hidden) fail('page became hidden during capture');
    const due = start + ordinal * 1000 / 60;
    if (mode === 'fixed') {
      while (!game.route_slot_ready()) { drain(); await wait(1); }
      while (inflight.size > 0) { drain(); await wait(1); }
    } else if (performance.now() < due) { drain(); await wait(Math.min(4, due - performance.now())); continue; }
    const requested = performance.now();
    if (!acted && control === 'stall' && requested - start > 1500) {
      const action = { kind: 'page dispatch stall', start_ms: performance.now(), requested_ms: 250 };
      while (performance.now() - action.start_ms < 250) { /* intentional diagnostic stall */ }
      action.end_ms = performance.now(); record.actions.push(action); acted = true;
    }
    if (!acted && control === 'resize' && requested - start > 1500) {
      const canvas = document.querySelector('canvas');
      canvas.width = 961; canvas.height = 541;
      record.actions.push({ kind: 'surface resize', requested_ms: performance.now(), extent: [961, 541] });
      acted = true;
    }
    const timestamp = mode === 'fixed'  ? base + (ordinal + 1) * 1000 / 60 : base + performance.now() - start;
    if (!released && timestamp - base >= 1000) {
      game.set_paused(false, timestamp);
      record.hold_release = { page_ms: performance.now(), frame_ms: timestamp };
      released = true;
    }
    if (performance.now() - start > 20000) fail('scenario deadline');
    game.frame(timestamp);
    record.opportunities.push({ ordinal, requested_ms: requested, frame_ms: timestamp, returned_ms: performance.now(), tick: game.committed_tick() });
    ordinal++;
    if (mode === 'realtime') ordinal = Math.max(ordinal, Math.floor((performance.now() - start) * 60 / 1000));
    if (control !== 'saturation' || performance.now() - start > 2000) drain();
    await wait(mode === 'fixed' ? 1 : 0);
  }
  const drainStart = performance.now();
  for (;;) {
    drain();
    if (control === 'off' || game.submitted_capture_pending() === 0) break;
    if (performance.now() - drainStart > 5000) fail('capture finalization deadline');
    await wait(4);
  }
  await Promise.all(inflight);
  record.renderer_omissions = game.submitted_capture_omissions();
  record.end_ms = performance.now();
  record.final_report = JSON.parse(game.route_report(0));
  if (record.renderer_omissions || record.export_omissions || exportError) fail(`incomplete capture: renderer=${record.renderer_omissions}, export=${record.export_omissions ?? 0}, error=${exportError}`);
  record.complete = true;
} catch (error) { record.error = String(error); }
finally {
  game?.end_submitted_capture();
  record.done = true;
  document.querySelector('#status').textContent = JSON.stringify({ complete: record.complete, error: record.error });
}
