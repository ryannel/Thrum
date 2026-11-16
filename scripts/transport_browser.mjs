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
    if (req.url === '/progress') {
      let body = '';
      for await (const chunk of req) { body += chunk; if (body.length > 16384) throw new Error('progress too large'); }
      await fs.writeFile(output+'.progress.json',body);
      res.end('ok'); return;
    }
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
function signalChrome(signal) {
  try { process.kill(-chrome.pid,signal); }
  catch (error) {
    if (error.code === 'EPERM') chrome.kill(signal);
    else if (error.code !== 'ESRCH') throw error;
  }
}
let cleaning;
function cleanup() {
  return cleaning ??= (async () => {
    socket?.close();
    server.closeAllConnections();
    server.close();
    if (chrome.exitCode === null && chrome.signalCode === null) {
      const exited = new Promise(resolve=>chrome.once('exit',resolve));
      signalChrome('SIGTERM');
      await Promise.race([exited,delay(1000)]);
      if (chrome.exitCode === null && chrome.signalCode === null) {
        signalChrome('SIGKILL');
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
  if (proofCase === 'suspend') {
    await fs.writeFile(output, JSON.stringify({ ...(await suspend(port, tabs.find(tab => tab.type === 'page'))), version, args }, null, 2) + '\n');
    process.exitCode = 0;
  } else {
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
      if (last.events.includes('Open')) await fetch('/progress',{method:'POST',body:JSON.stringify({elapsed_ms:performance.now()-started,id:last.id,events:last.events})});
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
    const closed_after_drain = JSON.parse(module.tick());
    return { result: last, events, closed_results, closed_after_drain };
  })()` });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  await fs.writeFile(output, JSON.stringify({ ...result.result.value, version, args }, null, 2) + '\n');
  }
} finally {
  await cleanup();
}

// M2.6: a page reloaded, then a tab closed, each without closing its link, then a renderer frozen past the idle
// timeout. The driver records each step's wall-clock time; the harness compares them with the host's reports.
async function suspend(port, firstTab) {
  const steps = [];
  const pages = [];
  const page = `http://127.0.0.1:${server.address().port}/`;
  const session = async url => {
    const ws = new WebSocket(url);
    await new Promise((resolve, reject) => { ws.addEventListener('open', resolve, { once: true }); ws.addEventListener('error', reject, { once: true }); });
    let id = 0;
    const pending = new Map();
    ws.addEventListener('message', event => {
      const packet = JSON.parse(event.data);
      const callback = pending.get(packet.id);
      if (callback) { pending.delete(packet.id); packet.error ? callback.reject(new Error(JSON.stringify(packet.error))) : callback.resolve(packet.result); }
    });
    const send = (method, params = {}) => new Promise((resolve, reject) => {
      const key = ++id;
      const timer = setTimeout(() => { pending.delete(key); reject(new Error(`CDP timeout: ${method}`)); }, 20000);
      pending.set(key, { resolve: v => { clearTimeout(timer); resolve(v); }, reject: e => { clearTimeout(timer); reject(e); } });
      ws.send(JSON.stringify({ id: key, method, params }));
    });
    return { send, close: () => ws.close() };
  };
  const startLink = async tab => {
    await tab.send('Page.navigate', { url: page });
    await delay(300);
    await tab.send('Runtime.evaluate', { awaitPromise: true, expression: `(async () => {
      const module = await import('/link_proof.js');
      await module.default();
      module.start(${JSON.stringify(ticket)}, 'suspend');
      window.__proof = { events: [], last: null };
      setInterval(() => {
        try {
          const report = JSON.parse(module.tick());
          for (const event of report.events) window.__proof.events.push({ unix_ms: Date.now(), event });
          window.__proof.last = report;
        } catch (error) { window.__proof.error = String(error); }
      }, 1);
      return true;
    })()` });
    for (let i = 0; i < 150; ++i) {
      const state = await snapshot(tab);
      if (state?.events.some(e => e.event === 'Open')) return state;
      if (state?.error) throw new Error(state.error);
      await delay(100);
    }
    throw new Error('page link did not open');
  };
  const snapshot = async tab => JSON.parse((await tab.send('Runtime.evaluate', { returnByValue: true, expression: 'JSON.stringify(window.__proof ?? null)' })).result.value);
  let tab = await session(firstTab.webSocketDebuggerUrl);
  let target = firstTab.id;
  await startLink(tab);
  await delay(3000);
  // A reload: the document goes, the link is never closed by the game.
  pages.push({ before: 'reload', state: await snapshot(tab) });
  steps.push({ step: 'reload', unix_ms: Date.now() });
  await tab.send('Page.reload', {});
  await delay(500);
  const reloaded = await startLink(tab);
  steps.push({ step: 'reloaded-open', unix_ms: reloaded.events.find(e => e.event === 'Open').unix_ms });
  await delay(3000);
  // A closed tab: a new tab opens the page again with the same ticket.
  pages.push({ before: 'close-tab', state: await snapshot(tab) });
  const fresh = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' })).json();
  steps.push({ step: 'close-tab', unix_ms: Date.now() });
  await fetch(`http://127.0.0.1:${port}/json/close/${target}`);
  tab.close();
  tab = await session(fresh.webSocketDebuggerUrl);
  target = fresh.id;
  const reopened = await startLink(tab);
  steps.push({ step: 'new-tab-open', unix_ms: reopened.events.find(e => e.event === 'Open').unix_ms });
  await delay(3000);
  // A frozen renderer: every renderer process stopped for the idle timeout and three seconds more.
  const browserUrl = (await (await fetch(`http://127.0.0.1:${port}/json/version`)).json()).webSocketDebuggerUrl;
  const browser = await session(browserUrl);
  const info = await browser.send('SystemInfo.getProcessInfo');
  const pids = info.processInfo.filter(p => p.type === 'renderer').map(p => p.id);
  const beforeFreeze = await snapshot(tab);
  steps.push({ step: 'freeze', unix_ms: Date.now(), pids });
  for (const pid of pids) process.kill(pid, 'SIGSTOP');
  await delay(7000);
  for (const pid of pids) process.kill(pid, 'SIGCONT');
  steps.push({ step: 'resume', unix_ms: Date.now() });
  await delay(3000);
  const afterResume = await snapshot(tab);
  pages.push({ before: 'end', frozen_link_before_freeze: beforeFreeze?.last, state: afterResume });
  browser.close();
  tab.close();
  return { steps, pages };
}
