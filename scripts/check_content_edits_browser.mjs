// Runs the watched valley on Chrome's ordinary frame path, optionally with the T2 observer.
import fs from 'node:fs/promises';
import path from 'node:path';
import { execFile, execFileSync } from 'node:child_process';
import { loadavg } from 'node:os';
import { promisify } from 'node:util';
import { captureReceiver } from './browser_capture_receiver.mjs';

const run = promisify(execFile);
const extent = [640, 360];

export async function loadContentEdits(files, output) {
  const capture = process.env.THRUM_CONTENT_EDIT_CAPTURE === '1';
  const manifest = { target: 'chrome-webgpu', source: 'sdr', mode: 'realtime', extent,
    record_limit: 4096, records: [], export_slots: 32, export_peak: 0, output_allowance: 4 * 1024 ** 3 };
  const file = (bytes, type) => ({ bytes, type });
  files.set('/content-edits.html', file(Buffer.from(
    '<!doctype html><style>body{margin:0}</style><canvas width="640" height="360"></canvas><script type="module" src="/content-edits.mjs"></script>'
  ), 'text/html'));
  files.set('/content-edits.mjs', file(await fs.readFile('scripts/content_edits_page.mjs'), 'text/javascript'));
  files.set('/capture_upload.mjs', file(await fs.readFile('examples/v1-game/web/capture_upload.mjs'), 'text/javascript'));
  files.set('/moving-v1.json', file(await fs.readFile('examples/v1-game/routes/moving-v1.json'), 'application/json'));
  let exportError;
  if (!capture) return { capture };
  await fs.mkdir(path.join(output, 'frames'));
  const receiver = captureReceiver(output, extent[0] * extent[1] * 4, manifest, error => { exportError = error; });
  return { capture, manifest, ...receiver, error: () => exportError };
}

const maxInterval = rows => Math.max(...rows.map(row => row.interval_ms));

