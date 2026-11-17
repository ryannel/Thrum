import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { promisify } from 'node:util';
import { gzip } from 'node:zlib';

const compress = promisify(gzip);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');

// An acknowledgement may be lost after storage. Replays must match both the
// original submitted bytes and metadata, and never create a second frame.
export function captureReceiver(output, frameBytes, manifest, failed, slow = false) {
  const recordLimit = manifest.record_limit ?? 1024;
  const exportSlots = manifest.export_slots ?? 8;
  let active = 0, outputBytes = 0;
  const pending = new Set(), writing = new Map(), headers = new Map();
  function receive(request, response) {
    if (!request.url.startsWith('/capture?')) return false;
    let ordinal;
    const work = (async () => {
      if (request.method !== 'POST' || active >= exportSlots) { request.resume(); response.writeHead(429).end(); return; }
      active++; manifest.export_peak = Math.max(manifest.export_peak, active);
      try {
        ordinal = Number(new URL(request.url, 'http://localhost').searchParams.get('ordinal'));
        if (!Number.isSafeInteger(ordinal) || ordinal < 0 || ordinal >= recordLimit) throw new Error('Invalid record ordinal');
        const header = request.headers['x-frame'];
        const metadata = JSON.parse(header);
        const chunks = []; let length = 0;
        for await (const chunk of request) {
          length += chunk.length;
          if (length > frameBytes) throw new Error('Oversized frame');
          chunks.push(chunk);
        }
        if (length !== frameBytes) throw new Error('Truncated frame');
        const raw = Buffer.concat(chunks), digest = sha(raw);
        while (writing.has(ordinal)) await writing.get(ordinal);
        if (manifest.records[ordinal]) {
          if (manifest.records[ordinal].rgba_sha256 !== digest || headers.get(ordinal) !== header) throw new Error('Conflicting record replay');
          manifest.export_replays = (manifest.export_replays ?? 0) + 1;
        } else {
          const stored = (async () => {
            if (slow) await new Promise(resolve => setTimeout(resolve, 500));
            const encoded = await compress(raw, { level: 1 });
            outputBytes += encoded.length;
            if (outputBytes > manifest.output_allowance) throw new Error('Output allowance exhausted');
            const file = `frames/${String(ordinal).padStart(6, '0')}.rgba.gz`;
            await fs.writeFile(path.join(output, file), encoded, { flag: 'wx' });
            headers.set(ordinal, header);
            manifest.records[ordinal] = { ...metadata, ordinal, file, rgba_sha256: digest, bytes: encoded.length };
          })();
          writing.set(ordinal, stored);
          try { await stored; } finally { writing.delete(ordinal); }
        }
        response.writeHead(200).end();
      } finally { active--; }
    })().catch(error => {
      if (error.code === 'ECONNRESET' && request.aborted) {
        (manifest.export_interruptions ??= []).push({ ordinal, error: String(error) });
      } else failed(error);
      if (!response.destroyed) response.writeHead(500).end(String(error));
    });
    pending.add(work); work.then(() => pending.delete(work));
    return true;
  }
  return { receive, pending, bytes: () => outputBytes };
}
