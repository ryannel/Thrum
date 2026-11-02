// Bounded prescribed thermal controls, never drawn geometry or an aloft density source.
export const HEAD_STRIDE = 16;
export const MAX_HEADS = 48,
  BAND_COUNT = 32,
  BAND_STRIDE = 49;
const clamp = (x, a, b) => Math.max(a, Math.min(b, x));
const smooth = (a, b, x) => {
  const t = clamp((x - a) / (b - a), 0, 1);
  return t * t * (3 - 2 * t);
};
export class ThermalDriver {
  constructor() {
    this.data = new Float32Array(MAX_HEADS * HEAD_STRIDE);
    this.bands = new Uint32Array(BAND_COUNT * BAND_STRIDE);
    this.heads = [];
    this.lastSerial = 0;
    this.time = 0;
  }
  reset(time, weather, pulses, materialCore = false) {
    this.materialCore = materialCore;
    this.heads = [];
    this.lastSerial = 0;
    this.time = time;
    return this.update(time, 0, weather, pulses, true);
  }
  update(time, dt, weather, pulses, seeded = false) {
    if (time < this.time - 1e-6 || dt < 0)
      throw Error("Thermal time cannot rewind");
    const speed = 28 * Math.sqrt(weather.lift);
    for (const h of this.heads) {
      const old = h.y;
      h.y += speed * dt;
      const shear = clamp(((old + h.y) * 0.5 - 1100) / 8000, 0, 1);
      const jet = smooth(5500, 9500, (old + h.y) * 0.5);
      h.x += (weather.wind * (1 + 0.6 * shear) + 18 * jet) * dt;
      h.z += (1.1 + 3 * shear - 5 * jet) * dt;
    }
    this.heads = this.heads.filter((h) => time - h.birth < 420 && h.y < 11000);
    for (const p of pulses) {
      if (p.serial <= this.lastSerial) continue;
      this.lastSerial = p.serial;
      if ((p.serial - 1) % 5 !== 0) continue;
      if (this.heads.length >= MAX_HEADS)
        throw Error("Thermal head capacity exceeded");
      const age = seeded ? Math.max(0, time - p.birth) : 0;
      const y = 1400 + speed * age;
      const u = clamp((y - 5500) / 4000, 0, 1);
      const integral = 4000 * (u ** 3 - 0.5 * u ** 4) + Math.max(0, y - 9500);
      const jetTime = speed > 1e-6 ? integral / speed : 0;
      const shear = clamp((300 + speed * age * 0.5) / 8000, 0, 1);
      this.heads.push({
        id: p.id,
        serial: p.serial,
        birth: p.birth,
        x: weather.windIntegral + 2 * p.x + weather.wind * 0.6 * shear * age + 18 * jetTime,
        y,
        z: 2 * p.z + 1.1 * time + 3 * shear * age - 5 * jetTime,
        r0: clamp(p.radius * 2.6, 1100, 1600),
      });
    }
    this.data.fill(0);
    this.bands.fill(0);
    this.forceHeads = 0;
    for (let i = 0; i < this.heads.length; i++) {
      const h = this.heads[i],
        age = time - h.birth,
        R = h.r0 + 0.08 * Math.max(0, h.y - 1400),
        H = this.materialCore ? .55 * h.r0 + .14 * Math.max(0,h.y-1400) : .55 * R;
      const life = smooth(0,12,age)*(1-smooth(360,420,age));
      const window = life*(1-smooth(8400,9900,h.y));
      const transportLife = this.materialCore ? life*(1-smooth(10300,11000,h.y)) : 0;
      const A = 24 * Math.sqrt(weather.lift) * window;
      this.data.set(
        [h.x, h.y, h.z, R, A, H, window, speed, h.serial, Math.cos(h.serial * 2.4), Math.sin(h.serial * 2.4), 1 + .28 * Math.sin(h.serial * 1.7), speed, .08*speed/R, (this.materialCore ? .14 : .044)*speed/H, transportLife],
        i * HEAD_STRIDE,
      );
      if (A < 1e-4 && transportLife < 1e-4) continue;
      this.forceHeads++;
      const first = clamp(Math.floor((h.y - 3 * H) / 500), 0, BAND_COUNT - 1),
        last = clamp(Math.floor((h.y + 3 * H) / 500), 0, BAND_COUNT - 1);
      for (let band = first; band <= last; band++) {
        const offset = band * BAND_STRIDE,
          count = this.bands[offset];
        if (count >= MAX_HEADS)
          throw Error("Thermal height-band capacity exceeded");
        this.bands[offset + 1 + count] = i;
        this.bands[offset] = count + 1;
      }
    }
    this.time = time;
    return { heads: this.data, bands: this.bands };
  }
}
export function rollingVelocity(d, R, H, A) {
  const r2 = (d[0] ** 2 + d[2] ** 2) / R ** 2,
    E = Math.exp(-r2 - d[1] ** 2 / H ** 2);
  return [
    ((2 * A * d[1] * d[0]) / H ** 2) * E,
    2 * A * (1 - r2) * E,
    ((2 * A * d[1] * d[2]) / H ** 2) * E,
  ];
}
export function evaporate(q, e, dt) {
  return Math.max(0, q - e * dt);
}
