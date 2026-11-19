// Retry transport loss only. A replay keeps the exact submitted bytes and ordinal.
export async function uploadCapture(ordinal, image, retried = () => {}, pack = false) {
  const body = pack ? packRgbaRuns(image.pixels) : image.pixels;
  const headers = { 'X-Frame': image.metadata };
  if (body !== image.pixels) headers['X-Pixel-Encoding'] = 'rgba-runs';
  if (pack) {
    const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', image.pixels));
    headers['X-Pixel-Sha256'] = Array.from(digest, byte => byte.toString(16).padStart(2, '0')).join('');
  }
  for (let attempt = 0; attempt < 3; attempt++) {
    let response;
    try {
      response = await fetch(`/capture?ordinal=${ordinal}`, {
        signal: AbortSignal.timeout(10000), method: 'POST',
        headers, body,
      });
    } catch (error) {
      if (attempt === 2) throw error;
      retried({ ordinal, attempt: attempt + 1, error: String(error) });
      await new Promise(resolve => setTimeout(resolve, 100));
      continue;
    }
    if (!response.ok) throw new Error(`export HTTP ${response.status}`);
    return;
  }
}

// Each packet holds a signed word count followed by literal RGBA words
// (positive count) or one repeated word (negative count). Incompressible
// frames retain the normal raw transport.
export function packRgbaRuns(pixels) {
  const input = new Uint32Array(pixels.buffer, pixels.byteOffset, pixels.byteLength / 4);
  const encoded = new Int32Array(input.length);
  let cursor = 0, end = 0;
  const beginsRun = at => at + 2 < input.length && input[at] === input[at + 1] && input[at] === input[at + 2];
  while (cursor < input.length) {
    const start = cursor;
    if (beginsRun(cursor)) {
      cursor += 3;
      while (cursor < input.length && input[cursor] === input[start]) cursor++;
      if (end + 2 >= encoded.length) return pixels;
      encoded[end++] = start - cursor;
      encoded[end++] = input[start];
    } else {
      do { cursor++; } while (cursor < input.length && !beginsRun(cursor));
      const count = cursor - start;
      if (end + 1 + count >= encoded.length) return pixels;
      encoded[end++] = count;
      encoded.set(input.subarray(start, cursor), end);
      end += count;
    }
  }
  return new Uint8Array(encoded.buffer, 0, end * 4);
}

export async function startCaptureUploadWorker(pack = false) {
  const worker = new Worker(new URL('./capture_upload.mjs', import.meta.url), { type: 'module' });
  const pending = new Map();
  let failure;
  try {
    await new Promise((resolve, reject) => {
      worker.onmessage = ({ data }) => { if (data.ready) resolve(); };
      worker.onerror = reject;
    });
  } catch (error) {
    worker.terminate();
    throw error;
  }
  worker.onmessage = ({ data }) => {
    const task = pending.get(data.ordinal);
    if (!task) return;
    pending.delete(data.ordinal);
    task.resolve(data);
  };
  worker.onerror = event => {
    failure = new Error(event.message);
    for (const task of pending.values()) task.reject(failure);
    pending.clear();
  };
  return {
    upload(ordinal, image) {
      if (failure) return Promise.reject(failure);
      return new Promise((resolve, reject) => {
        worker.postMessage({ ordinal, image, pack }, [image.pixels.buffer]);
        pending.set(ordinal, { resolve, reject });
      });
    },
    close() { worker.terminate(); },
  };
}

// Transfer ownership through the worker, including on request failure. An
// acknowledgement always returns the same array to the publisher.
if (typeof DedicatedWorkerGlobalScope !== 'undefined' && self instanceof DedicatedWorkerGlobalScope) {
  self.onmessage = async ({ data: { ordinal, image, pack } }) => {
    const retries = [];
    let error;
    try { await uploadCapture(ordinal, image, retry => retries.push(retry), pack); }
    catch (failure) { error = String(failure); }
    self.postMessage({ ordinal, pixels: image.pixels, retries, error }, [image.pixels.buffer]);
  };
  self.postMessage({ ready: true });
}
