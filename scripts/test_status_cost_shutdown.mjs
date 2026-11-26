import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs/promises';
import path from 'node:path';
import { test } from 'node:test';
import { cleanup } from './check_v1_browser.mjs';

const watchdog = 'Teardown watchdog expired; recording dump and terminating.\n';

test('only a completed status cost take accepts a confirmed teardown watchdog', async t => {
  await fs.mkdir('tmp', { recursive: true });
  const directory = await fs.mkdtemp('tmp/status-cost-shutdown-');
  t.after(() => fs.rm(directory, { recursive: true, force: true }));

  for (const arm of [
    { name: 'normal exit', code: 0 },
    { name: 'confirmed watchdog', code: 2, log: watchdog },
    { name: 'unexplained exit', code: 2, error: /did not exit cleanly/ },
    { name: 'incomplete take', code: 2, log: watchdog, incomplete: true, error: /did not exit cleanly/ },
    { name: 'early watchdog', code: 2, log: watchdog, early: true, error: /exited before graceful cleanup/ },
  ]) {
    await t.test(arm.name, async () => {
      const profile = await fs.mkdtemp(path.join(directory, 'profile-'));
      const log = path.join(profile, 'chrome.log');
      const handle = await fs.open(log, 'w');
      const child = spawn(process.execPath, ['-e', `
        process.stdin.once('data', () => {
          process.stdout.write(${JSON.stringify(arm.log ?? '')}, () => process.exit(${arm.code}));
        });
      `], { detached: true, stdio: ['pipe', handle.fd, handle.fd] });
      const exit = once(child, 'exit');
      await handle.close();
      const cdp = {
        socket: { readyState: WebSocket.OPEN, close() {} },
        async call(method) {
          assert.equal(method, 'Browser.close');
          child.stdin.end('close');
        },
      };
      if (arm.early) {
        child.stdin.end('exit early');
        await exit;
      }
      const receipt = {};
      const shutdown = cleanup(child, () => child.exitCode !== null || child.signalCode !== null,
        cdp, undefined, profile, {
          ownedShutdown: { log, receipt },
          allowTeardownWatchdog: !arm.incomplete,
        });
      if (arm.error) {
        await assert.rejects(shutdown, arm.error);
        assert((await fs.stat(profile)).isDirectory());
      } else {
        await shutdown;
        assert.equal(receipt.owned_processes_gone, true);
        assert.equal(receipt.watchdog, arm.code === 2);
        await assert.rejects(fs.stat(profile), { code: 'ENOENT' });
      }
      await exit;
    });
  }
});
