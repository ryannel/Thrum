import fs from 'node:fs/promises';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import { createServer } from 'node:net';
import { setTimeout as sleep } from 'node:timers/promises';

// LaunchServices does not inherit Node's descriptors. Authenticate the one
// supervisor on an ephemeral loopback port before granting permission to spawn.
export async function openLauncherControl() {
  const token = randomUUID();
  let owner, receipt;
  let closed = false;
  const sockets = new Set();
  const server = createServer(socket => {
    sockets.add(socket);
    socket.setEncoding('utf8');
    socket.on('error', () => socket.destroy());
    socket.on('close', () => sockets.delete(socket));
    let buffer = '';
    socket.on('data', chunk => {
      if (closed) return;
      buffer += chunk;
      let newline;
      while ((newline = buffer.indexOf('\n')) !== -1) {
        const line = buffer.slice(0, newline);
        buffer = buffer.slice(newline + 1);
        if (owner !== socket) {
          if (owner || line !== token) { socket.destroy(); return; }
          owner = socket;
          server.close();
          for (const other of sockets) if (other !== owner) other.destroy();
          owner.write('start\n');
        } else {
          try { receipt = JSON.parse(line); }
          catch { socket.destroy(); return; }
        }
      }
      if (buffer.length > 16384) socket.destroy();
    });
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  return {
    port: server.address().port,
    token,
    get admitted() { return Boolean(owner); },
    get connected() { return Boolean(owner && !owner.destroyed && !owner.readableEnded); },
    get receipt() { return receipt; },
    request(phase) { if (this.connected) owner.write(phase + '\n'); },
    close() {
      if (closed) return;
      closed = true;
      server.close();
      for (const socket of sockets) socket.destroy();
    },
  };
}

async function readLauncherReceipt(output, control) {
  const receipt = control.receipt;
  if (control.connected || receipt?.chrome_final) return receipt;
  // If the control connection failed, the supervisor still shuts Chrome down
  // and writes its final receipt when the volume permits.
  try {
    const saved = JSON.parse(await fs.readFile(path.join(output, 'launcher.json'), 'utf8'));
    return saved.chrome_final ? saved : receipt;
  } catch {
    return receipt;
  }
}

async function waitForReceipt(output, control, predicate, milliseconds) {
  const deadline = performance.now() + milliseconds;
  do {
    const receipt = await readLauncherReceipt(output, control);
    if (receipt && predicate(receipt)) return receipt;
    const remaining = deadline - performance.now();
    if (remaining <= 0) return;
    await sleep(Math.min(50, remaining));
  } while (true);
}

const finalReceipt = receipt => receipt.chrome_final === true;
const groupGone = receipt => finalReceipt(receipt) || receipt.owned_processes_gone;

async function stopBrowser({ output, control, cdp }, { graceMs, termMs, killMs, finalMs }) {
  if (!control?.admitted) return;
  let receipt = await readLauncherReceipt(output, control);
  if (receipt?.chrome_final) return receipt;
  // The supervisor checks for a prior exit before acknowledging this request.
  // Browser.close may never reply, so its promise cannot hold up escalation.
  control.request('close');
  receipt = await waitForReceipt(output, control, row => row.shutdown_requested || row.chrome_exited || finalReceipt(row), finalMs);
  if (receipt?.shutdown_requested && !receipt.chrome_final && !receipt.chrome_exited) {
    void cdp?.call('Browser.close').catch(() => {});
  }
  if (!receipt || !groupGone(receipt)) {
    receipt = await waitForReceipt(output, control, groupGone, graceMs);
  }
  for (const [phase, duration] of [['term', termMs], ['kill', killMs]]) {
    if (receipt && groupGone(receipt)) break;
    control.request(phase);
    receipt = await waitForReceipt(output, control, groupGone, duration);
  }
  if (!receipt?.chrome_final) {
    receipt = await waitForReceipt(output, control, finalReceipt, finalMs) ?? await readLauncherReceipt(output, control);
  }
  return receipt;
}

export async function finalizeBrowserCapture({
  output, control, cdp, closeConnections, profile, manifest, captureError,
  graceMs = 10000, termMs = 3000, killMs = 10000, finalMs = 15000,
}) {
  const cleanupErrors = [];
  const clean = async action => {
    try { return await action(); }
    catch (error) { cleanupErrors.push(String(error)); }
  };
  const launched = control?.admitted;
  if (!launched) control?.close(); // Prevent a delayed LaunchServices launch.
  const receipt = await clean(() => stopBrowser(
    { output, control, cdp }, { graceMs, termMs, killMs, finalMs },
  ));
  control?.close();
  if (launched) {
    manifest.permission_preflight = receipt ?? manifest.permission_preflight;
    manifest.chrome_shutdown = {
      requested: receipt?.shutdown_requested ?? false,
      escalation: receipt?.shutdown_escalation ?? [],
      exit_code: receipt?.chrome_exit_code ?? null,
      signal: receipt?.chrome_exit_signal ?? null,
      owned_processes_gone: receipt?.owned_processes_gone === true,
      profile_removed: receipt?.profile_removed === true,
    };
    if (!receipt?.chrome_final || !receipt.owned_processes_gone) cleanupErrors.push('Owned Chrome exit and final launcher receipt could not be confirmed');
    if (!receipt?.profile_removed) cleanupErrors.push(receipt?.profile_error ?? 'Owned Chrome profile removal could not be confirmed');
    if (receipt?.error) cleanupErrors.push(receipt.error);
    if (receipt && !receipt.shutdown_requested) cleanupErrors.push('Owned Chrome exited before requested shutdown');
    if (receipt?.screen_recording_before !== false || receipt?.screen_recording_after !== false) {
      cleanupErrors.push('Launcher requires both denied Screen Recording preflights');
    }
    if (receipt && !Number.isInteger(receipt.chrome_exit_code) && !Number.isInteger(receipt.chrome_exit_signal)) {
      cleanupErrors.push('Launcher did not record Chrome termination');
    }
  }
  await clean(closeConnections);
  if (profile && !launched) await clean(() => fs.rm(profile, { recursive: true }));
  let failure = captureError();
  if (cleanupErrors.length) {
    manifest.cleanup_errors = cleanupErrors;
    failure ??= new Error(cleanupErrors.join('; '));
  }
  manifest.error = failure ? String(failure) : undefined;
  manifest.complete &&= !failure;
  try {
    await fs.writeFile(path.join(output, 'manifest.json'), JSON.stringify(manifest, null, 2));
  } catch (error) {
    if (!failure) throw error;
    for (const diagnostic of cleanupErrors) console.error(`Chrome cleanup: ${diagnostic}`);
    console.error('Could not write capture manifest:', error);
  }
  if (failure) throw failure;
}
