#!/usr/bin/env node
// Diagnostic observer over the unchanged conformance browser runner.
// Usage: node scripts/sim2d_browser_profile.mjs WEB_ROOT REQUEST_JSON PROFILE_JSON
import fs from 'node:fs/promises';

const runner = await fs.readFile(new URL('./sim2d_browser.mjs', import.meta.url), 'utf8');
const begin = "  const evaluated = await call('Runtime.evaluate',";
const end = '  if (evaluated.exceptionDetails)';
if (runner.split(begin).length !== 2 || runner.split(end).length !== 2 || !process.argv[4]) {
  throw new Error('Expected browser observation sites and an output profile path');
}
const observed = runner
  .replace(begin, `  await call('Profiler.enable', {});
  await call('Profiler.setSamplingInterval', { interval: 1000 });
  await call('Profiler.start', {});
${begin}`)
  .replace(end, `  const profile = await call('Profiler.stop', {});
  await fs.writeFile(process.argv[4], JSON.stringify(profile));
${end}`);
await import(`data:text/javascript;base64,${Buffer.from(observed).toString('base64')}`);
