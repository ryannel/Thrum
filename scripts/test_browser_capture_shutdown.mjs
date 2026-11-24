import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs/promises';
import http from 'node:http';
import { createConnection } from 'node:net';
import { createInterface } from 'node:readline';
import path from 'node:path';
import { test } from 'node:test';
import { setTimeout as sleep } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { finalizeBrowserCapture, openLauncherControl } from './browser_capture_shutdown.mjs';

const fixture = fileURLToPath(import.meta.url);
const start = (role, output, mode, ...args) => spawn(process.execPath, [fixture, '--fixture', role, output, mode, ...args], { stdio: ['ignore', 'ignore', 'pipe'] });
const readLauncherReceipt = async output => JSON.parse(await read(path.join(output, 'launcher.json')) ?? 'null');
const disconnectCases = ['disconnected', 'disconnect-during-validation', 'disconnect-after-close'];
const respondsToClose = ['ordinary', 'status-2', 'missing-preflight', 'granted', 'missing-receipt',
  'request-write', 'manifest-write', 'both-writes', 'receipt-write'];

async function read(file) {
  try { return await fs.readFile(file, 'utf8'); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
}

async function until(check) {
  const deadline = performance.now() + 5000;
  while (performance.now() < deadline) {
    const result = await check();
    if (result) return result;
    await sleep(10);
  }
  throw new Error('Fixture did not reach its synchronization point');
}

async function processFixture(role, output, mode) {
  const ready = path.join(output, role + '-ready');
  const close = path.join(output, 'cdp-close');
  if (mode === 'kill' || mode === 'unconfirmed') process.on('SIGTERM', () => {});
  await fs.writeFile(ready, String(process.pid));
  if (role === 'browser' && respondsToClose.includes(mode)) {
    await until(() => read(close));
    process.exitCode = mode === 'status-2' ? 2 : 0;
  } else if (role === 'browser' && mode === 'early-exit') {
    process.exitCode = 2;
  } else {
    // The supervisor drives the signal cases through real child exits.
    await sleep(10000);
  }
}

async function supervisorFixture(output, mode, port, token) {
  const socket = createConnection({host: '127.0.0.1', port: Number(port)});
  socket.on('error', () => socket.destroy());
  const lines = createInterface({ input: socket });
  const phases = [];
  lines.on('line', phase => phases.push(phase));
  socket.write(token + '\n');
  await until(() => phases.shift() === 'start');
  const browser = start('browser', output, mode);
  const helper = start('helper', output, mode);
  const browserExit = once(browser, 'exit');
  const helperExit = once(helper, 'exit');
  const report = { chrome_pid: browser.pid, screen_recording_before: false, shutdown_requested: false };
  const save = async () => {
    const data = JSON.stringify(report);
    if (mode !== 'receipt-write') {
      const next = path.join(output, 'launcher.next');
      await fs.writeFile(next, data);
      await fs.rename(next, path.join(output, 'launcher.json'));
    }
    if (!socket.destroyed) socket.write(data + '\n');
  };
  let exited = false;
  void browserExit.then(() => { exited = true; });
  try {
    await until(async () => await read(path.join(output, 'browser-ready')) && await read(path.join(output, 'helper-ready')));
    if (mode === 'early-exit') await browserExit;
    report.chrome_exited = exited;
    await save();
    const escalation = [];
    while (!exited) {
      const command = phases.shift();
      const phase = socket.destroyed && disconnectCases.includes(mode) ? 'term' : command;
      if (phase) {
        if (command === 'close') report.shutdown_requested = true;
        if (['term', 'kill'].includes(phase) && !escalation.includes(phase)) {
          escalation.push(phase);
          if (mode !== 'unconfirmed') {
            const signal = phase === 'term' ? 'SIGTERM' : 'SIGKILL';
            browser.kill(signal);
            helper.kill(signal);
          }
          report.shutdown_escalation = escalation;
        }
        await save();
      }
      if (mode === 'unconfirmed' && await read(path.join(output, 'release-fixture'))) break;
      await sleep(15);
    }
    if (mode === 'unconfirmed') return;
    const [code, signal] = await browserExit;
    helper.kill('SIGKILL');
    await helperExit;
    report.chrome_exited = true;
    report.owned_processes_gone = true;
    await fs.rm(path.join(output, 'profile'), { recursive: true });
    report.profile_removed = true;
    await save();
    if (mode === 'missing-receipt') return;
    if (mode === 'status-2') {
      // Hold the final receipt past the grace period with the wrapper gone.
      // The production finalizer must wait without escalating against dead children.
      await sleep(1000);
    }
    if (mode !== 'missing-preflight') report.screen_recording_after = mode === 'granted';
    if (signal) report.chrome_exit_signal = { SIGTERM: 15, SIGKILL: 9 }[signal];
    else report.chrome_exit_code = code;
    report.chrome_final = true;
    await save();
    await fs.writeFile(path.join(output, 'control-phases'), JSON.stringify(phases));
  } finally {
    socket.end();
    browser.kill('SIGKILL');
    helper.kill('SIGKILL');
    await Promise.all([browserExit, helperExit]);
    await fs.writeFile(path.join(output, 'supervisor-exited'), 'yes');
  }
}

async function commandFixture(output, mode) {
  const { closeBrowserConnections } = await import('./check_v1_browser.mjs');
  const profile = path.join(output, 'profile');
  await fs.mkdir(profile);
  const control = await openLauncherControl();
  const wrapper = spawn(process.execPath, ['-e', 'process.exit(0)']);
  const supervisor = start('supervisor', output, mode, String(control.port), control.token);
  supervisor.stderr.unref();
  supervisor.unref();
  assert.deepEqual(await once(wrapper, 'exit'), [0, null]);
  await fs.writeFile(path.join(output, 'wrapper-exited'), 'yes');
  await until(() => control.receipt?.chrome_pid);
  const server = http.createServer();
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  // This is the production finalization boundary, after capture validation and sync.
  const beforeDevtools = ['before-devtools', 'capture-error-and-cleanup-error'].includes(mode);
  const captureError = beforeDevtools ? new Error('capture broke before DevTools')
    : ['request-write', 'manifest-write', 'both-writes', 'disconnected'].includes(mode)
      ? new Error('capture sink failed during frame validation') : undefined;
  const frame = await fs.open(path.join(output, 'validated-frame'), 'wx');
  try {
    await frame.writeFile('validated bytes');
    if (mode === 'disconnect-during-validation') {
      control.close();
      await until(async () => (await readLauncherReceipt(output))?.chrome_final);
    }
    assert.equal(await read(path.join(output, 'validated-frame')), 'validated bytes');
    await frame.sync();
  } finally { await frame.close(); }
  const cdp = beforeDevtools ? undefined : {
    call(method) {
      assert.equal(method, 'Browser.close');
      if (mode === 'disconnect-after-close') control.close();
      void fs.writeFile(path.join(output, 'cdp-close'), 'yes');
      // Chrome can disconnect without a reply. Deliberately never settle.
      return new Promise(() => {});
    },
  };
  const writeFile = fs.writeFile;
  fs.writeFile = async (file, ...args) => {
    if ((['request-write', 'both-writes'].includes(mode) && path.basename(file) === 'request.next') ||
        (['manifest-write', 'both-writes'].includes(mode) && path.basename(file) === 'manifest.json')) {
      throw Object.assign(new Error('ENOSPC writing ' + path.basename(file)), { code: 'ENOSPC' });
    }
    return writeFile(file, ...args);
  };
  if (mode === 'disconnected') control.close();
  const requests = [];
  const request = control.request.bind(control);
  control.request = phase => { requests.push(phase); request(phase); };
  try {
    await finalizeBrowserCapture({
      output, control, cdp, profile, manifest: { complete: !captureError },
      captureError: () => captureError,
      closeConnections: async () => {
        await closeBrowserConnections(undefined, server);
        await fs.writeFile(path.join(output, 'http-closed'), 'yes');
        if (mode === 'capture-error-and-cleanup-error') throw new Error('injected cleanup failure');
      },
      graceMs: mode === 'status-2' ? 600 : respondsToClose.includes(mode) ? 2000 : 300,
      termMs: mode === 'kill' ? 600 : 1200, killMs: 400, finalMs: 2500,
    });
  } catch (error) {
    if (captureError) assert.equal(error, captureError);
    throw error;
  } finally {
    fs.writeFile = writeFile;
    control.close();
    await fs.writeFile(path.join(output, 'harness-requests'), JSON.stringify(requests));
  }
}

const [, , flag, role, output, mode, ...args] = process.argv;
if (flag === '--fixture') {
  try {
    if (role === 'command') await commandFixture(output, mode);
    else if (role === 'supervisor') await supervisorFixture(output, mode, ...args);
    else await processFixture(role, output, mode);
  } catch (error) {
    await fs.writeFile(path.join(output, role + '-error'), String(error));
    process.exitCode = 1;
  }
} else {
  const cases = [
    { mode: 'ordinary', code: 0, escalation: [] },
    { mode: 'term', signal: 15, escalation: ['term'] },
    { mode: 'kill', signal: 9, escalation: ['term', 'kill'] },
    { mode: 'status-2', code: 2, escalation: [] },
    { mode: 'early-exit', code: 2, error: /exited before requested shutdown/ },
    { mode: 'missing-preflight', code: 0, error: /both denied Screen Recording preflights/ },
    { mode: 'granted', code: 0, error: /both denied Screen Recording preflights/ },
    { mode: 'missing-receipt', error: /final launcher receipt could not be confirmed/ },
    { mode: 'unconfirmed', error: /exit and final launcher receipt could not be confirmed/ },
    { mode: 'before-devtools', signal: 15, error: /capture broke before DevTools/ },
    { mode: 'capture-error-and-cleanup-error', signal: 15, error: /capture broke before DevTools/ },
    { mode: 'request-write', code: 0, error: /capture sink failed during frame validation/ },
    { mode: 'manifest-write', code: 0, error: /capture sink failed during frame validation/, noManifest: true },
    { mode: 'both-writes', code: 0, error: /capture sink failed during frame validation/, noManifest: true },
    { mode: 'receipt-write', code: 0, escalation: [] },
    { mode: 'disconnected', signal: 15, error: /capture sink failed during frame validation/ },
    { mode: 'disconnect-during-validation', signal: 15, error: /exited before requested shutdown/ },
    { mode: 'disconnect-after-close', signal: 15, escalation: ['term'] },
  ];
  test('cancels admission and removes the profile when the launcher never connects', async t => {
    await fs.mkdir('tmp', { recursive: true });
    const output = await fs.mkdtemp('tmp/capture-shutdown-test-');
    t.after(() => fs.rm(output, { recursive: true }));
    const profile = path.join(output, 'profile');
    await fs.mkdir(profile);
    const control = await openLauncherControl();
    t.after(() => control.close());
    const error = new Error('startup timed out');
    await assert.rejects(finalizeBrowserCapture({
      output, control, profile, manifest: { complete: false },
      captureError: () => error, closeConnections: () => {},
    }), actual => actual === error);
    await assert.rejects(fs.stat(profile), { code: 'ENOENT' });
    const late = createConnection({ host: '127.0.0.1', port: control.port });
    await assert.rejects(once(late, 'connect'), { code: 'ECONNREFUSED' });
    assert.equal(control.admitted, false);
  });

  for (const expected of cases) {
    test(expected.mode, { timeout: 9000 }, async t => {
      await fs.mkdir('tmp', { recursive: true });
      const output = await fs.mkdtemp('tmp/capture-shutdown-test-');
      t.after(async () => {
        await fs.writeFile(path.join(output, 'release-fixture'), 'yes');
        await until(() => read(path.join(output, 'supervisor-exited')));
        await fs.rm(output, { recursive: true });
      });
      const command = start('command', output, expected.mode);
      let stderr = '';
      command.stderr.on('data', bytes => { stderr += bytes; });
      const [status, signal] = await once(command, 'close');
      const manifest = JSON.parse(await read(path.join(output, 'manifest.json')) ?? 'null');
      const durableReceipt = await readLauncherReceipt(output);
      if (expected.mode === 'receipt-write') assert.equal(durableReceipt, null);
      const receipt = durableReceipt ?? manifest?.permission_preflight;
      assert.equal(await read(path.join(output, 'http-closed')), 'yes');
      assert.equal(signal, null);
      assert.equal(status, expected.error ? 1 : 0);
      assert.equal(Boolean(manifest), !expected.noManifest);
      if (manifest) assert.equal(manifest.complete, !expected.error);
      assert.equal(await read(path.join(output, 'validated-frame')), 'validated bytes');
      if (expected.error) {
        const primary = await read(path.join(output, 'command-error'));
        assert.match(primary, expected.error);
        if (manifest) assert.equal(primary, manifest.error);
        if (expected.noManifest) assert.match(stderr, /Could not write capture manifest: Error: ENOSPC writing manifest.json/);
      } else {
        assert.equal(receipt.chrome_final, true);
        assert.equal(receipt.screen_recording_before, false);
        assert.equal(receipt.screen_recording_after, false);
        assert.equal(manifest.chrome_shutdown.requested, true);
        assert.deepEqual(manifest.chrome_shutdown.escalation, expected.escalation);
        assert.equal(manifest.cleanup_errors, undefined);
      }
      if (expected.code !== undefined) {
        assert.equal(receipt.chrome_exit_code, expected.code);
        if (manifest) assert.equal(manifest.chrome_shutdown.exit_code, expected.code);
      }
      if (expected.signal !== undefined) {
        assert.equal(receipt.chrome_exit_signal, expected.signal);
        assert.equal(manifest.chrome_shutdown.signal, expected.signal);
      }
      const gone = expected.mode !== 'unconfirmed';
      if (manifest) {
        assert.equal(manifest.chrome_shutdown.owned_processes_gone, gone);
        assert.equal(manifest.chrome_shutdown.profile_removed, gone);
      }
      if (expected.noManifest) {
        assert.equal(receipt.owned_processes_gone, true);
        assert.equal(receipt.profile_removed, true);
      }
      const profile = fs.stat(path.join(output, 'profile'));
      if (gone) await assert.rejects(profile, { code: 'ENOENT' });
      else assert((await profile).isDirectory());
      for (const role of ['browser', 'helper']) {
        const pid = Number(await read(path.join(output, role + '-ready')));
        assert(pid > 0);
        if (gone) assert.throws(() => process.kill(pid, 0), { code: 'ESRCH' });
        else assert.equal(process.kill(pid, 0), true);
      }
      if (expected.mode === 'status-2') {
        assert.equal(await read(path.join(output, 'wrapper-exited')), 'yes');
        assert.equal(await read(path.join(output, 'cdp-close')), 'yes');
        assert.equal(receipt.shutdown_escalation, undefined);
        await until(() => read(path.join(output, 'supervisor-exited')));
        assert.deepEqual(JSON.parse(await read(path.join(output, 'control-phases'))), []);
      }
      if (expected.mode === 'early-exit' || expected.mode.startsWith('capture-error') || expected.mode === 'before-devtools') {
        assert.equal(await read(path.join(output, 'cdp-close')), undefined);
      }
      if (expected.mode === 'disconnect-during-validation') {
        assert.equal(receipt.shutdown_requested, false);
        assert.equal(manifest.chrome_shutdown.requested, false);
        assert.deepEqual(receipt.shutdown_escalation, ['term']);
        assert.deepEqual(JSON.parse(await read(path.join(output, 'harness-requests'))), []);
        assert.equal(await read(path.join(output, 'cdp-close')), undefined);
        assert.equal(receipt.screen_recording_before, false);
        assert.equal(receipt.screen_recording_after, false);
      }
      if (expected.mode === 'disconnect-after-close') {
        assert.equal(receipt.shutdown_requested, true);
        assert.equal(JSON.parse(await read(path.join(output, 'harness-requests')))[0], 'close');
      }
      if (expected.mode === 'capture-error-and-cleanup-error') {
        assert.deepEqual(manifest.cleanup_errors, ['Error: injected cleanup failure']);
      }
    });
  }
}
