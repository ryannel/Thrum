// The ordinary game page supplies one report for every observed submission.
import fs from 'node:fs/promises';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
export const cloudCases = ['clouds-first-frame', 'clouds-state', 'clouds-placed'];

export function cloudObserver(mode) {
  const ground = mode === 'clouds-state';
  return `
    globalThis.thrumCloudCheck = {enabled: ${mode !== 'clouds-placed'}, ground: ${ground}, frames: [], visibility: [], gpuErrors: []};
    const requestDevice = GPUAdapter.prototype.requestDevice;
    GPUAdapter.prototype.requestDevice = async function(...args) {
      const device = await Reflect.apply(requestDevice, this, args);
      device.addEventListener('uncapturederror', event => {
        thrumCloudCheck.gpuErrors.push(event.error.message);
      });
      return device;
    };
    document.addEventListener('visibilitychange', () => {
      thrumCloudCheck.visibility.push({hidden: document.hidden, time: performance.now()});
    });
  `;
}

export async function preparePlacedClouds(files, output, repo) {
  const source = path.join(output, 'placed-source');
  const store = path.join(output, 'placed-store');
  execFileSync('cargo', ['run', '--locked', '--offline', '--release', '-p', 'v1-game',
    '--example', 'cook_cloud_library', '--', source, store], { cwd: repo, stdio: 'inherit' });
  const file = bytes => ({ bytes, type: 'application/octet-stream' });
  files.set('/head', file(await fs.readFile(path.join(store, 'HEAD'))));
  for (const shard of await fs.readdir(path.join(store, 'cas'))) {
    for (const hash of await fs.readdir(path.join(store, 'cas', shard))) {
      files.set(`/cas/${hash}`, file(await fs.readFile(path.join(store, 'cas', shard, hash))));
    }
  }
}

