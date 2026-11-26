// Drive the packaged Crowd page through Chrome's real input and submitted-image tap.
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

const [url, folder, profile] = process.argv.slice(2);
const chrome = spawn(process.env.CHROME_BIN ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
  '--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-background-networking',
  '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank',
], { stdio: ['ignore', 'ignore', 'inherit'] });
const exit = new Promise(resolve => chrome.once('exit', resolve));
let socket;
let send;

function require(condition, message) {
  if (!condition) throw new Error(message);
}

async function until(read, accept, description, timeout = 90000) {
  const deadline = performance.now() + timeout;
  do {
    const value = await read();
    if (accept(value)) return value;
    await sleep(100);
  } while (performance.now() < deadline);
  throw new Error(`Timed out waiting for ${description}`);
}

try {
  const port = await until(async () => {
    try { return Number((await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]); }
    catch (error) { if (error.code !== 'ENOENT') throw error; return 0; }
  }, Boolean, 'Chrome debugging port');
  const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  socket = new WebSocket(targets.find(target => target.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.onopen = resolve;
    socket.onerror = reject;
  });
  const pending = new Map();
  let sequence = 0;
  socket.onmessage = ({ data }) => {
    const message = JSON.parse(data);
    const request = pending.get(message.id);
    if (!request) return;
    pending.delete(message.id);
    clearTimeout(request.timeout);
    if (message.error) request.reject(new Error(JSON.stringify(message.error)));
    else request.resolve(message.result);
  };
  send = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++sequence;
    const timeout = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`Chrome timed out on ${method}`));
    }, 45000);
    pending.set(id, { resolve, reject, timeout });
    socket.send(JSON.stringify({ id, method, params }));
  });
  const evaluate = async expression => {
    const reply = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    if (reply.exceptionDetails) throw new Error(reply.result.description ?? reply.exceptionDetails.text);
    return reply.result.value;
  };
  const read = () => evaluate(`(() => {
    const problem = document.querySelector('#problems')?.textContent;
    const status = document.querySelector('#status')?.textContent;
    if (problem || status?.startsWith('Scene could not start')) throw Error(problem || status);
    return window.game ? JSON.parse(game.witness()) : null;
  })()`);
  const moved = (a, b) => a[0] !== b[0] || a[1] !== b[1];
  const results = [];
  await send('Page.enable');
  for (const scale of [1, 2]) {
    await send('Emulation.setTouchEmulationEnabled', { enabled: false });
    await send('Emulation.setDeviceMetricsOverride', {
      width: 860, height: 640, deviceScaleFactor: scale, mobile: false,
    });
    await send('Page.navigate', { url: `${url}?density=${scale}` });
    const initial = await until(read, row => row?.drawn?.scale === scale && row.tick >= 2, 'presented status display');
    require(initial.live === initial.rows && initial.live >= 2000, 'missing live Crowd sprite rows');
    const hardware = await evaluate(`(async () => {
      const adapter = await navigator.gpu.requestAdapter({ powerPreference: 'high-performance' });
      if (!adapter) return null;
      return { vendor: adapter.info.vendor, architecture: adapter.info.architecture, fallback: adapter.info.isFallbackAdapter };
    })()`);
    require(hardware && !hardware.fallback, 'hardware WebGPU is unavailable');
    const box = await evaluate(`(() => {
      const canvas = document.querySelector('canvas');
      const r = canvas.getBoundingClientRect();
      return { x: r.left, y: r.top, width: r.width, height: r.height, backing: [canvas.width, canvas.height], density: devicePixelRatio };
    })()`);
    require(box.density === scale && box.backing.every((n, i) => n === Math.round((i ? box.height : box.width) * scale))
      && initial.drawn.extent.every((n, i) => n === box.backing[i]), 'canvas did not present at the requested density');
    const x = Math.round(box.x + box.width * 0.45);
    const y = Math.round(box.y + box.height * 0.62);
    const key = { code: 'ArrowRight', key: 'ArrowRight', windowsVirtualKeyCode: 39 };
    await send('Input.dispatchKeyEvent', { ...key, type: 'keyDown' });
    await send('Input.dispatchKeyEvent', { ...key, type: 'keyUp' });
    const keyboard = await until(read, row => row.presses[0] > initial.presses[0] && moved(initial.pan, row.pan), 'key-routed camera');
    await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 });
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: x + 70, y, button: 'none', buttons: 1 });
    await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: x + 70, y, button: 'left', buttons: 0, clickCount: 1 });
    const mouse = await until(read, row => row.presses[1] > keyboard.presses[1] && moved(keyboard.pan, row.pan), 'mouse-routed camera');
    await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 1 });
    await send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y, id: 0 }] });
    await send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x, y: y - 55, id: 0 }] });
    await send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    const touch = await until(read, row => row.presses[2] > mouse.presses[2] && moved(mouse.pan, row.pan), 'touch-routed camera');

    await evaluate('game.begin_capture()');
    const capture = await until(() => evaluate(`(() => {
      let capture = game.next_capture();
      if (!capture) return null;
      window.crowdCapture = capture[1];
      return JSON.parse(capture[0]);
    })()`), Boolean, 'submitted engine image');
    require(capture.presented && capture.scale === scale &&
      capture.extent.every((side, i) => side === box.backing[i]),
      'capture did not come from the presented interface at the requested density');
    const image = Buffer.alloc(capture.extent[0] * capture.extent[1] * 4);
    for (let offset = 0; offset < image.length; offset += 65536) {
      const data = await evaluate(`btoa(String.fromCharCode(...crowdCapture.subarray(${offset}, ${offset + 65536})))`);
      const bytes = Buffer.from(data, 'base64');
      require(bytes.length === Math.min(65536, image.length - offset), 'short engine capture');
      bytes.copy(image, offset);
    }
    const raw = `dpr-${scale}.rgba`;
    await fs.writeFile(path.join(folder, raw), image);
    results.push({
      scale, hardware, canvas: box, initial, keyboard, mouse, touch, capture, raw,
      rgba_sha256: createHash('sha256').update(image).digest('hex'),
    });
  }
  await fs.writeFile(path.join(folder, 'browser.json'), JSON.stringify(results, null, 2) + '\n');
} finally {
  if (socket?.readyState === WebSocket.OPEN) void send('Browser.close').catch(() => {});
  if (await Promise.race([exit.then(() => true), sleep(3000, false)]) === false) {
    chrome.kill('SIGKILL');
    await exit;
  }
  socket?.close();
}
