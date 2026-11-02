// Observer-only topology and head/feed checks. Reusable typed scratch bounds CPU memory.
export function analyzeDensity(
  q,
  dims,
  bounds,
  packedHeads = [],
  scratch = {},
) {
  const [nx, ny, nz] = dims,
    N = nx * ny * nz,
    plane = nx * ny;
  const labels = (scratch.labels ??= new Uint32Array(N)),
    queue = (scratch.queue ??= new Uint32Array(N)),
    base = (scratch.base ??= new Uint8Array(N + 1));
  labels.fill(0);
  base.fill(0);
  let components = 0,
    substantial = 0;
  const largest = [];
  for (let i = 0; i < N; i++) {
    if (q[i] < 0.08 || labels[i]) continue;
    const id = ++components;
    let first = 0,
      last = 1,
      count = 0,
      minX = nx,
      minY = ny,
      minZ = nz,
      maxX = 0,
      maxY = 0,
      maxZ = 0;
    queue[0] = i;
    labels[i] = id;
    const add = (k) => {
      if (!labels[k] && q[k] >= 0.08) {
        labels[k] = id;
        queue[last++] = k;
      }
    };
    while (first < last) {
      const k = queue[first++],
        x = k % nx,
        y = Math.floor(k / nx) % ny,
        z = Math.floor(k / plane);
      count++;
      minX = Math.min(minX, x);
      maxX = Math.max(maxX, x);
      minY = Math.min(minY, y);
      maxY = Math.max(maxY, y);
      minZ = Math.min(minZ, z);
      maxZ = Math.max(maxZ, z);
      if (x > 0) add(k - 1);
      if (x < nx - 1) add(k + 1);
      if (y > 0) add(k - nx);
      if (y < ny - 1) add(k + nx);
      if (z > 0) add(k - plane);
      if (z < nz - 1) add(k + plane);
    }
    if (
      bounds[0][1] + ((minY + 0.5) * (bounds[1][1] - bounds[0][1])) / ny <
      1800
    )
      base[id] = 1;
    if (count >= 8) substantial++;
    if (largest.length < 16 || count > largest[largest.length - 1].voxels) {
      largest.push({
        id,
        voxels: count,
        min: [minX, minY, minZ],
        max: [maxX, maxY, maxZ],
        touchesFeed: !!base[id],
      });
      largest.sort((a, b) => b.voxels - a.voxels);
      if (largest.length > 16) largest.pop();
    }
  }
  const cell = (p) => {
    const a = p.map((v, j) =>
      Math.floor(
        ((v - bounds[0][j]) / (bounds[1][j] - bounds[0][j])) * dims[j],
      ),
    );
    return a.some((v, j) => v < 0 || v >= dims[j])
      ? -1
      : (a[2] * ny + a[1]) * nx + a[0];
  };
  const heads = [];
  let total = 0,
    overlap = 0;
  for (let i = 0; i < packedHeads.length / 12; i++) {
    const a = packedHeads.subarray(i * 12, i * 12 + 12);
    if (a[4] <= 1e-4) continue;
    const center = cell(a.subarray(0, 3)),
      front = cell([a[0], a[1] + a[5] * 0.8, a[2]]);
    heads.push({
      id: a[8],
      position: Array.from(a.subarray(0, 3)),
      radius: a[3],
      height: a[5],
      amplitude: a[4],
      centerQ: center < 0 ? 0 : q[center],
      frontQ: front < 0 ? 0 : q[front],
      frontComponent: front < 0 ? 0 : labels[front],
      frontConnectedToFeed: front >= 0 && !!base[labels[front]],
    });
  }
  for (let i = 0; i < N; i++) {
    if (q[i] <= 0.015) continue;
    total += q[i];
    const x =
        bounds[0][0] + (((i % nx) + 0.5) * (bounds[1][0] - bounds[0][0])) / nx,
      y =
        bounds[0][1] +
        (((Math.floor(i / nx) % ny) + 0.5) * (bounds[1][1] - bounds[0][1])) /
          ny,
      z =
        bounds[0][2] +
        ((Math.floor(i / plane) + 0.5) * (bounds[1][2] - bounds[0][2])) / nz;
    if (
      heads.some(
        (h) =>
          ((x - h.position[0]) ** 2 + (z - h.position[2]) ** 2) /
            (4 * h.radius ** 2) +
            (y - h.position[1]) ** 2 / (4 * h.height ** 2) <=
          1,
      )
    )
      overlap += q[i];
  }
  return {
    scope:
      "Observer-only6-neighbour q>=.08 components; substantial>=8voxels; head overlap within2R/2H, not exact mass or visible-light topology",
    components,
    substantialComponents: substantial,
    largest,
    heads,
    headWeightedOverlap: total ? overlap / total : 0,
  };
}
