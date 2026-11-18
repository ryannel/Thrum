// A running BrowserGame, one ordinary RAF callback per opportunity.
import init, { create_route_game, fetch_cooked_valley } from '/v1_game.js';
import { startCaptureUploadWorker } from '/capture_upload.mjs';

const args = new URLSearchParams(location.search);
const capture = args.get('capture') === 'true';
const pending = new Set();
const pixels = [];
let game, valley, worker, error, stopped = false, ready = false;
let rows = [], serial = 0, lastTimestamp, lastSubmission, content, images = 0, fetching;
let fetchTiming;

const until = async predicate => {
  const deadline = performance.now() + 90000;
  while (!predicate()) {
    if (error || performance.now() > deadline) throw Error(error || 'valley edit deadline');
    await new Promise(resolve => setTimeout(resolve, 10));
  }
};

function drain() {
  if (!capture) return;
  for (;;) {
    const buffer = pixels.pop();
    if (!buffer) return;
    const image = game.next_submitted_image(buffer);
    if (!image) { pixels.push(buffer); return; }
    const upload = worker.upload(images++, image).then(reply => {
      pixels.push(reply.pixels);
      if (reply.error) throw Error(reply.error);
    }).catch(failure => { error = String(failure); }).finally(() => pending.delete(upload));
    pending.add(upload);
  }
}

function frame(timestamp) {
  try {
    if (stopped) return;
    if (document.hidden) throw Error('RAF page hidden');
    game.set_camera_yaw(0.052 * Math.sin(timestamp / 1400));
    const start = performance.now();
    game.frame(timestamp);
    const elapsed = performance.now() - start;
    const next = JSON.parse(game.content_preview_frame());
    const submitted = next.submissionId !== lastSubmission;
    if (ready) rows.push({ timestamp_ms: timestamp, interval_ms: timestamp - lastTimestamp,
      frame_ms: elapsed, submitted, drawable_presented: next.presented > content.presented, content: next });
    content = next;
    lastSubmission = next.submissionId;
    lastTimestamp = timestamp;
    serial++;
    drain();
    requestAnimationFrame(frame);
  } catch (failure) { error = String(failure); }
}

window.contentEdits = {
  state: () => ({ ready, error }),
  takeFrames: () => { const result = rows; rows = []; return result; },
  waitFrames: async count => { const end = serial + count; await until(() => serial >= end); },
  poll: () => {
    fetchTiming = { start_ms: performance.now() };
    fetching = valley.next_publication().then(changed => {
      fetchTiming.finish_ms = performance.now();
      return changed;
    }).catch(failure => { error = String(failure); });
    return fetchTiming.start_ms;
  },
  waitGeneration: async generation => {
    if (!await fetching) throw Error(error || 'dev feed did not publish');
    await until(() => content?.placement === generation);
    return { ...content, fetchTiming };
  },
  stop: async () => {
    stopped = true;
    await until(() => { drain(); return !capture || game.submitted_capture_pending() === 0; });
    await Promise.all(pending);
    worker?.close();
    return { tail: rows, images, omissions: game.submitted_capture_omissions() };
  },
};

try {
  await init();
  const route = await (await fetch('/moving-v1.json')).json();
  route.render_extent = [640, 360];
  game = await create_route_game(document.querySelector('canvas'), JSON.stringify(route), false, 'frustum', true, true);
  game.set_time_of_day(0.5);
  valley = await fetch_cooked_valley(args.get('assets'));
  game.attach_cooked_valley(valley, undefined, true);
  game.set_visible(true, performance.now());
  requestAnimationFrame(frame);
  await until(() => content?.placement === 1 && content.row === 1 &&
    content.lentTexture === 1 && content.lentVolume === 1);
  await window.contentEdits.waitFrames(60);
  if (capture) {
    worker = await startCaptureUploadWorker();
    for (let i = 0; i < 32; i++) pixels.push(new Uint8Array(640 * 360 * 4));
    game.begin_submitted_capture(1);
  }
  ready = true;
} catch (failure) { error = String(failure); }
