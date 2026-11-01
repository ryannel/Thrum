/** Focused source-only checks; no renderer/GPU, and no appearance/performance pass. */
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const bytes = await readFile(new URL('./convective.js', import.meta.url));
const { createCuRecipe, sampleCu, cuFrame, CONVECTIVE_WGSL } = await import(`data:text/javascript;base64,${bytes.toString('base64')}`);

const clamp = (x) => Math.max(0, Math.min(1, x));
function smooth(a, b, x) { const u = clamp((x - a) / (b - a)); return u * u * (3 - 2 * u); }
const lerp = (x, y, a) => x * (1 - a) + y * a;

// Independent integer arithmetic oracle: BigInt masks emulate WGSL wrapping u32.
function hash(x, y, z, seed) {
  const mask = 0xffffffffn;
  let h = (BigInt(x) * 374761393n + BigInt(y) * 668265263n + BigInt(z) * 2147483647n + BigInt(seed) * 1274126177n) & mask;
  h = ((h ^ (h >> 13n)) * 1274126177n) & mask;
  return Number((h ^ (h >> 16n)) & 16777215n) / 16777216;
}
function noise(p, seed) {
  const base = p.map(Math.floor), u = p.map((v, k) => smooth(0, 1, v - base[k]));
  let value = 0;
  for (let z = 0; z < 2; z++) for (let y = 0; y < 2; y++) for (let x = 0; x < 2; x++) {
    value += hash(base[0] + x, base[1] + y, base[2] + z, seed) * (x ? u[0] : 1 - u[0]) * (y ? u[1] : 1 - u[1]) * (z ? u[2] : 1 - u[2]);
  }
  return value;
}

// This reads only the public packed frame, not the recipe or CPU private producer.
function packedReference(frame, q) {
  if (frame[1] <= 0 || frame[1] >= frame[2] || q[1] <= 0) return 0;
  let result = 0;
  for (let i = 0; i < frame[0]; i++) {
    const o = 4 + i * 32;
    if (frame[o + 7] < 0.5) continue;
    const delta = q.map((v, k) => v - frame[o + k]);
    const r = Math.sqrt(delta.reduce((sum, v, k) => sum + (v * frame[o + 4 + k]) ** 2, 0));
    if (r >= 1) continue;
    const material = delta.map((v, k) => (v + frame[o + 8 + k]) * frame[o + 11]);
    const n = noise(material, frame[o + 27]);
    const nn = noise(material.map((v) => v * frame[o + 22]), frame[o + 27] + 17);
    const eta = frame[o + 20] * (0.25 + 0.75 * lerp(n, nn, frame[o + 23]));
    const projection = delta.reduce((sum, v, k) => sum + v * frame[o + 24 + k] * frame[o + 12 + k], 0);
    const exposure = clamp(0.5 + 0.45 * projection + 0.18 * (n - 0.5));
    const stop = frame[o + 15] + frame[o + 16] + frame[o + 17] * exposure;
    const dying = (1 + frame[o + 19]) * smooth(stop, stop + frame[o + 18], frame[1]);
    const argument = (1 - r) - eta * Math.pow(r, frame[o + 21]) - dying;
    result = Math.max(result, frame[o + 3] * smooth(0, frame[o + 19], argument));
  }
  return result * smooth(0, frame[3], q[1]);
}

