// Fixture and target runner for the lent-products case in check_v1_browser.mjs.
import fs from 'node:fs/promises';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

export async function prepareLent(files, output, cooker) {
  if (cooker === 'target/debug/thrum-cook') execFileSync('cargo', ['build', '--locked', '--offline', '-p', 'thrum-cook'], { stdio: 'inherit' });
  const source = path.join(output, 'lent-source');
  const store = path.join(output, 'lent-store');
  execFileSync('python3', ['scripts/content_loop_fixture.py', source], { stdio: 'inherit' });
  execFileSync(cooker, ['cook', source, store], { stdio: 'inherit' });
  const first = await fs.readFile(path.join(store, 'HEAD'));
  execFileSync('python3', ['scripts/content_loop_fixture.py', source, '--edit'], { stdio: 'inherit' });
  execFileSync(cooker, ['cook', source, store], { stdio: 'inherit' });
  const second = await fs.readFile(path.join(store, 'HEAD'));
  const file = bytes => ({ bytes, type: 'application/octet-stream' });
  files.set('/lent.html', { bytes: Buffer.from('<!doctype html><title>Cooked lease qualification</title>'), type: 'text/html' });
  files.set('/head', file(first));
  for (const shard of await fs.readdir(path.join(store, 'cas'))) {
    for (const hash of await fs.readdir(path.join(store, 'cas', shard))) {
      files.set(`/cas/${hash}`, file(await fs.readFile(path.join(store, 'cas', shard, hash))));
    }
  }
  const changedRequests = new Map();
  let advanced = false;
  return {
    receive(request, response) {
      const pathname = new URL(request.url, 'http://localhost').pathname;
      if (pathname === '/advance') {
        files.set('/head', file(second)); advanced = true;
        response.writeHead(200).end('advanced'); return true;
      }
      if (advanced && pathname.startsWith('/cas/')) changedRequests.set(pathname, (changedRequests.get(pathname) ?? 0) + 1);
      return false;
    },
    async qualify(cdp, origin, output) {
      const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
      const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
      try {
        await cdp.call('Runtime.enable', {}, sessionId);
        await cdp.call('Page.navigate', { url: `${origin}/lent.html` }, sessionId);
        // Run on a blank, controlled page so no ordinary demo RAF competes with this host.
        const r = await cdp.call('Runtime.evaluate', { expression: `(async()=>{const m=await import('/v1_game.js');await m.default();return await m.qualify_lent_products(${JSON.stringify(origin)});})()`, awaitPromise: true, returnByValue: true }, sessionId);
        if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails));
        const exact = JSON.parse(r.result.value);
        // One changed product plus its manifest, each fetched once by the game's client.
        if (changedRequests.size !== 2 || [...changedRequests.values()].some(count => count !== 1)) throw new Error(`Expected one product and one manifest, once each: ${JSON.stringify([...changedRequests])}`);
        const record = { case: 'lent-products', target: 'chrome-webgpu', exact, changedRequests: [...changedRequests] };
        await fs.writeFile(path.join(output, 'lent-products.json'), JSON.stringify(record, null, 2));
        return record;
      } finally { await cdp.call('Target.closeTarget', { targetId }); }
    },
  };
}
