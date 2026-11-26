// A separate capture driver. The deterministic route page keeps its original refusals.
import init, * as bindings from './v1_game.js';
const { create_route_game, create_sprite_game, create_sprite_probe, create_cooked_sprite_game, create_interface_text_game, fetch_cooked_valley } = bindings;
import { startCaptureUploadWorker, uploadCapture } from './capture_upload.mjs';
const record = { done: false, complete: false, opportunities: [], images: [], actions: [], export_peak: 0 };
window.thrumCapture = record;
const query = new URL(location.href).searchParams;
const mode = query.get('mode');
const control = query.get('control');
const source = query.get('source') ?? 'sdr';
const governed = query.get('governor') === 'on';
const scene = query.get('scene') ?? 'day';
const spriteScroll = scene === 'sprites-scroll';
const spriteSheets = scene === 'sprites-sheets';
const spriteStatus = scene === 'sprites-status';
const spriteField = scene === 'sprites-field' || spriteScroll || spriteSheets || spriteStatus;
const witness = query.get('witness') ?? 'off';
const atmosphere = query.get('atmosphere') ?? 'off';
const heldSetup = witness !== 'off' || atmosphere !== 'off';
const atmosphereMotion = atmosphere === 'scrub' || atmosphere === 'ground-pan';
const clouds = query.get('clouds') ?? 'off';
const cloudView = query.has('cloud-view');
const configuredExportSlots = Number(query.get('export-slots'));
const allowedExportSlots = atmosphereMotion || cloudView ? [24, 32, 64, 128] : [24, 32, 64];
const exportSlots = (governed || spriteField || scene === 'exit' || heldSetup || clouds !== 'off') && allowedExportSlots.includes(configuredExportSlots) ? configuredExportSlots : 8;
const useUploadWorker = mode === 'realtime' && control === 'ordinary' &&
  (governed || spriteField || scene === 'exit' || heldSetup || clouds !== 'off');
const useAnimationFrame = (governed || scene === 'exit' || atmosphereMotion || clouds !== 'off') && mode === 'realtime' &&
  (control === 'ordinary' || control === 'off');
