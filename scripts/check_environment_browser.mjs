// Browser-side referees run on the game's production resources. Consumer entry
// points use only the lent layout/library. Fixture inspection stays in this driver.
import fs from 'node:fs/promises';
import path from 'node:path';

function observeEnvironment() {
  const state = globalThis.__environment = { devices: [], groups: [], layouts: [], pipelines: [], pipelineLayouts: [], renderPipelines: [], views: new WeakMap(), buffers: new WeakMap() };
  for (const [method, list] of [['createBindGroup', 'groups'], ['createBindGroupLayout', 'layouts'], ['createComputePipeline', 'pipelines'], ['createPipelineLayout', 'pipelineLayouts'], ['createRenderPipeline', 'renderPipelines']]) {
    const original = GPUDevice.prototype[method];
    GPUDevice.prototype[method] = function(descriptor) {
      const value = original.call(this, descriptor);
      state[list].push({ value, descriptor, device: this });
      return value;
    };
  }
  const view = GPUTexture.prototype.createView;
  GPUTexture.prototype.createView = function(descriptor) {
    const value = view.call(this, descriptor);
    state.views.set(value, { texture: this, descriptor });
    return value;
  };
  const write = GPUQueue.prototype.writeBuffer;
  GPUQueue.prototype.writeBuffer = function(buffer, offset, data, dataOffset = 0, size) {
    const bytes = ArrayBuffer.isView(data) ? new Uint8Array(data.buffer, data.byteOffset, data.byteLength) : new Uint8Array(data);
    const elementBytes = data.BYTES_PER_ELEMENT ?? 1;
    const source = bytes.subarray(dataOffset * elementBytes, size === undefined ? undefined : (dataOffset + size) * elementBytes);
    let record = state.buffers.get(buffer);
    if (!record) { record = new Uint8Array(buffer.size); state.buffers.set(buffer, record); }
    record.set(source, offset);
    return write.call(this, buffer, offset, data, dataOffset, size);
  };
}

export async function qualifyEnvironment(cdp, origin, output, timeout, waitFor, existingGame) {
  const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
  const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
  try {
    await cdp.call('Page.enable', {}, sessionId);
    await cdp.call('Runtime.enable', {}, sessionId);
    await cdp.call('Page.addScriptToEvaluateOnNewDocument', { source: `(${observeEnvironment})();` }, sessionId);
    await cdp.call('Page.navigate', { url: `${origin}/` }, sessionId);
    const evaluate = async expression => {
      const result = await cdp.call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, sessionId);
      if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
      return result.result.value;
    };
    await waitFor(async () => evaluate(`/^Running/.test(document.querySelector('#status')?.textContent ?? '')`), timeout, 'environment game startup');
    const game = await existingGame(cdp, sessionId);
    await game.invoke('game.set_visible(false,performance.now());return true;');
    await waitFor(async () => {
      try { return await game.invoke('globalThis.__environmentLibrary=game.environment_library();return true;'); }
      catch (error) { if (!String(error).includes('no frames in flight')) throw error; return false; }
    }, timeout, 'environment contribution after frame drain');
    const source = await fs.readFile(new URL('./environment_referee.mjs', import.meta.url), 'utf8');
    const report = await evaluate(`(async()=>{${source}\nreturn await environmentReferee();})()`);
    await fs.writeFile(path.join(output, 'environment.json'), JSON.stringify(report, null, 2));
    for (const row of ['H1', 'H2', 'H3', 'H4', 'H5']) {
      if (report.rows?.[row]?.status !== 'pass' || !(report.rows[row].queries > 0)) throw new Error(`Environment ${row} missing, unresolved or failed`);
    }
    return { case: 'environment', ...report };
  } finally {
    await cdp.call('Target.closeTarget', { targetId });
  }
}

