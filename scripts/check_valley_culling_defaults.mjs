// Exercise the shipped page: scene selection must be independent of star visibility.
// Run after web/build.sh and a release thrum-cook build, under the shared GPU and Chrome locks.
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn, execFileSync } from 'node:child_process';
import { Cdp, assets, serve, serveCooked, hardware, waitFor, cleanup, existingGame } from './check_v1_browser.mjs';

const output = path.resolve(process.argv[2] ?? 'tmp/culling-m3/default-selection');
await fs.mkdir(output, { recursive: true });
const profile = await fs.mkdtemp(path.join(output, 'chrome-'));
let child, cdp, server, cook, ended = false;
const result = { source_revision: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), cases: [] };
try {
  const files = await assets(path.resolve('target/v1-game-web'));
  const serving = await serve(files); server = serving.server;
  const cooked = await serveCooked({ cook: 'target/release/thrum-cook', 'assets-source': 'examples/v1-game/assets/showcase' }, output); cook = cooked.cook;
  child = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
    `--user-data-dir=${profile}`, '--remote-debugging-port=0', '--headless=new',
    '--no-first-run', '--no-default-browser-check', 'about:blank',
  ], { stdio: 'ignore' });
  child.once('exit', () => ended = true);
  const endpoint = await waitFor(async () => {
    try { return (await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n'); } catch {}
  }, 15000, 'CDP');
  const socket = new WebSocket(`ws://127.0.0.1:${endpoint[0]}${endpoint[1]}`);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  cdp = new Cdp(socket);
  result.browser = await cdp.call('Browser.getVersion');
  result.gpu = await cdp.call('SystemInfo.getInfo'); hardware(result.gpu);
  for (const valley of [true, false]) {
    for (const stars of ['on', 'off']) {
      for (const request of [undefined, 'default', 'direct', 'frustum', 'occlusion']) {
        const query = new URLSearchParams({ stars });
        if (valley) query.set('assets', cooked.origin);
        if (request) query.set('culling', request);
        const expected = request === 'direct' ? 'Direct' : request === 'frustum' ? 'Frustum'
          : request === 'occlusion' ? 'Occlusion' : valley ? 'Frustum' : 'Direct';
        const reason = expected === 'Direct' ? 'RequestedDirect' : 'Supported';
        const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
        const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
        await cdp.call('Page.navigate', { url: `${serving.origin}/?${query}` }, sessionId);
        await waitFor(async () => {
          const response = await cdp.call('Runtime.evaluate', {
            expression: "document.querySelector('#status')?.textContent", returnByValue: true,
          }, sessionId);
          const value = response.result.value ?? '';
          if (/fault|error/i.test(value)) throw Error(value);
          return /^Running/.test(value);
        }, 90000, 'shipped page startup');
        const game = await existingGame(cdp, sessionId);
        const status = await waitFor(
          () => game.invoke('return game.presented() >= 600n ? game.report() : null;'),
          90000, 'shipped page completed 600 presentations',
        );
        if (!status.includes(`culling=${expected};`) || !status.includes(`culling_reason=${reason}`)) {
          throw Error(`valley=${valley} stars=${stars} request=${request}: ${status}`);
        }
        result.cases.push({ valley, stars, request: request ?? 'omitted', expected, reason, status });
        console.log(`PASS valley=${valley} stars=${stars} request=${request ?? 'omitted'}: ${expected}/${reason}`);
        await cdp.call('Target.closeTarget', { targetId });
      }
    }
  }
  result.passed = true;
} finally {
  await fs.writeFile(path.join(output, 'result.json'), JSON.stringify(result, null, 2) + '\n');
  if (cook) cook.kill('SIGTERM');
  await cleanup(child, () => ended, cdp, server, profile);
}
