import assert from "node:assert/strict";
import { mergeReset } from "../src/requests.js";
const clear = { clear: true, newWorld: true };
const family = { clear: false, newWorld: true, preserveClear: true };
const recover = { clear: false, newWorld: false, preserveClear: true };
assert.deepEqual(
  mergeReset(mergeReset(clear, family), recover),
  clear,
  "Clear/new-world survive subsequent structural family/recovery requests",
);
assert.deepEqual(
  mergeReset(clear, { clear: false, newWorld: true }),
  { clear: false, newWorld: true },
  "Latest explicit Seeded start overrides pending Clear",
);
assert.deepEqual(
  mergeReset({ clear: false, newWorld: true }, recover),
  { clear: false, newWorld: true },
  "Optical-only recovery cannot erase pending CPU world reset",
);
assert.deepEqual(
  mergeReset(null, recover),
  { clear: false, newWorld: false },
  "Standalone recovery stays optical-only",
);
let pending = clear;
for (let i = 0; i < 10000; i++)
  pending = mergeReset(pending, i % 2 ? recover : family);
assert.deepEqual(
  pending,
  clear,
  "Coalesced busy-window requests retain one bounded transaction",
);
console.log("Busy-window reset intent scenarios passed");