export async function checkContentEdits(cdp, origin, assetsOrigin, source, output, waitFor, observer) {
  const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
  const frames = [], edits = [];
  const larger = [];
  const requests = [];
  const previousEvents = cdp.event;
  let recording = false, pageError;
  cdp.event = event => {
    previousEvents?.(event);
    if (event.sessionId !== sessionId) return;
    if (event.method === 'Runtime.exceptionThrown') {
      const exception = event.params.exceptionDetails;
      pageError = exception.exception?.description ?? exception.text;
    }
    if (recording && event.method === 'Network.requestWillBeSent' &&
        event.params.request.url.startsWith(assetsOrigin + '/cas/')) {
      requests.push(event.params.request.url);
    }
  };
  const evaluate = async expression => {
    const result = await cdp.call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, sessionId);
    if (result.exceptionDetails) throw new Error(result.result.description ?? JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  const loadBefore = loadavg();
  const commit = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  try {
    await cdp.call('Runtime.enable', {}, sessionId);
    await cdp.call('Network.enable', {}, sessionId);
    await cdp.call('Target.activateTarget', { targetId });
    const query = new URLSearchParams({ assets: assetsOrigin, capture: String(observer.capture) });
    await cdp.call('Page.navigate', { url: origin + '/content-edits.html?' + query }, sessionId);
    await waitFor(async () => {
      if (pageError) throw new Error(pageError);
      const state = await evaluate('window.contentEdits?.state() ?? null');
      if (state?.error) throw new Error(state.error);
      return state?.ready;
    }, 90000, 'running reconstructed valley preview');
    recording = true;
    const waitFrames = count => evaluate('contentEdits.waitFrames(' + count + ')');
    const steady = [];
    for (const [index, product] of ['mesh', 'texture', 'volume'].entries()) {
      await waitFrames(120);
      steady.push(...await evaluate('contentEdits.takeFrames()'));
      const before = requests.length;
      // Keep the real long poll in flight while the editor renames its source.
      const start = await evaluate('contentEdits.poll()');
      await run('python3', ['scripts/edit_valley_content.py', source, product]);
      const generation = index + 2;
      const update = await evaluate('contentEdits.waitGeneration(' + generation + ')');
      await waitFrames(12);
      const window = await evaluate('contentEdits.takeFrames()');
      const switched = window.find(row => row.submitted && row.content.placement === generation)?.content;
      if (!switched || switched.swaps !== generation) {
        throw new Error(product + ': no single submitted publication switch');
      }
      const changed = requests.slice(before);
      if (changed.length !== 2 || new Set(changed).size !== 2) {
        throw new Error(product + ': expected one manifest and one changed product, got ' + JSON.stringify(changed));
      }
      const head = await fs.readFile(path.join(source, '..', 'store', 'HEAD'));
      const manifestHash = head.subarray(24, 56).toString('hex');
      if (!changed.includes(assetsOrigin + '/cas/' + manifestHash)) {
        throw new Error(product + ': no fetched manifest for the submitted publication');
      }
      const productHash = path.basename(changed.find(url => !url.endsWith(manifestHash)));
      const productBytes = (await fs.stat(path.join(source, '..', 'store', 'cas', productHash.slice(0, 2), productHash))).size;
      edits.push({ product, generation, start, fetchTiming: update.fetchTiming, switched, rows: window, requests: changed,
        productHash, productBytes, edit_max_interval_ms: maxInterval(window) });
    }
    await waitFrames(120);
    steady.push(...await evaluate('contentEdits.takeFrames()'));
    if (!observer.capture && process.env.THRUM_CONTENT_EDIT_PROBE === '1') {
      for (const [index, size] of [192, 256, 320, 384, 448, 512].entries()) {
        const probeSteady = await evaluate('contentEdits.takeFrames()');
        await waitFrames(120);
        probeSteady.push(...await evaluate('contentEdits.takeFrames()'));
        const start = await evaluate('contentEdits.poll()');
        await run('python3', ['scripts/edit_valley_content.py', source, 'volume', '--volume-size', String(size)]);
        const changed = await evaluate('contentEdits.waitGeneration(' + (index + 5) + ')');
        await waitFrames(12);
        const window = await evaluate('contentEdits.takeFrames()');
        const editMax = maxInterval(window), steadyMax = maxInterval(probeSteady);
        larger.push({ extent: [size, size, size], sourceBytes: 16 + size ** 3, generation: changed.placement,
          start, fetchTiming: changed.fetchTiming, editMax, steadyMax, losesInterval: editMax > steadyMax,
          steady: probeSteady, rows: window });
        if (editMax > steadyMax) break;
      }
    }
    const stopped = await evaluate('contentEdits.stop()');
    frames.push(...steady, ...edits.flatMap(edit => edit.rows),
      ...larger.flatMap(probe => [...probe.steady, ...probe.rows]), ...stopped.tail);
    frames.sort((a, b) => a.timestamp_ms - b.timestamp_ms);
    const submissions = frames.filter(row => row.submitted);
    if (!submissions.length || submissions.some(({ content }, i) =>
      content.placement !== content.row || content.placement !== content.lentTexture ||
      content.placement !== content.lentVolume || content.reconstruction !== 'Taau' || content.errors.length ||
      content.copyBytes > 8 * 1024 ** 2 || content.stagingPeakBytes > 32 * 1024 ** 2 ||
      (i > 0 && content.submissionId !== submissions[i - 1].content.submissionId + 1))) {
      throw new Error('Submitted placements, rows and lease diverged or the frame record has a gap');
    }
    const steadyMax = maxInterval(steady);
    const late = edits.filter(edit => edit.edit_max_interval_ms > steadyMax);
    const record = { commit, target: 'chrome-webgpu', extent, reconstruction: 'Taau',
      load_before: loadBefore, load_after: loadavg(), capture: observer.capture, steadyMax, late: late.map(edit => edit.product),
      edits, steady, larger, tail: stopped.tail, submissions: submissions.length };
    await fs.writeFile(path.join(output, 'content-edits.json'), JSON.stringify(record));
    if (observer.capture) {
      await Promise.all(observer.pending);
      const images = observer.manifest.records;
      if (observer.error() || stopped.omissions || stopped.images !== images.length ||
          images.length !== submissions.length ||
          submissions.some((row, i) => images[i]?.submission_id !== row.content.submissionId)) {
        throw new Error('Missing T2 image for a recorded submission: ' + JSON.stringify({
          error: String(observer.error()), omissions: stopped.omissions,
          images: stopped.images, received: images.length, submitted: submissions.length,
        }));
      }
      Object.assign(observer.manifest, { complete: true, commit, reconstruction: 'Taau' });
      await fs.writeFile(path.join(output, 'manifest.json'), JSON.stringify(observer.manifest));
    } else if (late.length) {
      throw new Error('Edit callback interval exceeded steady maximum: ' + late.map(edit =>
        edit.product + ' ' + edit.edit_max_interval_ms.toFixed(2) + ' ms versus ' + steadyMax.toFixed(2) + ' ms'
      ).join(', '));
    }
    return { case: 'content-edits', target: 'chrome-webgpu', capture: observer.capture, submissions: submissions.length,
      edits: edits.map(({ product, generation, edit_max_interval_ms }) => ({ product, generation, edit_max_interval_ms })), steadyMax };
  } finally {
    cdp.event = previousEvents;
    await cdp.call('Target.closeTarget', { targetId });
  }
}