if (!['day', 'night', 'noon', 'exit', 'sprites-field', 'sprites-scroll', 'sprites-sheets', 'sprites-status'].includes(scene)) throw new Error('Unknown route scene');
if (!['sdr', 'scene-linear'].includes(source)) throw new Error('Unknown capture source');
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
let hidden = document.hidden;
document.addEventListener('visibilitychange', () => { hidden ||= document.hidden; });
let game;
let hazeTake;
const inflight = new Set();
const reusablePixels = [];
let uploadWorker;
let exportError;
let withholdUntil = 0;
const fail = message => { throw new Error(message); };
function applyCaptureParameters(text, timestamp) {
  const result = JSON.parse(game.apply_parameters(text, timestamp));
  if (result.problems.length) fail(result.problems.join('; '));
}
function drain() {
  if (control === 'off' || performance.now() < withholdUntil) return;
  for (;;) {
    const pixels = reusablePixels.pop();
    const image = game.next_submitted_image(pixels);
    if (!image) {
      if (pixels) reusablePixels.push(pixels);
      return;
    }
    const metadata = JSON.parse(image.metadata);
    metadata.available_ms = performance.now();
    if (inflight.size >= exportSlots) {
      record.export_omissions = (record.export_omissions ?? 0) + 1;
      continue;
    }
    record.images.push(metadata);
    const ordinal = record.images.length - 1;
    const upload = uploadWorker ? uploadWorker.upload(ordinal, image).then(reply => {
      reusablePixels.push(reply.pixels);
      if (reply.retries.length) (record.export_retries ??= []).push(...reply.retries);
      if (reply.error) throw new Error(reply.error);
      if (spriteField) metadata.rgba_sha256 = reply.rgba_sha256;
    }) : uploadCapture(ordinal, image, retry => (record.export_retries ??= []).push(retry));
    const task = upload
      .catch(error => { exportError = String(error); }).finally(() => inflight.delete(task));
    inflight.add(task);
    record.export_peak = Math.max(record.export_peak, inflight.size);
  }
}
async function runCapture() {
  if (!['fixed', 'realtime'].includes(mode)) fail('mode must be fixed or realtime');
  if (hidden) fail('capture page started hidden');
  await init();
  if (spriteField) return captureSpriteField();
  let text = await (await fetch('/'+(query.get('route') ?? 'moving-v1.json'))).text();
  const route = JSON.parse(text);
  const view = cloudView ? await (await fetch('/capture-view.json')).json() : undefined;
  if (view) {
    route.render_extent = view.extent;
    text = JSON.stringify(route);
    record.cloud_view = view;
  }
  if (witness !== 'off') { route.overlay = 'reconstruction'; text = JSON.stringify(route); }
  if (scene === 'night' || scene === 'noon') {
    route.start.solar_hours = scene === 'night' ? 0 : 12;
    text = JSON.stringify(route);
  }
  if (query.get('overlay') === 'off') { route.overlay = 'off'; text = JSON.stringify(route); }
  record.effective_route = route;
  record.scene = scene;
  if (useUploadWorker) {
    uploadWorker = await startCaptureUploadWorker();
    const bytes = route.render_extent[0] * route.render_extent[1] * (source === 'scene-linear' ? 8 : 4);
    for (let index = 0; index < exportSlots; index++) reusablePixels.push(new Uint8Array(bytes));
  }
  record.diagnostic_clouds = query.get('diagnostic-clouds') === 'on';
  if (clouds !== 'off') record.clouds = clouds;
  game = await create_route_game(document.querySelector('canvas'), text, query.get('diagnostic-clouds') === 'on', query.get('culling') ?? undefined, query.get('meshes') === 'on', query.get('reconstruction') === 'taau', query.get('scene') === 'exit', clouds, query.get('haze') ?? undefined, view ? JSON.stringify({preset: view.preset, rate: view.weather_rate, wind: view.wind, placed_origin: query.get("cloud-store") ?? undefined, placement: view.placement}) : undefined);
  if (useAnimationFrame) game.capture_without_pass_timings();
  window.captureGame = game;
  const effect = query.get('effect') ?? 'off';
  if (!['off','slab','depth-only'].includes(effect)) fail('Unknown effect diagnostic');
  record.effect = effect;
  game.attach_cooked_valley(await fetch_cooked_valley(query.get('assets')), effect === 'off' ? undefined : effect === 'slab');
  game.set_visible(true, 0);
  if (view && !atmosphereMotion) {
    await captureCloudView(view, route.render_extent);
    return;
  }
  if (clouds !== 'off' && witness === 'off') {
    const key = route.keys[0];
    game.set_cloud_capture_eye(...key.eye);
    game.set_camera_yaw(key.yaw_degrees * Math.PI / 180);
    game.set_camera_pitch(key.pitch_degrees * Math.PI / 180);
  }
  if (!heldSetup) {
    // Fixed-time preparation stops at the ordinary route installation barrier.
    while (game.committed_tick() < route.origin_tick - 1) {
      if (!game.route_slot_ready()) { await wait(1); continue; }
      game.frame(game.route_step_ms(0.5));
      await wait(1);
    }
  } else {
    game.set_paused(true,0);
    const deadline = performance.now()+15000;
    while (JSON.parse(game.cooked_status()).drawn !== 1) {
      if (performance.now()>deadline) fail('reconstruction witness cooked residency deadline');
      if (game.route_slot_ready()) game.frame(0);
      await wait(1);
    }
    if (game.committed_tick() !== 0) fail('reconstruction witness advanced during residency');
  }
  if (!heldSetup) game.install_route(text);
  else if (atmosphere === 'off') {
    game.set_time_of_day(witness === 'stars' ? 0 : 0.5);
    if (witness === 'stars') game.set_camera_pitch(7*Math.PI/180);
  }
  if (governed) {
    game.route_render_scale(108);
    if (!game.enable_route_governor()) fail('the prepared capture route refused the governor');
  }
  const base = game.route_step_ms(0.5) - 1000 / 60;
  game.set_paused(true, base);
  if (atmosphere !== 'off') {
    const parameters = await (await fetch('/capture-params.json')).text();
    record.parameters = JSON.parse(parameters);
    applyCaptureParameters(parameters, base);
  }
  if (query.has('haze-take')) {
    hazeTake = await (await fetch('/haze-take.json')).text();
    record.haze_take = JSON.parse(hazeTake);
    game.configure_haze_take(hazeTake);
    game.sample_haze_take(0);
  }
  if (atmosphere === 'cost') game.enable_haze_timings();
  if (heldSetup) {
    // Match the native witness's held setup before recording elapsed motion.
    // Residency and the phase edit above must not become an AA startup transient.
    for (let frame = 0; frame < 128; frame++) {
      const deadline = performance.now()+15000;
      while (!game.route_slot_ready()) {
        if (performance.now()>deadline) fail('reconstruction witness warmup deadline');
        await wait(1);
      }
      game.frame(base);
      await wait(1);
    }
    if (game.committed_tick() !== 0) fail('reconstruction witness advanced during warmup');
    record.held_warmup_submissions = 128;
  }
  record.initial_report = JSON.parse(game.route_report(0));
  if (query.has('renderer-signature')) record.renderer_signature = JSON.parse(game.renderer_signature());
  record.culling = game.report();
  record.start_before_ms = performance.now();
  if (control !== 'off') {
    if (source === 'scene-linear') {
      const deadline = performance.now() + 15000;
      for (;;) {
        try { game.begin_linear_capture(1); break; }
        catch (error) {
          if (!String(error).includes('capture stage change requires retired frames') || performance.now() >= deadline) throw error;
          await wait(2);
        }
      }
    }
    else game.begin_submitted_capture(1);
  }
  record.pixel_source = source;
  record.start_after_ms = performance.now();
  record.base_world_ms = base;
  record.mode = mode;
  record.governor = governed;
  record.gpu_pass_timing = !useAnimationFrame;
  record.witness = witness;
  record.extent = route.render_extent;
  record.ui = { included: 'canvas scene and engine route overlay', excluded: 'DOM, browser and native UI' };
  record.device_pixel_ratio = devicePixelRatio;
  record.scheduling = useAnimationFrame ? 'visible headless page, browser animation-frame callbacks and elapsed time' :
    mode === 'fixed' ? 'visible headless page, synthetic 60 Hz samples with explicit GPU/export waits' :
    'visible headless page, timer opportunities at 60 Hz, ordinary lifecycle policy';
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
    await uploadCapture(0, fresh, retry => (record.export_retries ??= []).push(retry));
    if (fresh.pixels.length !== route.render_extent[0] * route.render_extent[1] * 4) fail('replacement pixels failed export');
    record.images.push(record.loss.replacement);
    if (!/lost/.test(record.loss.pending_refusal) || !/lost/.test(record.loss.begin_refusal)
        || !record.loss.retired_refusal || record.loss.tick_after <= record.loss.tick_before
        || record.loss.replacement.device_generation < 2 || record.loss.replacement.session !== 2) fail('loss/recovery control failed');
    record.control_pass = true;
    throw new Error('expected incomplete interval after pending-readback device loss');
  }
  const start = performance.now();
  if (useAnimationFrame) record.elapsed_origin_ms = start;
  if (control === 'saturation') {
    withholdUntil = start + 2000;
    record.actions.push({ kind: 'withhold readback collection', start_ms: start, end_ms: withholdUntil });
  }
  const bankMover = atmosphereMotion && record.haze_take?.motion === 'mover';
  const longGroundTake = ['climb', 'approach'].includes(record.haze_take?.motion);
  const throughHandover = atmosphereMotion && record.haze_take?.motion === 'throughhandover';
  const groundDuration = throughHandover ? 32000 : bankMover ? 13000 : longGroundTake ? 12000 : 7000;
  const duration = atmosphereMotion ? (atmosphere === 'scrub' ? 10000 : groundDuration) :
    (route.keys.at(-1).tick + 121) * 1000 / 60;
  const fixedExitPhaseMs = scene === 'exit' && mode === 'fixed'
    ? (route.phase[0] / route.phase[1]) * 1000 / 60 : 0;
  let ordinal = 0;
  let acted = false;
  let released = false;
  while (atmosphere === 'checkpoint' ? ordinal < 1 : atmosphere === 'cost' ? ordinal < 96 :
    atmosphereMotion ? performance.now() - start < duration :
    witness !== 'off' ? ordinal < 270 : mode === 'fixed' ? ordinal < route.keys.at(-1).tick + 121 :
    performance.now() - start < duration || game.committed_tick() < route.origin_tick + route.keys.at(-1).tick + 59) {
    if (hidden) fail('page became hidden during capture');
    const due = start + ordinal * 1000 / 60;
    let rafMs;
    if (mode === 'fixed') {
      while (!game.route_slot_ready()) { drain(); await wait(1); }
      while (inflight.size > 0) { drain(); await wait(1); }
    } else if (useAnimationFrame) {
      rafMs = await new Promise(requestAnimationFrame);
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
    if (witness === 'pan') game.set_camera_yaw(36*Math.PI/180*Math.max(0,Math.min(1,(timestamp-base-1000)/2000)));
    if (witness === 'scrub' && timestamp-base >= 1000) {
      const solar = 6+12*Math.max(0,Math.min(1,(timestamp-base-1000)/2000));
      game.set_time_of_day(solar/24);
      record.actions.push({kind:'solar-scrub',frame_ms:timestamp,solar_hours:solar});
    }
    if (atmosphereMotion && hazeTake) {
      const pose = JSON.parse(game.sample_haze_take(Math.max(0, (timestamp - base) / 1000)));
      record.actions.push({kind:'haze', frame_ms:timestamp, pose});
    } else if (atmosphereMotion) {
      const scrub = atmosphere === 'scrub';
      const progress = Math.max(0, Math.min(1, (timestamp-base-1000)/(scrub ? 8000 : 5000)));
      const solar = scrub ? 5+19*progress : 17.4+0.4*progress;
      const yaw = scrub ? -45+22.5*Math.sin(2*Math.PI*progress) : -110+40*progress;
      game.set_time_of_day(solar / 24);
      game.set_camera_yaw(yaw * Math.PI / 180);
      record.actions.push({kind:atmosphere,frame_ms:timestamp,solar_hours:game.time_of_day()*24,yaw_degrees:yaw});
    }
    if ((atmosphere === 'off' || bankMover) && !released && timestamp - base >= 1000 && !['stars','scrub'].includes(witness)) {
      // Resume clears the clock fraction. Put the fixed exit route's origin
      // between the last held callback and this sample to restore its phase.
      game.set_paused(false, timestamp - fixedExitPhaseMs);
      record.hold_release = { page_ms: performance.now(), frame_ms: timestamp };
      released = true;
    }
    if (performance.now() - start > (mode === 'fixed' ? 120000 : Math.max(20000, duration + 6000))) fail('scenario deadline');
    game.frame(timestamp);
    const opportunity = { ordinal, requested_ms: requested, frame_ms: timestamp, returned_ms: performance.now(), tick: game.committed_tick() };
    if ((governed || clouds !== 'off') && useAnimationFrame) opportunity.raf_ms = rafMs;
    if (control === 'off' && scene === 'exit') opportunity.presented = Number(game.presented());
    record.opportunities.push(opportunity);
    ordinal++;
    if (mode === 'realtime' && !useAnimationFrame) ordinal = Math.max(ordinal, Math.floor((performance.now() - start) * 60 / 1000));
    if (control !== 'saturation' || performance.now() - start > 2000) drain();
    if (useAnimationFrame) opportunity.drain_finished_ms = performance.now();
    if (!useAnimationFrame) await wait(mode === 'fixed' ? 1 : 0);
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
  if (atmosphere === 'cost') record.haze_cost = JSON.parse(game.haze_cost());
  if (record.renderer_omissions || record.export_omissions || exportError) fail(`incomplete capture: renderer=${record.renderer_omissions}, export=${record.export_omissions ?? 0}, error=${exportError}`);
  record.complete = true;
}

async function captureSpriteField() {
  if (mode !== 'realtime' || control !== 'ordinary' || source !== 'sdr') fail('sprite field requires a realtime SDR capture');
  const canvas = document.querySelector('canvas');
  const extent = [1920, 1080];
  [canvas.width, canvas.height] = extent;
  let store;
  if (spriteScroll || spriteStatus) {
    canvas.style.width = `${extent[0] / devicePixelRatio}px`;
    canvas.style.height = `${extent[1] / devicePixelRatio}px`;
    if (spriteStatus) {
      const fontStore = await fetch_cooked_valley(query.get('assets'));
      game = await create_interface_text_game(canvas, fontStore, devicePixelRatio, 'status', 10_000);
      game.set_interface_output(devicePixelRatio, 0, 0, 0, 0);
    } else game = await create_sprite_probe(canvas, query.get('sprite-probe'));
    const bounds = canvas.getBoundingClientRect();
    record.device_pixel_ratio = devicePixelRatio;
    record.css_extent = [bounds.width, bounds.height];
  } else if (spriteSheets) {
    store = await fetch_cooked_valley(query.get('assets'));
    game = await create_cooked_sprite_game(canvas, store, 500);
  } else {
    game = await create_sprite_game(canvas, 2, 10000);
  }
  const passTimings = spriteScroll && query.has('sprite-pass-timing');
  if (!passTimings) game.capture_without_pass_timings();
  game.set_visible(true, 0);
  game.set_paused(true, 0);
  uploadWorker = await startCaptureUploadWorker({ spool: true });
  for (let index = 0; index < exportSlots; index++) {
    reusablePixels.push(new Uint8Array(extent[0] * extent[1] * 4));
  }
  const deadline = performance.now() + 15000;
  while (!game.route_slot_ready() || game.presented() === 0n) {
    if (performance.now() > deadline) fail('sprite field warmup deadline');
    game.frame(0);
    await wait(2);
  }
  if (store || spriteStatus) {
    while (game.presented() < 8n || (spriteStatus && !JSON.parse(game.status_cost_report()).counts)) {
      if (performance.now() > deadline) fail('sprite sheet warmup deadline');
      game.frame(0);
      await wait(3);
    }
  }
  record.initial_report = JSON.parse(game.route_report(0));
  record.graph = JSON.parse(game.sprite_scene_report()).passes;
  record.scene = scene;
  record.mode = mode;
  record.pixel_source = source;
  record.extent = extent;
  record.scheduling = 'elapsed timer opportunities against a 45 Hz schedule on the visible headless page';
  record.admission_policy = 'wait';
  record.spool_backpressure_opportunities = 0;
  record.gpu_pass_timing = passTimings;
  record.start_before_ms = performance.now();
  game.begin_submitted_capture(1);
  const start = performance.now();
  record.start_after_ms = start;
  game.set_paused(false, 0);
  const period = 1000 / 45;
  let next = 0;
  let save;
  let polling;
  while (performance.now() - start < (spriteStatus ? 4900 : spriteSheets ? 4600 : spriteScroll ? 4400 : 22400)) {
    if (hidden) fail('sprite capture page became hidden');
    drain();
    const elapsed = performance.now() - start;
    if (store && !save && elapsed >= 1600) {
      const requested = performance.now();
      save = fetch('/sprite-sheet-edit', { method: 'POST' }).then(async response => {
        if (!response.ok) fail('sprite sheet save failed');
        record.actions.push({ kind: 'sheet-save', requested_ms: requested,
          acknowledged_ms: performance.now(), ...(await response.json()) });
      }).catch(error => { exportError = String(error); });
      polling = store.next_publication().then(changed => {
        if (!changed) fail('sprite sheet publication not fetched');
        record.actions.push({ kind: 'sheet-fetched', at_ms: performance.now() });
      }).catch(error => { exportError = String(error); });
    }
    if (elapsed < next) {
      await wait(Math.min(3, next - elapsed));
      continue;
    }
    const ordinal = Math.floor(elapsed / period);
    next = (ordinal + 1) * period;
    // Leave room for the last GPU-ready readbacks before offering another frame.
    if (inflight.size >= exportSlots - 3) {
      record.spool_backpressure_opportunities++;
      await wait(1);
      continue;
    }
    game.frame(elapsed);
    record.opportunities.push({
      ordinal, frame_ms: elapsed, returned_ms: performance.now(),
      tick: game.committed_tick(), presented: Number(game.presented()),
    });
    await wait(0);
  }
  await Promise.all([save, polling]);
  const drainUntil = performance.now() + 10000;
  for (;;) {
    drain();
    if (game.submitted_capture_pending() === 0) break;
    if (performance.now() >= drainUntil) fail('sprite field readback deadline');
    await wait(2);
  }
  await Promise.all(inflight);
  record.renderer_omissions = game.submitted_capture_omissions();
  record.end_ms = performance.now();
  record.final_report = JSON.parse(game.route_report(0));
  if (passTimings) record.sprite_timing = JSON.parse(game.sprite_scene_report());
  if (record.renderer_omissions || record.export_omissions || exportError || !record.images.length) {
    fail(`incomplete sprite capture: renderer=${record.renderer_omissions}, export=${record.export_omissions ?? 0}, error=${exportError}`);
  }
  if (record.images.at(-1).submit_ms - record.images[0].submit_ms < (spriteStatus ? 4200 : spriteSheets || spriteScroll ? 3500 : 20000)) fail('sprite capture interval was too short');
  record.spool_bytes = await uploadWorker.seal();
  const directory = await navigator.storage.getDirectory();
  const entry = await directory.getFileHandle('sprite-capture.rgba');
  const file = await entry.getFile();
  if (file.size !== record.spool_bytes || file.size !== record.images.length * extent[0] * extent[1] * 4) {
    fail('sealed browser spool size differs from submitted images');
  }
  // The public browser download streams the File without a page-sized array
  // or an HTTP request per frame. The runner owns the download directory.
  const download = document.createElement('a');
  download.href = URL.createObjectURL(file);
  download.download = 'sprites-field.rgba';
  download.click();
  record.complete = true;
}

async function captureCloudView(view, extent) {
  game.set_paused(true, 0);
  const ready = async (predicate, description, budgetMs = 15000) => {
    const deadline = performance.now() + budgetMs;
    while (!predicate()) {
      if (performance.now() >= deadline) fail(description);
      await wait(2);
    }
  };
  await ready(() => {
    if (JSON.parse(game.cooked_status()).drawn === 1) return true;
    if (game.route_slot_ready()) game.frame(0);
    return false;
  }, 'cloud view did not draw the cooked valley');
  game.set_cloud_capture_eye(...view.eye);
  const edits = JSON.parse(game.apply_parameters(JSON.stringify(view.params), 0));
  if (edits.problems.length || game.committed_tick() !== 0 || !game.paused()) fail(`invalid held cloud view: ${JSON.stringify(edits)}`);
  game.hold_cloud_capture_sun();
  if (query.has('cloud-store')) {
    await ready(() => {
      const placed = JSON.parse(game.placed_cloud_report() ?? 'null');
      if (placed?.resources?.some(([name]) => name.startsWith('cloud-growth '))) return true;
      if (game.route_slot_ready()) game.frame(0);
      return false;
    }, 'cloud volumes did not become resident');
    if (mode === 'fixed') {
      game.set_paused(false, 0);
      let time = 0;
      await ready(() => {
        if (game.committed_tick() >= 300) return true;
        if (game.route_slot_ready()) {
          time = game.route_step_ms(0.5);
          game.frame(time);
        }
        return false;
      }, 'placed cloud weather did not settle at tick 300', 75000);
      game.set_paused(true, time);
    }
  }
  const heldTick = mode === 'fixed' && query.has('cloud-store') ? 300 : 0;
  const heldTime = heldTick ? game.route_step_ms(0.5) : 0;
  if (view.overlay_before !== undefined) {
    if (mode !== 'fixed' || !Number.isInteger(view.overlay_before) ||
        view.overlay_before < 1 || view.overlay_before > 7) fail('invalid initial cloud overlay');
    applyCaptureParameters(JSON.stringify({'clouds.overlay': view.overlay_before}), heldTime);
  }
  const warmupEnd = performance.now() + 3000;
  const warmupStart = game.presented();
  const targetSubmission = view.warmup_submission;
  if (targetSubmission !== undefined && (mode !== 'fixed' ||
      !Number.isSafeInteger(targetSubmission) || targetSubmission <= Number(warmupStart))) {
    fail('invalid fixed cloud warmup submission');
  }
  const deadline = performance.now() + 30000;
  while (targetSubmission === undefined ? performance.now() < warmupEnd
      : Number(game.presented()) < targetSubmission) {
    if (performance.now() >= deadline) fail('fixed cloud warmup missed its submission');
    if (hidden) fail('cloud view became hidden during warmup');
    if (game.route_slot_ready()) game.frame(heldTime);
    await wait(2);
  }
  if (query.has('cloud-store')) record.placed_clouds = JSON.parse(game.placed_cloud_report());
  record.held_warmup_submissions = Number(game.presented() - warmupStart);
  if (!record.held_warmup_submissions || game.committed_tick() !== heldTick) fail('cloud view warmup missed the held tick');
  if (view.overlay_before !== undefined) {
    applyCaptureParameters('{"clouds.overlay":0}', heldTime);
  }
  await ready(() => game.route_slot_ready(), 'cloud view has no free frame slot');
  record.initial_report = JSON.parse(game.route_report(0));
  record.culling = game.report();
  record.start_before_ms = performance.now();
  if (mode === 'fixed') game.begin_fixed_submission(1);
  else game.begin_submitted_capture(1);
  record.start_after_ms = performance.now();
  record.mode = mode;
  record.pixel_source = source;
  record.extent = extent;
  record.base_world_ms = heldTime;
  if (mode === 'fixed') {
    record.scheduling = 'one held T2 submission';
    game.frame(heldTime);
    record.opportunities.push({ordinal: 0, frame_ms: heldTime, tick: game.committed_tick()});
  } else {
    record.scheduling = 'browser animation-frame callbacks at elapsed time';
    const duration = view.duration_seconds;
    if (!Number.isFinite(duration) || duration < 1 || duration > 60) fail('Cloud view duration must be 1 to 60 seconds');
    const start = performance.now();
    record.elapsed_origin_ms = start;
    game.set_paused(false, 0);
    let timestamp = 0;
    let ordinal = 0;
    let discreteEditDone = false;
    const motion = view.motion;
    if (motion && !['pan', 'walk', 'lapse', 'preset', 'cut', 'profile'].includes(motion.kind)) {
      fail('unknown cloud motion');
    }
    const phase = (seconds, start, span) => Math.min(1, Math.max(0, (seconds - start) / span));
    const originalEye = view.eye;
    while (timestamp < duration * 1000) {
      const rafMs = await new Promise(requestAnimationFrame);
      if (hidden) fail('cloud view became hidden during the recording');
      const requested = performance.now();
      timestamp = requested - start;
      const seconds = timestamp / 1000;
      let cloudEdit = null;
      if (motion?.kind === 'pan') {
        const yaw = motion.from + phase(seconds, 2, 0.8) * (motion.to - motion.from);
        game.set_camera_yaw(yaw * Math.PI / 180);
        cloudEdit = { yaw };
      } else if (motion?.kind === 'walk') {
        const eye = [originalEye[0] + 40 * phase(seconds, 1, 5), originalEye[1], originalEye[2]];
        game.move_cloud_capture_eye(...eye, false);
        cloudEdit = { eye };
      } else if (motion && !discreteEditDone && seconds >= 2) {
        discreteEditDone = true;
        if (motion.kind === 'lapse') {
          cloudEdit = { 'clouds.time_lapse': 60 };
        } else if (motion.kind === 'preset') {
          cloudEdit = { 'clouds.coverage': 100, 'clouds.type': 0, 'clouds.time_lapse': 1,
            'clouds.wind_speed': 10, 'clouds.wind_direction': 0 };
        } else if (motion.kind === 'cut') {
          const eye = [originalEye[0] + 350, originalEye[1] + 100, originalEye[2]];
          game.move_cloud_capture_eye(...eye, true);
          cloudEdit = { cut_eye: eye };
        } else if (motion.kind === 'profile') {
          game.cloud_capture_scale(75);
          cloudEdit = { layer_scale: 75 };
        }
        if (['lapse', 'preset'].includes(motion.kind)) applyCaptureParameters(JSON.stringify(cloudEdit), timestamp);
      }
      const before = game.presented();
      game.frame(timestamp);
      const opportunity = { ordinal: ordinal++, requested_ms: requested, frame_ms: timestamp,
        returned_ms: performance.now(), tick: game.committed_tick(), raf_ms: rafMs };
      if (motion) {
        opportunity.cloud_edit = cloudEdit;
        if (game.presented() !== before) {
          opportunity.submission_id = Number(game.presented());
          opportunity.cloud_history_valid = game.cloud_capture_history();
        }
      }
      record.opportunities.push(opportunity);
      drain();
      opportunity.drain_finished_ms = performance.now();
    }
  }
  await ready(() => {
    drain();
    return game.submitted_capture_pending() === 0;
  }, 'cloud view readback timed out');
  await Promise.all(inflight);
  record.renderer_omissions = game.submitted_capture_omissions();
  record.end_ms = performance.now();
  record.final_report = JSON.parse(game.route_report(0));
  if ((mode === 'fixed' ? record.images.length !== 1 : record.images.length < 2)
      || record.renderer_omissions || record.export_omissions || exportError) {
    fail(`incomplete cloud view: ${record.images.length} images, renderer=${record.renderer_omissions}, export=${record.export_omissions ?? 0}, error=${exportError}`);
  }
  record.complete = true;
}

try { await runCapture(); }
catch (error) { record.error = String(error); }
finally {
  uploadWorker?.close();
  game?.end_submitted_capture();
  record.done = true;
  document.querySelector('#status').textContent = JSON.stringify({ complete: record.complete, error: record.error });
  window.thrumCaptureDone?.('');
}
