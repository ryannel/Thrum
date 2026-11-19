import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { promisify } from 'node:util';
import { gzip } from 'node:zlib';

const compress = promisify(gzip);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');

export function unpackRgbaRuns(encoded, frameBytes) {
  if (encoded.length % 4 !== 0 || frameBytes % 4 !== 0) throw new Error('Unaligned RGBA run packet');
  const raw = Buffer.allocUnsafe(frameBytes);
  const words = new Uint32Array(raw.buffer, raw.byteOffset, frameBytes / 4);
  let cursor = 0, output = 0;
  while (cursor < encoded.length && output < words.length) {
    const count = encoded.readInt32LE(cursor);
    cursor += 4;
    if (count === 0 || output + Math.abs(count) > words.length) throw new Error('Invalid RGBA run length');
    if (count < 0) {
      if (cursor + 4 > encoded.length) throw new Error('Truncated RGBA run');
      words.fill(encoded.readUInt32LE(cursor), output, output - count);
      cursor += 4;
    } else {
      if (cursor + count * 4 > encoded.length) throw new Error('Truncated RGBA literals');
      encoded.copy(raw, output * 4, cursor, cursor + count * 4);
      cursor += count * 4;
    }
    output += Math.abs(count);
  }
  if (cursor !== encoded.length || output !== words.length) throw new Error('Invalid RGBA run packet extent');
  return raw;
}

// An acknowledgement may be lost after storage. Replays must match both the
// original submitted bytes and metadata, and never create a second frame.
export function captureReceiver(output, frameBytes, manifest, failed, { slow = false, rawSpool, acceptRuns = false } = {}) {
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
        const packet = Buffer.concat(chunks);
        const encoding = request.headers['x-pixel-encoding'];
        if (encoding !== undefined && (!acceptRuns || encoding !== 'rgba-runs')) throw new Error('Undeclared pixel transport');
        if (encoding === undefined && length !== frameBytes) throw new Error('Truncated frame');
        if (acceptRuns && !/^[0-9a-f]{64}$/.test(request.headers['x-pixel-sha256'] ?? '')) {
          throw new Error('Sprite source lacks its original pixel hash');
        }
        const raw = acceptRuns ? null : encoding === 'rgba-runs' ? unpackRgbaRuns(packet, frameBytes) : packet;
        const digest = acceptRuns ? request.headers['x-pixel-sha256'] : sha(raw);
        const packetDigest = acceptRuns ? sha(packet) : null;
        if (acceptRuns) manifest.transport_bytes = (manifest.transport_bytes ?? 0) + length;
        while (writing.has(ordinal)) await writing.get(ordinal);
        if (manifest.records[ordinal]) {
          if (manifest.records[ordinal].rgba_sha256 !== digest || manifest.records[ordinal].transport_sha256 !== (packetDigest ?? undefined) || headers.get(ordinal) !== header) throw new Error('Conflicting record replay');
          manifest.export_replays = (manifest.export_replays ?? 0) + 1;
        } else {
          const stored = (async () => {
            if (slow) await new Promise(resolve => setTimeout(resolve, 500));
            const encoded = acceptRuns ? packet : rawSpool ? raw : await compress(raw, { level: 1 });
            outputBytes += encoded.length;
            if (outputBytes > manifest.output_allowance) throw new Error('Output allowance exhausted');
            const file = `frames/${String(ordinal).padStart(6, '0')}.${acceptRuns ? 'packet' : 'rgba.gz'}`;
            if (rawSpool) {
              let written = 0;
              while (written < frameBytes) {
                const result = await rawSpool.write(raw, written, frameBytes - written, ordinal * frameBytes + written);
                if (result.bytesWritten === 0) throw new Error('Short raw spool write');
                written += result.bytesWritten;
              }
            } else {
              await fs.writeFile(path.join(output, file), encoded, { flag: 'wx' });
            }
            headers.set(ordinal, header);
            const transport = acceptRuns ? { transport_encoding: encoding ?? 'raw', transport_sha256: packetDigest } : {};
            manifest.records[ordinal] = { ...metadata, ...transport, ordinal, file, rgba_sha256: digest, bytes: encoded.length };
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
  return { receive, pending };
}
