// Retry transport loss only. A replay keeps the exact submitted bytes and ordinal.
export async function uploadCapture(ordinal, image, retried = () => {}) {
  for (let attempt = 0; attempt < 3; attempt++) {
    let response;
    try {
      response = await fetch(`/capture?ordinal=${ordinal}`, {
        signal: AbortSignal.timeout(10000), method: 'POST',
        headers: { 'X-Frame': image.metadata }, body: image.pixels,
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

export async function startCaptureUploadWorker({ spool = false } = {}) {
  const worker = new Worker(new URL('./capture_upload.mjs', import.meta.url), { type: 'module' });
  const pending = new Map();
  let failure;
  let sealed;
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
    if (data.sealed) {
      if (data.error) sealed?.reject(new Error(data.error));
      else sealed?.resolve(data.spool_bytes);
      return;
    }
    const task = pending.get(data.ordinal);
    if (!task) return;
    pending.delete(data.ordinal);
    task.resolve(data);
  };
  worker.onerror = event => {
    failure = new Error(event.message);
    for (const task of pending.values()) task.reject(failure);
    pending.clear();
    sealed?.reject(failure);
  };
  return {
    upload(ordinal, image) {
      if (failure) return Promise.reject(failure);
      return new Promise((resolve, reject) => {
        worker.postMessage({ ordinal, image, spool }, [image.pixels.buffer]);
        pending.set(ordinal, { resolve, reject });
      });
    },
    seal() {
      if (failure) return Promise.reject(failure);
      return new Promise((resolve, reject) => {
        sealed = { resolve, reject };
        worker.postMessage({ seal: true });
      });
    },
    close() { worker.terminate(); },
  };
}

// Transfer ownership through the worker, including on request failure. An
// acknowledgement always returns the same array to the publisher.
if (typeof DedicatedWorkerGlobalScope !== 'undefined' && self instanceof DedicatedWorkerGlobalScope) {
  let opened;
  let ownedBytes = 0;
  self.onmessage = async ({ data: { ordinal, image, spool, seal } }) => {
    const retries = [];
    let error;
    let rgba_sha256;
    try {
      if (seal) {
        const file = await opened;
        file.flush();
        file.close();
      } else if (spool) {
        opened ??= (async () => {
          const directory = await navigator.storage.getDirectory();
          const entry = await directory.getFileHandle('sprite-capture.rgba', { create: true });
          return entry.createSyncAccessHandle();
        })();
        const file = await opened;
        const length = image.pixels.byteLength;
        const offset = ordinal * length;
        // Half the output allowance is reserved for the browser's exported copy.
        if (offset + length > 8 * 1024 ** 3 - 64 * 1024 ** 2) throw new Error('Browser spool allowance exhausted');
        const digest = await crypto.subtle.digest('SHA-256', image.pixels);
        rgba_sha256 = new Uint8Array(digest).toHex();
        // Hashes can finish out of order. The submission ordinal owns its offset.
        if (file.write(image.pixels, { at: offset }) !== length) throw new Error('Short browser spool write');
        ownedBytes += length;
      } else {
        await uploadCapture(ordinal, image, retry => retries.push(retry));
      }
    } catch (failure) { error = String(failure); }
    if (seal) self.postMessage({ sealed: true, error, spool_bytes: ownedBytes });
    else self.postMessage({ ordinal, pixels: image.pixels, retries, error, rgba_sha256 }, [image.pixels.buffer]);
  };
  self.postMessage({ ready: true });
}
