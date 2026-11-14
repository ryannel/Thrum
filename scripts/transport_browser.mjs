#!/usr/bin/env node
// Schedules Rust/Wasm ticks and records them. No message exchange is implemented in JS.
import fs from 'node:fs/promises';
import path from 'node:path';
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
const [ticket, directory, secondsText = '32', output, proofCase = 'exchange', controlPath] = process.argv.slice(2);
const seconds = Number(secondsText);
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const root = path.resolve('target/link-proof-web');
await fs.mkdir(directory, { recursive: true });
const server = createServer(async (req, res) => {
  try {
    if (req.url === '/control') { res.end(controlPath ? await fs.readFile(controlPath,'utf8').catch(()=>'') : ''); return; }
    const file = req.url === '/' ? 'index.html' : path.basename(req.url);
    if (file === 'index.html') { res.end('<!doctype html><title>Thrum transport proof</title>'); return; }
    res.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
    res.end(await fs.readFile(path.join(root, file)));
  } catch { res.writeHead(404).end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const args = ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${path.resolve(directory)}`, '--no-first-run', 'about:blank'];
const chrome = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', args, { detached:true, stdio: ['ignore', 'ignore', 'pipe'] });
await fs.writeFile(path.join(directory,'driver.json'),JSON.stringify({chrome_pid:chrome.pid}));
let stderr = '';
chrome.stderr.on('data', chunk => { stderr = (stderr + chunk.toString()).slice(-6000); });
let socket;
let next = 0;
const calls = new Map();
let cleaning;
function cleanup() {
  return cleaning ??= (async () => {
    socket?.close();
    server.closeAllConnections();
    server.close();
    if (chrome.exitCode === null && chrome.signalCode === null) {
      const exited = new Promise(resolve=>chrome.once('exit',resolve));
      try { process.kill(-chrome.pid,'SIGTERM'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
      await Promise.race([exited,delay(1000)]);
      if (chrome.exitCode === null && chrome.signalCode === null) {
        try { process.kill(-chrome.pid,'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
        await Promise.race([exited,delay(1000)]);
      }
    }
    chrome.stderr.destroy();
    chrome.unref();
  })();
}
process.once('SIGTERM',()=>{ cleanup().finally(()=>process.exit(143)); });
process.once('SIGINT',()=>{ cleanup().finally(()=>process.exit(130)); });
try {
  let port;
  for (let i = 0; i < 150; ++i) {
    try { port = Number((await fs.readFile(path.join(directory, 'DevToolsActivePort'), 'utf8')).split('\n')[0]); break; } catch { await delay(100); }
  }
  if (!port) throw new Error(`Chrome did not start: ${stderr}`);
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  socket = new WebSocket(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  socket.addEventListener('message', event => {
    const packet = JSON.parse(event.data);
    const callback = calls.get(packet.id);
    if (callback) { clearTimeout(callback.timer); calls.delete(packet.id); packet.error ? callback.reject(new Error(JSON.stringify(packet.error))) : callback.resolve(packet.result); }
  });
  socket.addEventListener('close',()=>{ for (const call of calls.values()) { clearTimeout(call.timer); call.reject(new Error('Chrome connection closed')); } calls.clear(); });
  const call = (method, params = {}) => new Promise((resolve, reject) => { const id = ++next; const timer=setTimeout(()=>{calls.delete(id);reject(new Error(`CDP timeout: ${method}`));},method==='Runtime.evaluate'?seconds*1000+20000:10000); calls.set(id, { resolve, reject, timer }); socket.send(JSON.stringify({ id, method, params })); });
  const version = await call('Browser.getVersion');
  await call('Page.navigate', { url: `http://127.0.0.1:${server.address().port}/` });
  await delay(300);
  const result = await call('Runtime.evaluate', { awaitPromise: true, returnByValue: true, expression: `(async () => {
    const module = await import('/link_proof.js');
    await module.default();
    module.start(${JSON.stringify(ticket)}, ${JSON.stringify(proofCase)});
    const events = [];
    const closed_results = [];
    const started = performance.now();
    let last;
    let lastCommand = '', nextControl = 0;
    while (performance.now() - started < ${seconds * 1000}) {
      last = JSON.parse(module.tick());
      for (const event of last.events) events.push({ elapsed_ms: performance.now() - started, event });
      if (last.events.some(e=>e.startsWith('Closed('))) closed_results.push(last);
      if (${Boolean(controlPath)} && performance.now() >= nextControl) {
        nextControl = performance.now() + 50;
        const command = (await (await fetch('/control')).text()).trim();
        if (command !== lastCommand) {
          if (command === 'reconnect' || command === 'close') { module.close(88); closed_results.push(JSON.parse(module.tick())); }
          if (command === 'reconnect') module.start(${JSON.stringify(ticket)}, ${JSON.stringify(proofCase)});
          if (command === 'drain') module.drain_only();
          if (command === 'finish') break;
          lastCommand = command;
        }
      }
      await new Promise(resolve => setTimeout(resolve, 1));
    }
    module.close(77);
    return { result: last, events, closed_results };
  })()` });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  await fs.writeFile(output, JSON.stringify({ ...result.result.value, version, args }, null, 2) + '\n');
} finally {
  await cleanup();
}
