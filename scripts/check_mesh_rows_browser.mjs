// Real hardware WebGPU valley: one fetched mesh edit, then an exact fresh-page oracle.
import fs from 'node:fs/promises';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

export async function prepareMeshRows(files, output, cooker, assetsSource) {
  if (cooker === 'target/debug/thrum-cook') execFileSync('cargo', ['build', '--locked', '--offline', '-p', 'thrum-cook'], { stdio: 'inherit' });
  const source = path.join(output, 'rows-source'), store = path.join(output, 'rows-store');
  await fs.cp(assetsSource, source, { recursive: true });
  execFileSync(cooker, ['cook', source, store], { stdio: 'inherit' });
  const first = await fs.readFile(path.join(store, 'HEAD'));
  const gltfPath = path.join(source, 'rock.gltf');
  const gltf = JSON.parse(await fs.readFile(gltfPath, 'utf8'));
  const accessor = gltf.accessors[gltf.meshes[0].primitives[0].attributes.POSITION];
  const view = gltf.bufferViews[accessor.bufferView];
  if (accessor.componentType !== 5126 || accessor.type !== 'VEC3') throw new Error('Rock positions changed layout');
  const binPath = path.join(source, gltf.buffers[view.buffer].uri);
  const bytes = await fs.readFile(binPath), offset = (view.byteOffset ?? 0) + (accessor.byteOffset ?? 0);
  for (let i = 0; i < accessor.count; i++) {
    const at = offset + i * (view.byteStride ?? 12) + 4;
    bytes.writeFloatLE(bytes.readFloatLE(at) * 2, at);
  }
  accessor.min[1] *= 2; accessor.max[1] *= 2;
  await fs.writeFile(binPath, bytes);
  await fs.writeFile(gltfPath, JSON.stringify(gltf));
  execFileSync(cooker, ['cook', source, store], { stdio: 'inherit' });
  const second = await fs.readFile(path.join(store, 'HEAD'));
  const file = bytes => ({ bytes, type: 'application/octet-stream' });
  files.set('/rows.html', { bytes: Buffer.from('<!doctype html><style>body{margin:0}</style><canvas width="640" height="360"></canvas>'), type: 'text/html' });
  files.set('/rows-route.json', { bytes: await fs.readFile('examples/v1-game/routes/moving-v1.json'), type: 'application/json' });
  files.set('/head', file(first));
  for (const shard of await fs.readdir(path.join(store, 'cas'))) for (const hash of await fs.readdir(path.join(store, 'cas', shard))) {
    files.set(`/cas/${hash}`, file(await fs.readFile(path.join(store, 'cas', shard, hash))));
  }
  let edited = false, counting = false;
  const changedRequests = new Map();
  return {
    receive(request, response) {
      const pathname = new URL(request.url, 'http://localhost').pathname;
      if (pathname === '/advance') {
        files.set('/head', file(second)); edited = counting = true;
        response.writeHead(200).end('advanced'); return true;
      }
      if (counting && pathname.startsWith('/cas/')) changedRequests.set(pathname, (changedRequests.get(pathname) ?? 0) + 1);
      return false;
    },
    async qualify(cdp, origin, output, waitFor) {
      const targets = [];
      async function page() {
        const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' }); targets.push(targetId);
        const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
        await cdp.call('Runtime.enable', {}, sessionId);
        await cdp.call('Page.navigate', { url: `${origin}/rows.html` }, sessionId);
        await waitFor(async () => (await cdp.call('Runtime.evaluate', { expression: `location.pathname==='/rows.html'&&document.readyState==='complete'`, returnByValue: true }, sessionId)).result.value, 15000, 'rows page');
        const evaluate = async expression => {
          const result = await cdp.call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, sessionId);
          if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
          return result.result.value;
        };
        await evaluate(`(async()=>{
          const m=await import('/v1_game.js');await m.default();
          const route=await (await fetch('/rows-route.json')).json();route.render_extent=[640,360];
          window.game=await m.create_route_game(document.querySelector('canvas'),JSON.stringify(route),false,'occlusion',true,false);
          game.set_paused(true,0); window.clock=0;
          window.valley=await m.fetch_cooked_valley(${JSON.stringify(origin)}); game.attach_cooked_valley(valley);
          window.settle=async generation=>{for(let i=0;i<2000;i++){game.frame(clock+=16);await new Promise(r=>setTimeout(r,0));const s=JSON.parse(game.cooked_status());if(s.errors.length)throw Error(JSON.stringify(s));if(s.drawn===generation){if(s.rowGeneration!==generation)throw Error('row and placement generation differ');return s;}}throw Error('row upload timed out');};
          window.image=async()=>{game.begin_capture();for(let i=0;i<2000;i++){const p=game.try_finish_capture();if(p){const hash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',p))).map(v=>v.toString(16).padStart(2,'0')).join('');return {hash,bytes:p.length,identity:game.report()};}await new Promise(r=>setTimeout(r,0));}throw Error('capture timed out');};
        })()`);
        return { evaluate, sessionId };
      }
      try {
        const live = await page();
        const initial = await live.evaluate('(async()=>{await settle(1);return await image();})()');
        await live.evaluate(`(async()=>{await fetch('/advance');await valley.next_publication();await settle(2);})()`);
        const changed = await live.evaluate('image()');
        const rowStatus = await live.evaluate('JSON.parse(game.cooked_status())');
        const drawn = /culling_drawn_p50=(\d+)/.exec(changed.identity);
        if (!drawn || Number(drawn[1]) === 0 || rowStatus.rowWrittenBytes === 0) throw new Error('The valley fixture must draw visible cooked entities and replace their row');
        counting = false;
        if (!edited || changedRequests.size !== 2 || [...changedRequests.values()].some(n => n !== 1)) throw new Error(`Expected one changed product and manifest once: ${JSON.stringify([...changedRequests])}`);
        const fresh = await page();
        const reference = await fresh.evaluate('(async()=>{await settle(2);return await image();})()');
        if (initial.hash === changed.hash) throw new Error('Mesh edit changed no pixel');
        if (changed.hash !== reference.hash || changed.bytes !== reference.bytes) throw new Error(`Edited valley differs from fresh page: ${JSON.stringify({changed,reference})}`);
        for (const [label, page] of [['edited', live], ['fresh', fresh]]) {
          const screenshot = await cdp.call('Page.captureScreenshot', { format: 'png', clip: { x: 0, y: 0, width: 640, height: 360, scale: 1 } }, page.sessionId);
          await fs.writeFile(path.join(output, `mesh-rows-${label}.png`), Buffer.from(screenshot.data, 'base64'));
        }
        const record = { case: 'mesh-rows', target: 'chrome-webgpu', culling: 'Occlusion', overlay: 'the shipped moving-route entity overlay, held at tick zero', paused: true, rowStatus, initial, changed, reference, changedRequests: [...changedRequests] };
        await fs.writeFile(path.join(output, 'mesh-rows.json'), JSON.stringify(record, null, 2));
        return record;
      } finally { for (const targetId of targets) await cdp.call('Target.closeTarget', { targetId }); }
    },
  };
}
