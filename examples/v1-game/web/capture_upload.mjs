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