export async function qualifyClouds({ cdp, sessionId, targetId, mode, poll, game, timeout, waitFor, destroy }) {
  const read = async () => {
    await poll();
    const state = await game.invoke(`return {
      frames: thrumCloudCheck.frames,
      visibility: thrumCloudCheck.visibility,
      gpuErrors: thrumCloudCheck.gpuErrors,
      hidden: document.hidden,
      fault: game.graphics_fault(),
      tick: Number(game.committed_tick()),
      presented: Number(game.presented()),
      report: game.report(),
    };`);
    if (state.gpuErrors.length) throw new Error(`WebGPU errors: ${state.gpuErrors.join('\n')}`);
    return state;
  };
  const deadline = Date.now() + timeout;
  const until = (condition, description) => waitFor(async () => {
    const state = await read();
    return condition(state) ? state : undefined;
  }, Math.max(0, deadline - Date.now()), description);
  if (mode === 'clouds-placed') {
    await game.invoke("game.edit_control('camera.yaw', -65, performance.now()); game.edit_control('camera.pitch', 14, performance.now());");
    const resident = await waitFor(async () => {
      await read();
      const report = await game.invoke("return JSON.parse(game.placed_cloud_report() ?? 'null');");
      return report?.volume_instances > 0 ? report : undefined;
    }, timeout, 'submitted placed-cloud lease');
    for (const name of ['cloud-library', 'cloud-growth']) {
      if (!resident.resources.some(([resource, bytes]) => resource === `${name} R8Unorm` && bytes > 0)) {
        throw new Error(`Portable WebGPU did not resolve ${name} as R8Unorm`);
      }
    }
    await game.invoke('thrumCloudCheck.enabled = true;');
    const visible = await until(value => value.frames.find(row => row.visible_trace_texels > 0), 'visible placed cloud');
    await game.invoke('thrumCloudCheck.enabled = false;');
    const final = await read();
    return { target: 'chrome-webgpu', resident, visible: visible.frames.at(-1), gpuErrors: final.gpuErrors };
  }
  const cleanFrame = row =>
    row.weather_texels[0] > 0 && row.weather_texels[1] > 0 && row.weather_rows.length === 4;
  const initial = await until(value => value.frames.length > 0, 'first submitted cloud frame');
  const first = initial.frames[0];
  if (first.presented !== 1 || first.generation !== 1 || first.tick !== 0 || !cleanFrame(first)) {
    throw new Error(`First submission was not a verified cloud frame: ${JSON.stringify(first)}`);
  }
  if (mode === 'clouds-first-frame') {
    if (!(first.visible_trace_texels > 0)) throw new Error('First submission has no visible cloud trace texels');
    await game.invoke('thrumCloudCheck.hold = true;');
    const final = await until(state => state.frames.length === state.presented, 'first-frame readbacks drained');
    return {first, gpuErrors: final.gpuErrors, scope: 'Visible cloud trace and whole submitted weather map, including exact lattice identities and comparison against the CPU field at the Metal tolerance'};
  }
  const before = await until(({frames}) => frames.length >= 3 && frames.every(row => row.tick === first.tick), 'repeated submitted tick');
  await game.invoke('game.set_paused(false, performance.now());');
  const partial = await until(({frames}) => frames.some(row => row.weather_rows.some(level => level.current_rows > 0 &&
    level.current_rows < 256) && row.ground.reference_texels.some(count => count > 0 && count < 256 * 256)), 'partially refreshed weather and Sun rows');

  const { targetId: other } = await cdp.call('Target.createTarget', { url: 'about:blank', background: true });
  let hidden, resumed;
  try {
    await cdp.call('Target.activateTarget', { targetId: other });
    hidden = await until(state => state.hidden && state.frames.length === state.presented, 'hidden tab with all submissions checked');
    await new Promise(resolve => setTimeout(resolve, 700));
    const held = await read();
    if (!held.hidden || held.presented !== hidden.presented || held.tick !== hidden.tick) {
      throw new Error('Cloud game submitted or ticked while its Chrome tab was hidden');
    }
    await cdp.call('Target.activateTarget', { targetId });
    resumed = await until(value => !value.hidden && value.frames.length > hidden.presented, 'first checked frame after showing the tab');
  } finally {
    await cdp.call('Target.closeTarget', { targetId: other });
  }

  const hitch = await game.invoke(`
    const start = performance.now();
    while (performance.now() - start < 350) { /* block the actual page event loop */ }
    return {start, end: performance.now(), presented: Number(game.presented())};
  `);
  const afterHitch = await until(({frames}) => frames.some(row => row.presented > hitch.presented &&
    row.callback_ms >= hitch.end), 'checked submission after the page hitch');
  await game.invoke('thrumCloudCheck.hold = true;');
  await until(value => value.frames.length === value.presented, 'readbacks drained before device destruction');
  await destroy();
  const lost = await until(value => /^Destroyed:/.test(value.fault ?? '') && value.frames.length === value.presented, 'device lost after checked submissions');
  await game.invoke('thrumCloudCheck.hold = false; thrumCloudCheck.resume();');
  await until(value => value.tick > lost.tick && value.presented === lost.presented, 'world ticks with no lost-device submissions');
  await game.invoke('document.querySelector("#restore-graphics").click();');
  await until(value => !value.fault && value.frames.filter(row => row.generation === 2).length >= 3, 'three checked replacement-device frames');
  await game.invoke('thrumCloudCheck.hold = true;');
  const final = await until(value => value.frames.length === value.presented, 'replacement readbacks drained');
  const firstReplacement = final.frames.find(row => row.generation === 2);
  if ([first, firstReplacement].some(row => !row.ground.reference_texels.every(count => count === 256 * 256)) ||
      firstReplacement.presented !== lost.presented + 1 ||
      [...firstReplacement.weather_rows, ...firstReplacement.ground.rows].some(row => row.oldest_tick !== firstReplacement.tick)) {
    throw new Error('Replacement device did not rebuild every row on its first submission');
  }
  if (final.frames.some((row, i) => row.presented !== i + 1 || !cleanFrame(row) || !row.ground?.shade_texels ||
      (i > 0 && row.tick < final.frames[i - 1].tick))) {
    throw new Error('Missing, clear, or out-of-order cloud submission in the lifecycle record');
  }
  if (!final.visibility.some(edge => edge.hidden) || !final.visibility.some(edge => !edge.hidden)) {
    throw new Error('Chrome tab visibility did not change in both directions');
  }
  return { frames: final.frames, visibility: final.visibility, gpuErrors: final.gpuErrors, repeated_through: before.presented,
    partial_refresh_by: partial.presented, hidden_at: hidden.presented, resumed_at: resumed.presented,
    hitch, after_hitch: afterHitch.presented, lost_at: lost.presented, first_replacement: firstReplacement.presented,
    scope: 'Every observed actual page submission; weather against its CPU row source; every written Sun-path ground slice against an independent march through the verified weather map, retained rows against the last observed write, and every packed ground texel against those slices, with cadence bounds' };
}
