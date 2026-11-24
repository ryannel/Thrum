import fs from 'node:fs/promises';
import path from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

export async function readLauncherReceipt(output) {
  try {
    return JSON.parse(await fs.readFile(path.join(output, 'launcher.json'), 'utf8'));
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
}

async function requestShutdown(control, phase) {
  const next = path.join(control, 'request.next');
  await fs.writeFile(next, phase);
  await fs.rename(next, path.join(control, 'request'));
}

async function waitForReceipt(output, predicate, milliseconds) {
  const deadline = performance.now() + milliseconds;
  do {
    const receipt = await readLauncherReceipt(output);
    if (receipt && predicate(receipt)) return receipt;
    const remaining = deadline - performance.now();
    if (remaining <= 0) return;
    await sleep(Math.min(50, remaining));
  } while (true);
}

const finalReceipt = receipt => receipt.chrome_final === true;

async function stopBrowser({ output, control, launchError, cdp }, { graceMs, termMs, killMs, finalMs }) {
  if (!control || launchError) return;
  let receipt = await readLauncherReceipt(output);
  if (finalReceipt(receipt ?? {})) return receipt;
  // The supervisor checks for a prior exit before acknowledging this request.
  // Browser.close may never reply, so its promise cannot hold up escalation.
  await requestShutdown(control, 'close');
  receipt = await waitForReceipt(output, row => row.shutdown_requested || row.chrome_exited || finalReceipt(row), finalMs);
  if (receipt?.shutdown_requested && !receipt.chrome_final && !receipt.chrome_exited) {
    void cdp?.call('Browser.close').catch(() => {});
  }
  if (!receipt?.chrome_final) {
    receipt = await waitForReceipt(output, row => finalReceipt(row) || row.owned_processes_gone, graceMs);
  }
  for (const [phase, duration] of [['term', termMs], ['kill', killMs]]) {
    if (receipt?.chrome_final || receipt?.owned_processes_gone) break;
    await requestShutdown(control, phase);
    receipt = await waitForReceipt(output, row => finalReceipt(row) || row.owned_processes_gone, duration);
  }
  // The supervisor still has to reap and run the attributed postflight after
  // the group is empty. Its final receipt is the only proof of termination.
  if (!receipt?.chrome_final) {
    receipt = await waitForReceipt(output, finalReceipt, finalMs) ?? await readLauncherReceipt(output);
  }
  return receipt;
}

export async function finalizeBrowserCapture({
  output, control, launchError, cdp, closeConnections, profile, manifest, captureError,
  graceMs = 10000, termMs = 3000, killMs = 10000, finalMs = 15000,
}) {
  const cleanupErrors = [];
  const clean = async action => {
    try { return await action(); }
    catch (error) { cleanupErrors.push(String(error)); }
  };
  const receipt = await clean(() => stopBrowser(
    { output, control, launchError, cdp }, { graceMs, termMs, killMs, finalMs },
  ));
  const launched = Boolean(control && !launchError);
  const gone = !launched || receipt?.owned_processes_gone === true;
  if (launched) {
    manifest.permission_preflight = receipt ?? manifest.permission_preflight;
    manifest.chrome_shutdown = {
      requested: receipt?.shutdown_requested ?? false,
      escalation: receipt?.shutdown_escalation ?? [],
      exit_code: receipt?.chrome_exit_code ?? null,
      signal: receipt?.chrome_exit_signal ?? null,
      owned_processes_gone: gone,
      profile_removed: false,
    };
    if (!receipt?.chrome_final || !gone) cleanupErrors.push('Owned Chrome exit and final launcher receipt could not be confirmed');
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
  if (profile && gone) {
    try {
      await fs.rm(profile, { recursive: true });
      if (manifest.chrome_shutdown) manifest.chrome_shutdown.profile_removed = true;
    } catch (error) { cleanupErrors.push(`Profile cleanup: ${error.message}`); }
  }
  let failure = captureError();
  if (cleanupErrors.length) {
    manifest.cleanup_errors = cleanupErrors;
    failure ??= new Error(cleanupErrors.join('; '));
  }
  manifest.error = failure ? String(failure) : undefined;
  manifest.complete &&= !failure;
  await fs.writeFile(path.join(output, 'manifest.json'), JSON.stringify(manifest, null, 2));
  if (failure) throw failure;
}
