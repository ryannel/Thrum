import assert from "node:assert/strict";
import {
  Weather,
  WorldClock,
  reaction,
  validateProvider,
} from "../src/weather.js";
import { SourceDriver } from "../src/source.js";
for (const q of [0, 0.1, 0.8, 1])
  for (const s of [0, 0.003, 2])
    for (const d of [0, 0.01, 3]) {
      const v = reaction(q, s, d, 0.25);
      assert(v >= 0 && v <= 1);
      assert(
        Math.abs(reaction(reaction(q, s, d, 0.125), s, d, 0.125) - v) < 1e-12,
      );
    }
assert.equal(reaction(0.6, 0, 0, 2), 0.6);
assert(reaction(0.4, 1, 0, 1) > 0.4);
assert(reaction(0.4, 0, 1, 1) < 0.4);
const a = new Weather(),
  b = new Weather();
a.command({ wind: -8, dry: 0.9 });
b.command({ wind: -8, dry: 0.9 });
assert.equal(a.revision, 0);
a.advanceTo(120);
for (let t = 0; t <= 120; t += 0.25) b.advanceTo(t);
assert.deepEqual(a.snapshot(120), b.snapshot(120));
assert(a.state.wind < 0);
assert.equal(a.tick, 120);
assert.throws(() => a.command({ moist: -1 }));
assert.throws(() => a.command({ wind: NaN }));
a.sendFront();
assert.equal(a.frontX, 1e6);
a.advanceTo(121);
assert.equal(a.frontX, 3800);
assert.equal(a.revision, 2);
assert.equal(a.commands.at(-1).type, "front");
const w = new Weather(),
  clock = new WorldClock();
w.sendFront();
for (let i = 0; i < 600; i++) clock.step(1 / 60, 1, true, w);
assert.equal(w.tick, 10);
assert(w.frontX < 3800);
assert(Math.abs(clock.time - 10) < 1e-10); // No renderer or GPU is involved.
assert.throws(() => validateProvider("full", "shadow"));
assert.throws(() => validateProvider("shadow", "clear"));
assert(validateProvider("full", "volume").runtime);
assert(!validateProvider("shadow", "shadow").visible);
const source = new SourceDriver();
for (let i = 0; i < 4800; i++)
  source.update(i * 0.25, 0.25, { moist: 1, lift: 1 });
assert(source.serial > 100);
assert(source.pulses.length < 32);
const zero = new SourceDriver();
for (let i = 0; i < 100; i++)
  zero.update(i * 0.25, 0.25, { moist: 1, lift: 0 });
assert.equal(zero.serial, 0);
console.log(
  "weather barrier/replay/headless clock, positive reaction, provider rejection and indefinite emitter tests passed",
);

// A GPU that consumes an interval later must see the same committed forcing.
const causal = new Weather();
causal.command({ wind: -12 });
causal.sendFront();
const early = [0, 0.25, 0.5, 0.75].map((t) => causal.sample(t));
causal.advanceTo(1);
assert.deepEqual(
  early,
  [0, 0.25, 0.5, 0.75].map((t) => causal.sample(t)),
);
assert.equal(causal.sample(1).frontX, 3800);
assert.equal(causal.sample(1).windIntegral, 3);
const live = new Weather(),
  delayed = new Weather();
const observed = [];
for (let tick = 0; tick < 20; tick++) {
  if (tick === 5) {
    live.command({ wind: -8, dry: 0.9 });
    delayed.command({ wind: -8, dry: 0.9 });
  }
  for (let q = 0; q < 4; q++) observed.push(live.sample(tick + q / 4));
  live.step();
  delayed.step();
}
assert.deepEqual(
  observed,
  observed.map((f) => delayed.sample(f.time)),
);
