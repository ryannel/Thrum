import assert from 'node:assert/strict';
import { once } from 'node:events';
import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { gunzipSync } from 'node:zlib';
import { test } from 'node:test';
import { captureReceiver } from './browser_capture_receiver.mjs';
import { uploadCapture } from '../examples/v1-game/web/capture_upload.mjs';

async function fixture(t, dropAcknowledgement = false) {
  await fs.mkdir('tmp', { recursive: true });
  const output = await fs.mkdtemp('tmp/capture-upload-test-');
  await fs.mkdir(path.join(output, 'frames'));
  const pixels = Buffer.alloc(1024 * 1024, 73);
  const image = { pixels, metadata: JSON.stringify({ submission_id: 42, tick: 17 }) };
  const manifest = { records: [], export_peak: 0, output_allowance: 4 * pixels.length };
  const failures = [];
  const receiver = captureReceiver(output, pixels.length, manifest, error => failures.push(error));
  const server = http.createServer((request, response) => {
    if (dropAcknowledgement) {
      const end = response.end;
      response.end = function (...args) {
        if (dropAcknowledgement && this.statusCode === 200) {
          dropAcknowledgement = false;
          this.destroy();
          return this;
        }
        return end.apply(this, args);
      };
    }
    receiver.receive(request, response);
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  const origin = `http://127.0.0.1:${server.address().port}`;
  const originalFetch = globalThis.fetch;
  globalThis.fetch = (url, options) => originalFetch(new URL(url, origin), options);
  t.after(async () => {
    globalThis.fetch = originalFetch;
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
    await Promise.all(receiver.pending);
    await fs.rm(output, { recursive: true });
  });
  return { output, image, manifest, failures, receiver, origin, server };
}

test('an interrupted request leaves a hole until the same submitted image is uploaded', async t => {
  const f = await fixture(t);
  const received = once(f.server, 'request');
  const request = http.request(`${f.origin}/capture?ordinal=0`, { method: 'POST', headers: { 'X-Frame': f.image.metadata, 'Content-Length': f.image.pixels.length } });
  request.on('error', () => {});
  request.write(f.image.pixels.subarray(0, 100));
  const [incoming] = await received;
  const aborted = once(incoming, 'aborted');
  request.destroy();
  await aborted;
  await Promise.all(f.receiver.pending);
  assert.equal(f.manifest.records.length, 0);
  assert.equal(f.manifest.export_interruptions.length, 1);
  assert.deepEqual(f.failures, []);
  await uploadCapture(0, f.image);
  const raw = gunzipSync(await fs.readFile(path.join(f.output, f.manifest.records[0].file)));
  assert.deepEqual(raw, f.image.pixels);
});

test('a lost acknowledgement retries identical bytes and stores exactly one frame', async t => {
  const f = await fixture(t, true);
  const retries = [];
  await uploadCapture(0, f.image, row => retries.push(row));
  assert.equal(retries.length, 1);
  assert.equal(f.manifest.export_replays, 1);
  assert.equal(f.manifest.records.length, 1);
  assert.equal(f.manifest.records[0].submission_id, 42);
  assert.equal((await fs.readdir(path.join(f.output, 'frames'))).length, 1);
  assert.deepEqual(f.failures, []);
});

test('concurrent identical deliveries store one frame and conflicting bytes or metadata fail', async t => {
  const f = await fixture(t);
  await Promise.all([uploadCapture(0, f.image), uploadCapture(0, f.image)]);
  assert.equal(f.manifest.export_replays, 1);
  const before = await fs.readFile(path.join(f.output, f.manifest.records[0].file));
  await assert.rejects(uploadCapture(0, { ...f.image, pixels: Buffer.alloc(f.image.pixels.length, 19) }), /HTTP 500/);
  await assert.rejects(uploadCapture(0, { ...f.image, metadata: '{"submission_id":43}' }), /HTTP 500/);
  assert.equal(f.failures.length, 2);
  assert.ok(f.failures.every(error => /Conflicting record replay/.test(String(error))));
  assert.deepEqual(await fs.readFile(path.join(f.output, f.manifest.records[0].file)), before);
});

test('persistent transport loss fails after three attempts', async t => {
  const originalFetch = globalThis.fetch;
  t.after(() => { globalThis.fetch = originalFetch; });
  let attempts = 0;
  globalThis.fetch = async () => { attempts++; throw new TypeError('connection reset'); };
  await assert.rejects(uploadCapture(7, { metadata: '{}', pixels: new Uint8Array(4) }), /connection reset/);
  assert.equal(attempts, 3);
});
