// Hash the bytes actually passed to WebAssembly, inside the page. Returning a
// digest avoids sending a multi-megabyte base64 body over the CDP WebSocket.
export function observeWasmLoads(scope = globalThis) {
  const wasm = scope.WebAssembly;
  const receipts = scope.__thrumWasmReceipts = [];
  const publish = async (bytes, url, method) => {
    const digest = await scope.crypto.subtle.digest('SHA-256', bytes);
    receipts.push({
      sha256: Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, '0')).join(''),
      byteLength: bytes.byteLength, url, method,
    });
  };
  const instantiate = wasm.instantiate;
  wasm.instantiate = async function(source, imports) {
    if (source instanceof wasm.Module) return instantiate.call(wasm, source, imports);
    const bytes = ArrayBuffer.isView(source)
      ? new Uint8Array(source.buffer, source.byteOffset, source.byteLength).slice()
      : new Uint8Array(source).slice();
    const result = await instantiate.call(wasm, source, imports);
    await publish(bytes, null, 'instantiate');
    return result;
  };
  if (wasm.instantiateStreaming) {
    const streaming = wasm.instantiateStreaming;
    wasm.instantiateStreaming = async function(source, imports) {
      const response = await source;
      const bytes = response.clone().arrayBuffer();
      // Observe a clone, leaving the original streaming compiler input intact.
      const [result, data] = await Promise.all([streaming.call(wasm, response, imports), bytes]);
      await publish(data, response.url, 'instantiateStreaming');
      return result;
    };
  }
}

export async function installWasmReceipt(cdp, sessionId) {
  await cdp.call('Page.addScriptToEvaluateOnNewDocument', {
    source: `(${observeWasmLoads.toString()})();`,
  }, sessionId);
}

export async function readWasmReceipt(cdp, sessionId, url, expectedHash, expectedBytes) {
  const response = await cdp.call('Runtime.evaluate', {
    expression: 'globalThis.__thrumWasmReceipts ?? []', returnByValue: true,
  }, sessionId);
  if (response.exceptionDetails) throw new Error(JSON.stringify(response.exceptionDetails));
  const rows = response.result.value;
  if (!Array.isArray(rows)) throw new Error('Missing instantiated-Wasm receipts');
  const matching = rows.filter(row => row.url === url || (row.url === null && row.sha256 === expectedHash));
  if (!matching.length) throw new Error('No receipt for the actual WebAssembly instantiation input');
  if (matching.some(row => row.sha256 !== expectedHash || row.byteLength !== expectedBytes)) {
    throw new Error('Instantiated Wasm differs from the built artifact');
  }
  return matching.at(-1).sha256;
}
