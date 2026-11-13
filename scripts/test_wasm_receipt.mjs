import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { observeWasmLoads, readWasmReceipt } from './wasm_receipt.mjs';

const url = 'http://127.0.0.1/v1_game_bg.wasm';
const header = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
function scope() {
  const value = { crypto: globalThis.crypto, WebAssembly: {
    Module: WebAssembly.Module, instantiate: WebAssembly.instantiate,
    instantiateStreaming: WebAssembly.instantiateStreaming,
  } };
  observeWasmLoads(value);
  return value;
}
function cdp(value) {
  return { call: async method => {
    assert.equal(method, 'Runtime.evaluate');
    return { result: { value: value.__thrumWasmReceipts } };
  } };
}
function leb(value) {
  const result = [];
  do { const byte = value & 127; value >>>= 7; result.push(byte | (value ? 128 : 0)); } while (value);
  return Buffer.from(result);
}

test('streamed compiled bytes larger than a 4 MiB base64 CDP reply keep exact identity', async () => {
  const custom = Buffer.alloc(3 * 1024 * 1024 + 256 * 1024);
  const bytes = Buffer.concat([header, Buffer.from([0]), leb(custom.length + 1), Buffer.from([0]), custom]);
  assert.ok(Math.ceil(bytes.length / 3) * 4 > 4 * 1024 * 1024);
  const response = new Response(bytes, { headers: { 'Content-Type': 'application/wasm' } });
  Object.defineProperty(response, 'url', { value: url });
  const value = scope();
  const compiled = await value.WebAssembly.instantiateStreaming(Promise.resolve(response), {});
  assert.ok(compiled.instance instanceof WebAssembly.Instance);
  assert.equal(await readWasmReceipt(cdp(value), 'session', url, sha(bytes), bytes.length), sha(bytes));
  assert.ok(JSON.stringify(value.__thrumWasmReceipts).length < 512);
  await assert.rejects(readWasmReceipt(cdp(value), 'session', url, sha(header), bytes.length), /differs/);
});

test('buffer fallback hashes only the actual view, without surrounding bytes', async () => {
  const value = scope();
  const padded = Buffer.concat([Buffer.alloc(11, 255), header, Buffer.alloc(7, 255)]);
  const compiled = await value.WebAssembly.instantiate(padded.subarray(11, 11 + header.length), {});
  assert.ok(compiled.instance instanceof WebAssembly.Instance);
  assert.equal(await readWasmReceipt(cdp(value), 'session', url, sha(header), header.length), sha(header));
  await assert.rejects(readWasmReceipt(cdp(value), 'session', url, sha(header), header.length + 1), /differs/);
});

test('a failed compilation and an opaque precompiled module cannot fabricate a receipt', async () => {
  const value = scope();
  await assert.rejects(value.WebAssembly.instantiate(new Uint8Array([1, 2, 3]), {}), WebAssembly.CompileError);
  await assert.rejects(value.WebAssembly.instantiateStreaming(new Response(new Uint8Array([1, 2, 3]), {
    headers: { 'Content-Type': 'application/wasm' },
  }), {}), WebAssembly.CompileError);
  const module = await WebAssembly.compile(header);
  assert.ok(await value.WebAssembly.instantiate(module, {}) instanceof WebAssembly.Instance);
  await assert.rejects(readWasmReceipt(cdp(value), 'session', url, sha(header), header.length), /No receipt/);
});