// Byte-identical engine tap captures in one browser process, on one cooked store.
export async function qualifyEnvironmentParity(cdp, currentOrigin, output, timeout, assetsOrigin, baseFiles, serve, waitFor, existingGame) {
  const baseServing = await serve(baseFiles);
  const records = [];
  const capture = async (origin, arm, preset, negative = false) => {
    const { targetId } = await cdp.call('Target.createTarget', { url: `${origin}/?assets=${encodeURIComponent(assetsOrigin)}&params=${preset}` });
    const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
    try {
      await cdp.call('Runtime.enable', {}, sessionId);
      await waitFor(async () => {
        const result = await cdp.call('Runtime.evaluate', { expression: `/^Running/.test(document.querySelector('#status')?.textContent ?? '')`, returnByValue: true }, sessionId);
        return result.result.value;
      }, timeout, `${arm} ${preset} startup`);
      const game = await existingGame(cdp, sessionId);
      await waitFor(async () => game.invoke('const value=JSON.parse(game.cooked_status());return value?.drawn>=1;'), timeout, `${arm} cooked content`);
      if (negative) await game.invoke("game.set_camera_yaw((document.querySelector('#camera-yaw').valueAsNumber+3)*Math.PI/180);return true;");
      const identity = await game.invoke('return game.begin_capture();');
      const length = await waitFor(async () => game.invoke('const pixels=game.try_finish_capture();if(!pixels)return undefined;globalThis.__environmentPixels=pixels;return pixels.length;'), timeout, `${arm} engine tap`);
      const chunks = [];
      for (let offset = 0; offset < length; offset += 1 << 18) {
        chunks.push(Buffer.from(await game.invoke(`const part=globalThis.__environmentPixels.subarray(args[0],args[0]+${1 << 18});let text='';for(let i=0;i<part.length;i+=0x8000)text+=String.fromCharCode.apply(null,part.subarray(i,i+0x8000));return btoa(text);`, [offset]), 'base64'));
      }
      const bytes = Buffer.concat(chunks);
      if (bytes.length !== length || !length) throw new Error('Incomplete engine tap');
      const file = `${arm}-${preset}.rgba`;
      await fs.writeFile(path.join(output, file), bytes);
      records.push({ arm, preset, identity, file, bytes: bytes.length });
      return bytes;
    } finally { await cdp.call('Target.closeTarget', { targetId }); }
  };
  try {
    const presets = [...baseFiles.keys()].filter(name => /^\/showcase-.*\.params\.json$/.test(name)).sort();
    if (presets.length !== 6) throw new Error('Parity requires the six route stops');
    for (const name of presets) {
      const preset = name.slice(1);
      const base = await capture(baseServing.origin, 'base', preset);
      const claim = await capture(currentOrigin, 'claim', preset);
      const rerun = await capture(currentOrigin, 'rerun', preset);
      if (!base.equals(claim) || !claim.equals(rerun)) throw new Error(`No-provider parity differs at ${preset}`);
    }
    const name = presets.find(name => name.includes('noon')).slice(1);
    const unchanged = await capture(currentOrigin, 'negative-base', name);
    const changed = await capture(currentOrigin, 'negative-view', name, true);
    if (unchanged.equals(changed)) throw new Error('Wrong-view negative control incorrectly passed');
    const result = { case: 'environment-off-parity', target: 'chrome-webgpu', records, same_build_control: true, wrong_view_negative_rejected: true, comparison: 'Full raw engine-tap RGBA8 bytes, zero tolerance or mask, one Chrome process and cooked store' };
    await fs.writeFile(path.join(output, 'environment-parity.json'), JSON.stringify(result, null, 2));
    return result;
  } finally {
    await new Promise(resolve => baseServing.server.close(resolve));
  }
}

export async function qualifyEnvironmentCost(cdp, origin, output, timeout, assetsOrigin, waitFor, existingGame) {
  const arms = [];
  for (const [order, provider] of ['off', 'on', 'on', 'off'].entries()) {
    const { targetId } = await cdp.call('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.call('Target.attachToTarget', { targetId, flatten: true });
    const evaluate = async expression => {
      const result = await cdp.call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }, sessionId);
      if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
      return result.result.value;
    };
    try {
      await cdp.call('Page.enable', {}, sessionId);
      await cdp.call('Runtime.enable', {}, sessionId);
      await cdp.call('Page.addScriptToEvaluateOnNewDocument', { source: `(${observeEnvironment})();` }, sessionId);
      await cdp.call('Page.navigate', { url: `${origin}/?assets=${encodeURIComponent(assetsOrigin)}&params=showcase-noon.params.json${provider === 'on' ? '&diagnostic-clouds' : ''}` }, sessionId);
      await waitFor(async () => evaluate(`/^Running/.test(document.querySelector('#status')?.textContent ?? '')`), timeout, 'cost game startup');
      const game = await existingGame(cdp, sessionId);
      await waitFor(async () => game.invoke('return JSON.parse(game.cooked_status())?.drawn>=1;'), timeout, 'cost cooked content');
      const intervals = await evaluate(`new Promise(resolve=>{const values=[];let start,previous;function sample(now){if(start===undefined)start=now;if(previous!==undefined)values.push(now-previous);previous=now;if(now-start>=4000)resolve(values);else requestAnimationFrame(sample);}requestAnimationFrame(sample);})`);
      const pipelines = await evaluate(`(()=>{const s=globalThis.__environment;return s.renderPipelines.filter(p=>p.descriptor.fragment&&(p.descriptor.label?.startsWith('prepared opaque')||['cooked mesh','cooked volume'].includes(p.descriptor.label))).map(p=>{const layout=s.pipelineLayouts.find(item=>item.value===p.descriptor.layout);if(!layout)throw new Error('Missing pipeline layout for '+p.descriptor.label);const sampled=layout.descriptor.bindGroupLayouts.reduce((sum,value)=>{if(!value)return sum;const group=s.layouts.find(item=>item.value===value);if(!group)throw new Error('Missing bind-group layout');return sum+group.descriptor.entries.filter(entry=>entry.texture).length;},0);return {name:p.descriptor.label,sampled,limit:p.device.limits.maxSampledTexturesPerShaderStage,created:true};});})()`);
      for (const pipeline of pipelines) if (!(pipeline.sampled <= pipeline.limit)) throw new Error(`Texture limit exceeded ${JSON.stringify(pipeline)}`);
      if (!pipelines.some(p => p.name === 'cooked volume') || !pipelines.some(p => p.name === 'prepared opaque synthetic role') || !pipelines.some(p => p.name.includes('cooked') && p.name !== 'cooked volume')) throw new Error('Missing forward pipeline census');
      arms.push({ order, provider, whole_frame_raf_intervals_ms: intervals, report: await game.read(), pipelines });
    } finally { await cdp.call('Target.closeTarget', { targetId }); }
  }
  const result = { case: 'environment-cost', target: 'chrome-webgpu', arms, load_class: 'shared GPU lock held, one active owned headless page', scope: 'Order-alternated whole-frame browser RAF interval observations. No display or performance gate.' };
  await fs.writeFile(path.join(output, 'environment-cost.json'), JSON.stringify(result, null, 2));
  return result;
}
