// Double precision independent atmosphere quadrature; not used by the renderer.
export function airTransmittance(height, mu, samples = 4096) {
  const R = 6371000,
    top = 6471000,
    r = R + height,
    b = r * mu;
  if (b < 0 && b * b - r * r + R * R > 0) return [0, 0, 0];
  const end = -b + Math.sqrt(b * b - r * r + top * top),
    tau = [0, 0, 0],
    ds = end / samples;
  for (let i = 0; i < samples; i++) {
    const s = (i + 0.5) * ds,
      h = Math.sqrt(r * r + s * s + 2 * b * s) - R,
      ray = Math.exp(-h / 8000),
      mie = 4e-6 * Math.exp(-h / 1200),
      ozone = Math.max(0, 1 - Math.abs(h - 25000) / 15000);
    for (let c = 0; c < 3; c++)
      tau[c] +=
        ([5.802e-6, 13.558e-6, 33.1e-6][c] * ray +
          mie +
          [0.65e-6, 1.881e-6, 0.085e-6][c] * ozone) *
        ds;
  }
  return tau.map((t) => Math.exp(-t));
}
export function halfToFloat(h) {
  const sign = h & 0x8000 ? -1 : 1,
    e = (h >> 10) & 31,
    m = h & 1023;
  if (e === 0) return sign * m * 2 ** -24;
  if (e === 31) return m ? NaN : sign * Infinity;
  return sign * (1 + m / 1024) * 2 ** (e - 15);
}
export function floatToHalf(value) {
  const sign = value < 0 || Object.is(value, -0) ? 0x8000 : 0;
  const x = Math.abs(value);
  if (Number.isNaN(x)) return sign | 0x7e00;
  if (x === Infinity) return sign | 0x7c00;
  if (x === 0) return sign;
  const roundEven = (n) => {
    const floor = Math.floor(n),
      frac = n - floor;
    return floor + (frac > 0.5 || (frac === 0.5 && floor & 1) ? 1 : 0);
  };
  if (x < 2 ** -14) return sign | roundEven(x / 2 ** -24);
  let exponent = Math.floor(Math.log2(x)),
    significand = roundEven(x / 2 ** (exponent - 10));
  if (significand === 2048) {
    exponent++;
    significand = 1024;
  }
  if (exponent + 15 >= 31) return sign | 0x7c00;
  return sign | ((exponent + 15) << 10) | (significand - 1024);
}