let maxPackedError = 0;
const summaries = [];
for (let variant = 0; variant < 4; variant++) {
  const r = createCuRecipe(variant, 1729);
  assert.deepEqual(r, createCuRecipe(variant, 1729), 'recipe must replay exactly');
  assert.deepEqual(JSON.parse(JSON.stringify(r)), r, 'source input must round-trip as JSON');
  assert.ok(Object.isFrozen(r) && Object.isFrozen(r.cohorts) && Object.isFrozen(r.cohorts[0].radii_m));
  assert.ok(r.cohorts.length <= 64 && r.bounds.extent[0] >= 800 && r.bounds.extent[0] <= 2000);
  assert.ok(r.bounds.extent[1] >= 500 && r.bounds.extent[1] <= 1300);
  assert.ok(r.cohorts.some((n) => n.birth_s >= r.stages.mature[0] && n.birth_s < r.stages.mature[1]), 'mature interval needs fresh local cohorts');
  assert.ok(r.cohorts.some((n) => n.retire_s > 650 && n.retire_s < 1250), 'older cohorts must retire during renewal');
  assert.ok(r.cohorts.every((n) => n.retire_s <= r.duration_s));
  for (const n of r.cohorts.slice(1)) {
    assert.ok(sampleCu(r, n.start_m, n.birth_s) > 0.01, 'daughter starts in a filled surviving parent');
  }
  const frame = cuFrame(r, 900);
  assert.equal(frame.length, 4 + 32 * r.cohorts.length);
  assert.deepEqual([...frame.slice(0, 4)], [r.cohorts.length, 900, 1800, 12]);
  for (const time of [-1, 0, 160, 510, 990, 1320, 1540, 1731, 1800, 2000]) {
    const f = cuFrame(r, time);
    for (let sample = 0; sample < 48; sample++) {
      const q = r.bounds.min.map((v, k) => v + r.bounds.extent[k] * (((sample * [17, 31, 47][k] + 11 * k) % 97) / 96));
      const sigma = sampleCu(r, q, time);
      assert.ok(sigma >= 0 && sigma <= 0.1 && Number.isFinite(sigma));
      const error = Math.abs(sigma - packedReference(f, q));
      maxPackedError = Math.max(maxPackedError, error);
      assert.ok(error < 1.5e-6, `packed source mismatch ${variant}/${time}/${q}: ${error}`);
      if (time <= 0 || time >= 1731) assert.equal(sigma, 0, 'exact birth/terminal emptiness');
    }
    for (let axis = 0; axis < 3; axis++) for (const sign of [-1, 1]) {
      const outside = r.bounds.min.map((v, k) => v + 0.5 * r.bounds.extent[k]);
      outside[axis] = r.bounds.min[axis] + (sign < 0 ? -1 : r.bounds.extent[axis] + 1);
      assert.equal(sampleCu(r, outside, time), 0, 'source must remain inside conservative support');
    }
  }
  let born = 0, lost = 0, occupied = 0;
  for (let z = 0; z < 17; z++) for (let y = 0; y < 17; y++) for (let x = 0; x < 17; x++) {
    const q = [x, y, z].map((v, k) => r.bounds.min[k] + (v + 0.5) / 17 * r.bounds.extent[k]);
    const a = sampleCu(r, q, 740), b = sampleCu(r, q, 1140);
    if (a < 1e-7 && b > 1e-5) born++;
    if (a > 1e-5 && b < 1e-7) lost++;
    if (b > 1e-5) occupied++;
  }
  assert.ok(born > 0 && lost > 0, 'mature behaviour needs simultaneous local birth/loss, not one opacity change');
  summaries.push({ variant, nodes: r.cohorts.length, bounds: r.bounds, bornSamples: born, lostSamples: lost, occupiedSamples: occupied });
}
// Tiny positive ages previously packed Infinity despite mathematically positive radii.
// Check each actual birth, then both sides of the declared minimum-support cutoff.
let tinyBirthChecks = 0;
for (let variant = 0; variant < 4; variant++) {
  const recipe = createCuRecipe(variant);
  const epsilon = recipe.numerical_birth.minimum_max_radius_m;
  assert.equal(epsilon, 1e-6);
  const tauBound = recipe.cohorts.reduce((sum, n) => sum + 2 * n.sigma_m_inv * epsilon, 0);
  assert.ok(recipe.numerical_birth.max_omitted_ray_tau >= tauBound);
  assert.ok(recipe.numerical_birth.max_omitted_ray_tau - tauBound < 1e-18);
  for (const node of recipe.cohorts) {
    const isolated = { ...recipe, cohorts: [node] };
    const maxRadius = Math.max(...node.radii_m);
    for (const offset of [Number.MIN_VALUE, 1e-20, 1e-12, 1e-8, 1e-5]) {
      const t = node.birth_s + offset;
      const frame = cuFrame(isolated, t);
      const q = [...frame.slice(4, 7)];
      assert.ok(frame.every(Number.isFinite), 'all tiny-age packed floats must be finite');
      assert.equal(frame[11], 0, 'sub-resolution support must remain inactive');
      assert.deepEqual([...frame.slice(8, 11)], [0, 0, 0], 'inactive support never computes inverse radii');
      assert.equal(sampleCu(isolated, q, t), 0, 'CPU uses the same omitted-support admission');
      assert.equal(packedReference(frame, q), 0);
      tinyBirthChecks++;
    }
    let lo = 0, hi = node.grow_s;
    for (let i = 0; i < 70; i++) {
      const mid = (lo + hi) / 2;
      if (maxRadius * smooth(0, node.grow_s, mid) < epsilon) lo = mid;
      else hi = mid;
    }
    const before = cuFrame(isolated, node.birth_s + hi * 0.5);
    const after = cuFrame(isolated, node.birth_s + hi * 2);
    assert.equal(before[11], 0);
    assert.equal(after[11], 1, 'resolved support becomes active without changing its source age');
    assert.ok(after.every(Number.isFinite));
    assert.ok(after.slice(8, 11).every((x) => x > 0));
    const q = [...after.slice(4, 7)];
    const expected = sampleCu(isolated, q, node.birth_s + hi * 2);
    assert.ok(Math.abs(expected - packedReference(after, q)) < 1.5e-6);
  }
}
const badReciprocal = JSON.parse(JSON.stringify(createCuRecipe()));
badReciprocal.cohorts[0].detail_m = Number.MIN_VALUE;
assert.throws(() => cuFrame(badReciprocal, 0), RangeError, 'non-finite metadata reciprocal rejects even inactive uploads');
assert.throws(() => cuFrame(createCuRecipe(), 1e40), RangeError, 'float32 metadata overflow also rejects upload');

assert.equal(new Set(summaries.map((s) => JSON.stringify(s.bounds.extent))).size, 4, 'gross profiles must differ');
assert.notDeepEqual(createCuRecipe(0, 1), createCuRecipe(0, 2), 'seed changes retained source content');
assert.throws(() => createCuRecipe(4), RangeError);
assert.throws(() => createCuRecipe(0, -1), RangeError);
assert.throws(() => sampleCu(createCuRecipe(), [NaN, 1, 2], 100), TypeError);
const bad = JSON.parse(JSON.stringify(createCuRecipe()));
bad.cohorts[0].sigma_m_inv = 0.2;
assert.throws(() => sampleCu(bad, [0, 100, 0], 500), RangeError, 'reject rather than clamp extinction');
assert.throws(() => cuFrame(bad, 500), RangeError);
assert.ok(CONVECTIVE_WGSL.includes('@group(0) @binding(2)'));
console.log(JSON.stringify({ result: 'source checks pass; appearance/GPU unproven', maxPackedError, tinyBirthChecks, summaries }, null, 2));
