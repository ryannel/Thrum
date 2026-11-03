// Actual rendered canvas recording; deliberately separate from quiet performance timing.
export async function recordCanvas(canvas, durationMs = 12000) {
  if (!canvas.captureStream || !globalThis.MediaRecorder)
    throw Error("Canvas recording unavailable");
  const mimeType = [
    "video/webm;codecs=vp9",
    "video/webm;codecs=vp8",
    "video/webm",
  ].find((t) => MediaRecorder.isTypeSupported(t));
  if (!mimeType) throw Error("WebM recording unavailable");
  const stream = canvas.captureStream(15),
    chunks = [];
  const started = performance.now();
  try {
    return await new Promise((resolve, reject) => {
      const recorder = new MediaRecorder(stream, {
        mimeType,
        videoBitsPerSecond: durationMs > 60000 ? 750000 : 1500000,
      });
      const timer = setTimeout(() => {
        if (recorder.state !== "inactive") recorder.stop();
      }, durationMs);
      recorder.ondataavailable = (e) => {
        if (e.data.size) chunks.push(e.data);
      };
      recorder.onerror = (e) => {
        clearTimeout(timer);
        reject(Error(e.error?.message ?? "Motion recorder failed"));
      };
      recorder.onstop = () => {
        clearTimeout(timer);
        const blob = new Blob(chunks, { type: mimeType });
        if (!blob.size) reject(Error("Motion recorder produced no frames"));
        else
          resolve({
            blob,
            mimeType,
            elapsedMs: performance.now() - started,
            requestedFps: 15,
          });
      };
      recorder.start(1000);
    });
  } finally {
    for (const track of stream.getTracks()) track.stop();
  }
}

// Endpoint equality alone misses controls changed and changed back during a clip.
export function validateMotion(start, end) {
  if (start.revision !== end.revision) return "fixture-changed";
  if (end.fault || end.gpuErrors || !end.playing) return "faulted-or-paused";
  if (start.epoch !== end.epoch) return "epoch-changed";
  if (!(end.sourceTime > start.sourceTime && end.imageFrame > start.imageFrame))
    return "no-source-or-image-progress";
  return "fixed-fixture";
}
