#!/usr/bin/env node
// Execute the same Wasm exports in a real, isolated Chrome process.
import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { createServer } from 'node:http';

const [webRoot, requestJson] = process.argv.slice(2);
const request = JSON.parse(requestJson);
const scratch = await fs.mkdtemp(path.resolve('tmp/sim2d/chrome-'));
const server = createServer(async (req, res) => {
  try {
    const name = path.basename(new URL(req.url, 'http://localhost').pathname);
    const bytes = await fs.readFile(path.join(webRoot, name));
    res.setHeader('Content-Type', name.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.end(bytes);
  } catch { res.writeHead(404); res.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
  '--headless=new', '--no-first-run', '--no-default-browser-check', '--remote-debugging-port=0',
  `--user-data-dir=${scratch}`, 'about:blank',
], { stdio: ['ignore', 'ignore', 'ignore'] });
let chromeExited = false;
const chromeExit = new Promise(resolve => chrome.once('exit', () => { chromeExited = true; resolve(); }));
let socket;
try {
  let port;
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    try { port = Number((await fs.readFile(path.join(scratch, 'DevToolsActivePort'), 'utf8')).split('\n')[0]); break; }
    catch { await new Promise(resolve => setTimeout(resolve, 100)); }
  }
  if (!port) throw new Error('Chrome did not publish its CDP port');
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  socket = new WebSocket(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
  let sequence = 0;
  const pending = new Map();
  socket.onmessage = event => {
    const message = JSON.parse(event.data);
    const entry = pending.get(message.id);
    if (!entry) return;
    pending.delete(message.id);
    message.error ? entry.reject(new Error(JSON.stringify(message.error))) : entry.resolve(message.result);
  };
  const call = (method, params) => new Promise((resolve, reject) => {
    const id = ++sequence;
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params }));
  });
  const url = `http://127.0.0.1:${server.address().port}/crowd_2d_run.js`;
  // Import from an HTTP origin to keep the generated Wasm loader's fetch local.
  const expression = `(async () => {
    const module = await import(${JSON.stringify(url)});
    await module.default();
    const request = ${JSON.stringify(request)};
    const result = module[request.method](...request.args);
    return { result: JSON.parse(result), userAgent: navigator.userAgent };
  })()`;
  const evaluated = await call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true, timeout: 1800000 });
  if (evaluated.exceptionDetails) throw new Error(JSON.stringify(evaluated.exceptionDetails));
  process.stdout.write(`${JSON.stringify(evaluated.result.value)}\n`);
} finally {
  if (socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify({ id: 2147483647, method: 'Browser.close' }));
  let timer;
  await Promise.race([chromeExit, new Promise(resolve => { timer = setTimeout(resolve, 5000); })]);
  clearTimeout(timer);
  socket?.close();
  if (!chromeExited) { chrome.kill('SIGKILL'); await chromeExit; }
  await new Promise(resolve => server.close(resolve));
  await fs.rm(scratch, { recursive: true, force: true, maxRetries: 20, retryDelay: 100 });
}
