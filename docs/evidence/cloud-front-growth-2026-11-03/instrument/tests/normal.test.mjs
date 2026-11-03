import assert from 'node:assert/strict';
import { applyFrontMotionCpu } from '../src/normal-reference.js';

const six = (v) => Array.from({ length: 6 }, () => [...v]);
const close = (actual, expected, tolerance = 1e-10) =>
  assert.ok(Math.abs(actual - expected) <= tolerance, `${actual} differs from ${expected}`);

const vacuum = applyFrontMotionCpu([0, 0, 0, 0], six([0, 0, 0, 0]), [187.5, 181.25, 187.5], .25, 1000, 25, 21);
assert.deepEqual(vacuum, [0, 0, 0, 0], 'An empty front cannot create matter');

const uniform = [.4, .2, 70, .1];
applyFrontMotionCpu(uniform, six(uniform), [187.5, 181.25, 187.5], .25, 1000, 25, 21)
  .forEach((v, i) => close(v, uniform[i]));

const rising = six([.4, 0, 40, .2]);
rising[0] = [.5, 0, 80, .3]; rising[1] = [.3, 0, 20, .1];
const ramp = applyFrontMotionCpu([.4, 0, 40, .2], rising, [100, 100, 100], .25, 0, 20, 0);
close(ramp[0], .405);
close(ramp[2], 40 * .4 / .405, 1e-9);
close(ramp[3], .205);

const fullNeighbours = six([1, 1, 20, .4]);
const capped = applyFrontMotionCpu([.98, .3, 100, .2], fullNeighbours, [10, 10, 10], .25, 0, 40, 0);
close(capped[0], 1, 1e-12);
close(capped[1], .3, 1e-12);
close(capped[2], 1.28 * 100 / 1.3, 1e-9);
close(capped[3], .22, 1e-12);

const eroded = applyFrontMotionCpu([.4, .4, 100, .3], six([0, 0, 0, 0]), [10, 10, 10], .25, 0, 0, 20);
const retreat = .25 * 20 * Math.sqrt(3) * .8 / 10;
close(eroded[0], .4 - retreat * .5);
close(eroded[1], .4 - retreat * .08 * .5);
close(eroded[2], 100, 1e-9);
close(eroded[3], .3 * (eroded[0] + eroded[1]) / .8, 1e-9);

const mixedNeighbours = six([0, 0, 0, 0]);
mixedNeighbours[0] = [.6, .2, 20, .3];
const mixed = applyFrontMotionCpu([.2, .4, 100, .1], mixedNeighbours, [100, 100, 100], .25, 1000, 0, 0);
const weight = .25 * 1000 / 10000;
close(mixed[0], .2 + weight * (.6 - .2) + 5 * weight * (0 - .2));
close(mixed[1], .4 + weight * (.2 - .4) + 5 * weight * (0 - .4));
const expectedMoment = .6 * 100 + weight * (.8 * 20 - .6 * 100) + 5 * weight * (0 - .6 * 100);
close(mixed[2], expectedMoment / (mixed[0] + mixed[1]));
close(mixed[3], .1 + weight * (.3 - .1) + 5 * weight * (0 - .1));

for (const result of [vacuum, uniform, ramp, capped, eroded, mixed])
  assert.ok(result.every(Number.isFinite), 'All phases and metadata remain finite');
console.log('Independent normal-front vacuum, uniform, ramp, capacity, phase erosion and moment/birth checks passed');
