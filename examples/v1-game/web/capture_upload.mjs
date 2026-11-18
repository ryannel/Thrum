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

export async function startCaptureUploadWorker() {
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
        worker.postMessage({ ordinal, image }, [image.pixels.buffer]);
        pending.set(ordinal, { resolve, reject });
      });
    },
    close() { worker.terminate(); },
  };
}

// Transfer ownership through the worker, including on request failure. An
// acknowledgement always returns the same array to the publisher.
if (typeof DedicatedWorkerGlobalScope !== 'undefined' && self instanceof DedicatedWorkerGlobalScope) {
  self.onmessage = async ({ data: { ordinal, image } }) => {
    const retries = [];
    let error;
    try { await uploadCapture(ordinal, image, retry => retries.push(retry)); }
    catch (failure) { error = String(failure); }
    self.postMessage({ ordinal, pixels: image.pixels, retries, error }, [image.pixels.buffer]);
  };
  self.postMessage({ ready: true });
}
