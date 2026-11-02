// Finite seeded cirrus bundles in the existing 32×two-vec4 source-control layout.
// Head controls supply ice; already emitted tails follow GPU wind/fall and losses.
export const CIRRUS_HEADS = 8;
const random = (n) => {
  let x = n | 0;
  x ^= x >>> 16;
  x = Math.imul(x, 0x7feb352d);
  x ^= x >>> 15;
  x = Math.imul(x, 0x846ca68b);
  x ^= x >>> 16;
  return (x >>> 0) / 4294967296;
};
export function cirrusControls(seed = 71, wind = 3) {
  const data = new Float32Array(32 * 8);
  const direction = Math.atan2(1.7, wind * 3.12);
  for (let i = 0; i < CIRRUS_HEADS; i++) {
    const n = seed + i * 97,
      angle = direction + (random(n + 1) - 0.5) * 0.42,
      length = 1700 + random(n + 2) * 2200,
      c = Math.cos(angle), s = Math.sin(angle),
      cx = (random(n + 3) - 0.5) * 4500,
      cz = (random(n + 4) - 0.5) * 4600;
    data.set([
      cx - .45 * length * c, cz - .45 * length * s,
      160 + random(n + 5) * 170, .65 + random(n + 6) * .35,
      length, c, s, 9500 + random(n + 7) * 1000,
    ], i * 8);
  }
  return data;
}
