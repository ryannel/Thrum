import { HEAD_STRIDE } from "./thermals.js";
// Observer-only topology and head/feed checks. Reusable typed scratch bounds CPU memory.
export function analyzeDensity(
  q,
  dims,
  bounds,
  packedHeads = [],
  scratch = {},
  { threshold = .08, feedRoot = null, coreRadialScale = 1 } = {},
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
    if (q[i] < threshold || labels[i]) continue;
    const id = ++components;
    let first = 0,
      last = 1,
      count = 0,
      minX = nx,
      minY = ny,
      minZ = nz,
      maxX = 0,
      maxY = 0,
      maxZ = 0, touchesRoot = false;
    queue[0] = i;
    labels[i] = id;
    const add = (k) => {
      if (!labels[k] && q[k] >= threshold) {
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
      if(feedRoot){const wy=bounds[0][1]+(y+.5)*(bounds[1][1]-bounds[0][1])/ny;
        if(wy>=1100&&wy<=1750){const wx=bounds[0][0]+(x+.5)*(bounds[1][0]-bounds[0][0])/nx,wz=bounds[0][2]+(z+.5)*(bounds[1][2]-bounds[0][2])/nz;
          if((wx-feedRoot[0])**2+(wz-feedRoot[2])**2<=500**2)touchesRoot=true;
        }
      }
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
    if (feedRoot ? touchesRoot : bounds[0][1] + ((minY + 0.5) * (bounds[1][1] - bounds[0][1])) / ny < 1800)
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
  for (let i = 0; i < packedHeads.length / HEAD_STRIDE; i++) {
    const a = packedHeads.subarray(i * HEAD_STRIDE, i * HEAD_STRIDE + HEAD_STRIDE);
    if (a[4] <= 1e-4 && a[15] <= 1e-4) continue;
    const center = cell(a.subarray(0, 3)),
      front = cell([a[0], a[1] + a[5] * (coreRadialScale < 1 ? .45 : .8), a[2]]);
    const reach=.65*coreRadialScale*a[3]*Math.max(a[11],1/a[11]);
    const lo=[a[0]-reach,a[1]-.65*a[5],a[2]-reach].map((v,j)=>Math.max(0,Math.floor((v-bounds[0][j])*dims[j]/(bounds[1][j]-bounds[0][j]))));
    const hi=[a[0]+reach,a[1]+.65*a[5],a[2]+reach].map((v,j)=>Math.min(dims[j]-1,Math.ceil((v-bounds[0][j])*dims[j]/(bounds[1][j]-bounds[0][j]))));
    let nativeCells=0,occupiedCells=0,connectedCells=0,sum=0,peak=0;
    for(let z=lo[2];z<=hi[2];z++)for(let y=lo[1];y<=hi[1];y++)for(let x=lo[0];x<=hi[0];x++){
      const dx=bounds[0][0]+(x+.5)*(bounds[1][0]-bounds[0][0])/nx-a[0],dy=bounds[0][1]+(y+.5)*(bounds[1][1]-bounds[0][1])/ny-a[1],dz=bounds[0][2]+(z+.5)*(bounds[1][2]-bounds[0][2])/nz-a[2];
      const u=(a[9]*dx+a[10]*dz)/a[11],v=(-a[10]*dx+a[9]*dz)*a[11];
      if((u*u+v*v)/(coreRadialScale**2*a[3]*a[3])+dy*dy/(a[5]*a[5])>.65**2)continue;
      const k=(z*ny+y)*nx+x;nativeCells++;sum+=q[k];peak=Math.max(peak,q[k]);
      if(q[k]>=threshold){occupiedCells++;if(base[labels[k]])connectedCells++;}
    }
    heads.push({
      id: a[8],
      position: Array.from(a.subarray(0, 3)),
      orientation: [a[9], a[10]],
      aspect: a[11],
      radius: a[3],
      height: a[5],
      amplitude: a[4],
      transportActivity: a[15],
      coreAscent: a[12],
      fractionalGrowth: [a[13],a[14]],
      plateau:{radialScale:coreRadialScale,semiAxes:[.65*coreRadialScale*a[3]*a[11],.65*a[5],.65*coreRadialScale*a[3]/a[11]],nativeCells,occupiedCells,connectedCells,meanQ:nativeCells?sum/nativeCells:0,peakQ:peak,connectedOccupiedFraction:occupiedCells?connectedCells/occupiedCells:0},
      centerConnectedToFeed:center>=0&&!!base[labels[center]],
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
      heads.some((h) => {
        const dx=x-h.position[0], dz=z-h.position[2], [c,s]=h.orientation;
        const u=(c*dx+s*dz)/h.aspect, v=(-s*dx+c*dz)*h.aspect;
        return (u*u+v*v)/(4*h.radius**2)+(y-h.position[1])**2/(4*h.height**2)<=1;
      })
    )
      overlap += q[i];
  }
  return {
    scope:
      `Observer-only6-neighbour q>=${threshold} components; substantial>=8voxels; head overlap within2R/2H; feed=${feedRoot?'current root500m catchment, height1100..1750m':'any support below1800m'}; not exact mass or visible-light topology`,
    components,
    substantialComponents: substantial,
    largest,
    heads,
    headWeightedOverlap: total ? overlap / total : 0,
  };
}
