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

// A held profile has an independent integral; catches CPU head jet/base drift.
const steady = {...f, wind: 0, windIntegral: 0, lift: 1};
const jetHead = new ThermalDriver();
jetHead.reset(0, steady, [{...pulse, x: 100, z: -50}]);
for (let i = 1; i <= 1000; i++) jetHead.update(i * .25, .25, steady, []);
const h = jetHead.heads[0], u = (8400 - 5500) / 4000;
const jetSeconds = 4000 * (u**3 - .5*u**4) / 28;
assert.ok(Math.abs(h.x - (200 + 18*jetSeconds)) < .001);
assert.ok(Math.abs(h.z - (-100 + 1.1*250 + 3*118.75 - 5*jetSeconds)) < .001);

// The old head's upper moving-frame stagnation is axially compressive, independent of resolution.
const H=800,R=1500,A=24,U=28,bg=5,zeta=H*Math.sqrt(Math.log(2*A/(U-bg)));
const strain=(U-bg)*zeta/(H*H);
const d=[0,zeta,0], spectrum=[];
for(let axis=0;axis<3;axis++){
 const a=[...d],b=[...d];a[axis]+=.01;b[axis]-=.01;
 spectrum.push((rollingVelocity(a,R,H,A)[axis]-rollingVelocity(b,R,H,A)[axis])/.02);
}
assert.ok(Math.abs(rollingVelocity(d,R,H,A)[1]+bg-U)<1e-10);
for(let axis=0;axis<3;axis++)assert.ok(Math.abs(spectrum[axis]-strain*(axis===1?-2:1))<1e-8);
assert.ok(Math.exp(-2*strain*60)<.06,'old roll compresses thickness below6% in60s');

// Active material control remains visible and indexed after its roll has shut down.
const { HEAD_STRIDE } = await import('../src/thermals.js');
const terminal=new ThermalDriver(),tw={...f,lift:2};terminal.reset(0,tw,[pulse],true);
for(let i=1;i<=870;i++)terminal.update(i*.25,.25,tw,[]);
assert.equal(terminal.data[4],0);assert.ok(terminal.data[15]>.99);
assert.equal(terminal.forceHeads,1);assert.ok(terminal.bands.some(v=>v>0));
assert.equal(analyzeDensity(new Float32Array(512),[8,8,8],[[0,0,0],[1500,15000,1500]],terminal.data).heads.length,1);
assert.equal(terminal.data.length,48*HEAD_STRIDE);
const Uc=28,r0=1500,h0=.55*r0,T=180;
let dx=100,dy=100;
for(let n=0;n<T*4;n++){const t=n*.25;dx*=1+.25*.08*Uc/(r0+.08*Uc*t);dy*=1+.25*.14*Uc/(h0+.14*Uc*t);}
assert.ok(Math.abs(dx/100-(r0+.08*Uc*T)/r0)<1e-9);
assert.ok(Math.abs(dy/100-(h0+.14*Uc*T)/h0)<1e-9);
assert.ok(dy/dx>1.4,'occupied material thickens relative to width');
