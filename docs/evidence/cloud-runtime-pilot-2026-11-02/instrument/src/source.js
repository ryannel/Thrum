// Derived source controls: finite, seeded injection pulses, never rendered lobe geometry.
function random(n) {
  let x = n | 0;
  x ^= x >>> 16;
  x = Math.imul(x, 0x7feb352d);
  x ^= x >>> 15;
  x = Math.imul(x, 0x846ca68b);
  x ^= x >>> 16;
  return (x >>> 0) / 4294967296;
}
export class SourceDriver {
  constructor(seed = 71) {
    this.seed = seed;
    this.serial = 0;
    this.hazard = 0;
    this.threshold = 1 + 0.5 * random(seed);
    this.pulses = [];
    this.data = new Float32Array(32 * 8);
  }
  update(time, dt, weather) {
    this.pulses = this.pulses.filter(
      (p) => time < p.birth + p.rise + p.hold + p.fall,
    );
    this.hazard += 0.15 * weather.moist * weather.lift * dt;
    while (this.hazard >= this.threshold) {
      this.hazard -= this.threshold;
      this.spawn(time);
      this.threshold = 1 + 0.5 * random(this.seed + this.serial * 17);
    }
    this.data.fill(0);
    for (let i = 0; i < this.pulses.length; i++) {
      const p = this.pulses[i];
      this.data.set(
        [p.x, p.z, p.radius, 1, p.birth, p.rise, p.hold, p.fall],
        i * 8,
      );
    }
    return this.data;
  }
  spawn(time) {
    if (this.pulses.length >= 32)
      throw Error("Source emitter capacity exceeded");
    const n = this.serial++ * 97 + this.seed;
    this.pulses.push({
      id: `cell:${this.seed}/source:${this.serial}`,
      x: (random(n + 1) - 0.5) * 1700,
      z: (random(n + 2) - 0.5) * 1500,
      radius: 330 + random(n + 3) * 300,
      birth: time,
      rise: 6 + random(n + 4) * 10,
      hold: 16 + random(n + 5) * 25,
      fall: 12 + random(n + 6) * 18,
    });
  }
  reset(time, weather, seeded) {
    this.serial = 0;
    this.hazard = 0;
    this.pulses = [];
    if (seeded) {
      for (let i = 0; i < 5; i++) this.spawn(time - 15 - i * 4);
    }
    return this.update(time, 0, weather);
  }
}
