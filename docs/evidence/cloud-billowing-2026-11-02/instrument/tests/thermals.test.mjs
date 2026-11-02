import { analyzeDensity } from "../src/morphology.js";
import assert from "node:assert/strict";
import {
  ThermalDriver,
  rollingVelocity,
  evaporate,
  MAX_HEADS,
  BAND_STRIDE,
} from "../src/thermals.js";
import { SourceDriver } from "../src/source.js";
import { Weather } from "../src/weather.js";
assert.deepEqual(rollingVelocity([0, 0, 0], 900, 630, 24), [0, 48, 0]);
const sample = rollingVelocity([900, 630, 0], 900, 630, 24);
assert.ok(Math.abs(sample[0] - ((2 * 24 * 900) / 630) * Math.exp(-2)) < 1e-12);
assert.equal(sample[1], 0);
const point = [100, 200, 50],
  epsilon = 0.01;
let div = 0;
for (let axis = 0; axis < 3; axis++) {
  const a = [...point],
    b = [...point];
  a[axis] += epsilon;
  b[axis] -= epsilon;
  div +=
    (rollingVelocity(a, 900, 630, 24)[axis] -
      rollingVelocity(b, 900, 630, 24)[axis]) /
    (2 * epsilon);
}
assert.ok(Math.abs(div) < 1e-8, "isolated streamfunction roll divergence");
assert.equal(evaporate(0.1, 0.005, 30), 0);
assert.ok(Math.abs(evaporate(0.4, 0.001, 30) - 0.37) < 1e-12);
let source = new SourceDriver(),
  heads = new ThermalDriver(),
  weather = new Weather(71, { moist: 1, lift: 2, wind: 12 });
source.reset(0, weather.sample(0), false);
heads.reset(0, weather.sample(0), source.pulses);
let maximum = 0;
for (let step = 0; step < 9600; step++) {
  const t = step * 0.25;
  weather.advanceTo(t);
  const f = weather.sample(t);
  source.update(t + 0.25, 0.25, f);
  const table = heads.update(t + 0.25, 0.25, f, source.pulses);
  maximum = Math.max(maximum, heads.heads.length);
  assert.ok(heads.heads.length <= MAX_HEADS);
  for (let row = 0; row < 32; row++)
    assert.ok(table.bands[row * BAND_STRIDE] <= MAX_HEADS);
}
assert.ok(maximum > 5, "sustained post-emitter head population exercised");
const fresh = new ThermalDriver(),
  f = new Weather().sample(0),
  s = new SourceDriver();
s.reset(0, f, true);
fresh.reset(0, f, s.pulses);
heads.reset(0, f, s.pulses);
assert.deepEqual(heads.data, fresh.data);
assert.deepEqual(heads.bands, fresh.bands);
const first = heads.heads[0];
assert.ok(
  first.birth < 0 && first.y >= 1400,
  "seeded pose is explicit and finite",
);
const pulse = { id: "test", serial: 1, birth: 0, x: 0, z: 0, radius: 500 };
const retained = new ThermalDriver();
retained.reset(0, f, [pulse]);
for (let i = 1; i <= 400; i++) retained.update(i * 0.25, 0.25, f, []);
assert.equal(
  retained.heads.length,
  1,
  "head persists after finite source pulse is removed",
);
console.log(
  "Thermal roll, finite loss, bounded long-run packing and deterministic reset tests passed",
);

const size = [8, 8, 8],
  field = new Float32Array(512);
const at = (x, y, z) => (z * 8 + y) * 8 + x;
for (let z = 2; z < 4; z++)
  for (let y = 2; y < 4; y++)
    for (let x = 1; x < 3; x++) {
      field[at(x, y, z)] = 0.8;
      field[at(x + 4, y, z)] = 0.8;
    }
for (let x = 3; x < 5; x++) field[at(x, 2, 2)] = 0.1;
assert.equal(
  analyzeDensity(field, size, [
    [0, 0, 0],
    [8, 8, 8],
  ]).substantialComponents,
  1,
);
const eroded = field.map((q) => evaporate(q, 0.005, 25));
assert.equal(
  analyzeDensity(eroded, size, [
    [0, 0, 0],
    [8, 8, 8],
  ]).substantialComponents,
  2,
  "thin neck disappears while two dense lobes survive",
);

const { validateMotion } = await import("../src/motion.js");
const before = {revision: 1, epoch: 2, playing: true, gpuErrors: 0, fault: null, sourceTime: 10, imageFrame: 20};
const after = {...before, sourceTime: 22, imageFrame: 100};
assert.equal(validateMotion(before, after), "fixed-fixture");
assert.equal(validateMotion(before, {...after, revision: 3}), "fixture-changed");
assert.equal(validateMotion(before, {...after, fault: "history expired"}), "faulted-or-paused");
assert.equal(validateMotion(before, {...after, imageFrame: 20}), "no-source-or-image-progress");
