/**
 * Authored Cu optical candidate for CLOUDS.md CF01, not a fluid simulation or visual pass.
 * All positions/radii/detail metrics are physical local metres; condensation base is y=0.
 * Source input is this versioned finite graph plus seed, never camera/frame-rate state.
 * Four gross morphologies, <64 cohorts, 1800s history; exact terminal loss precedes 1800s.
 * No external asset/library; integer value noise is fixed in each cohort's physical frame.
 * Preserve the prepared Q1 source as an independent comparator/provider.
 * Numerical birth policy: a cohort is inactive while its largest radius is <1e-6m.
 * Any omitted ellipsoid chord is <=2e-6m, so omitted tau per node is bounded by
 * 2 * sigma0 * 1e-6 (<=3.8e-8 for these recipes). Summing node bounds covers
 * disjoint omitted chords and max-union overlap; |delta T|<=delta tau. This is a
 * declared negligible-support approximation, not a change to the source clock.
 * Current pose centres/radii are float32-defined for CPU/frame agreement when
 * sub-millimetre newborns are smaller than centre-coordinate float32 rounding.
 *
 * Frame layout (Float32Array, vec4 units):
 * header [node count, source time, duration, base feather metres], then 8 vec4/node:
 * 0 centre.xyz,sigma0; 1 inverse radii.xyz,active;
 * 2 material offset.xyz,1/detail metres; 3 drying direction.xyz,stop time;
 * 4 delay base,delay span,dry duration,e_width;
 * 5 erosion amplitude,p,secondary frequency,secondary weight;
 * 6 inverse FINAL/reference radii.xyz,integer noise seed (exact float <=65535);
 * 7 birth time,parent index,category,reserved.
 * canonicalSigma(q,age) requires age == header.sourceTime; poses already describe that time.
 * No binding0 declaration here: the caller owns its field uniform. Only binding2 is added.
 */

const DURATION = 1800;
const SIGMA_LIMIT = 0.1;
const STRIDE = 32;
const BASE_FEATHER = 12;
const MIN_RESOLVED_MAX_RADIUS_M = 1e-6;
const PROFILES = [
  { name: 'broad shallow', width: 1380, height: 640, depth: 1050, branches: [-0.7, 0, 0.65], generations: [90, 420, 850], heightBias: [0.72, 0.93, 0.79] },
  { name: 'clustered turrets', width: 1770, height: 1080, depth: 1310, branches: [-0.75, -0.2, 0.3, 0.78], generations: [100, 360, 690, 990], heightBias: [0.77, 1, 0.91, 0.7] },
  { name: 'narrow rising', width: 1050, height: 1250, depth: 770, branches: [-0.42, 0.47], generations: [90, 360, 710, 1030], heightBias: [1, 0.84] },
  { name: 'wide asymmetric banks', width: 1800, height: 850, depth: 1160, branches: [-0.95, -0.42, 0.06, 0.53, 0.9], generations: [110, 470, 930], heightBias: [0.58, 0.76, 0.91, 0.74, 0.6] },
];

function hash32(x, y, z, seed) {
  let h = (Math.imul(x | 0, 374761393) + Math.imul(y | 0, 668265263) + Math.imul(z | 0, 2147483647) + Math.imul(seed | 0, 1274126177)) >>> 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177) >>> 0;
  return (h ^ (h >>> 16)) >>> 0;
}

function random01(seed, index) {
  return (hash32(index, 17, 41, seed) & 0xffffff) / 0x1000000;
}

function smooth(a, b, x) {
  const u = Math.max(0, Math.min(1, (x - a) / (b - a)));
  return u * u * (3 - 2 * u);
}

function mix(a, b, t) { return a + (b - a) * t; }
function freezeTree(value) {
  if (value && typeof value === 'object') {
    Object.values(value).forEach(freezeTree);
    Object.freeze(value);
  }
  return value;
}

function noise3(x, y, z, seed) {
  const ix = Math.floor(x), iy = Math.floor(y), iz = Math.floor(z);
  const fx = smooth(0, 1, x - ix), fy = smooth(0, 1, y - iy), fz = smooth(0, 1, z - iz);
  const corner = (dx, dy, dz) => (hash32(ix + dx, iy + dy, iz + dz, seed) & 0xffffff) / 0x1000000;
  return mix(mix(mix(corner(0, 0, 0), corner(1, 0, 0), fx), mix(corner(0, 1, 0), corner(1, 1, 0), fx), fy),
    mix(mix(corner(0, 0, 1), corner(1, 0, 1), fx), mix(corner(0, 1, 1), corner(1, 1, 1), fx), fy), fz);
}

function pose(node, time) {
  const grow = smooth(node.birth_s, node.birth_s + node.grow_s, time);
  const physicalRadius = node.radii_m.map((v) => v * grow);
  return {
    centre: node.start_m.map((v, k) => Math.fround(mix(v, node.end_m[k], grow))),
    radius: physicalRadius.map(Math.fround),
    active: time > node.birth_s && time < node.retire_s && Math.max(...physicalRadius) >= MIN_RESOLVED_MAX_RADIUS_M,
  };
}

function finiteReciprocal(value) {
  const inverse = Math.fround(1 / value);
  if (!(value > 0) || !Number.isFinite(inverse) || inverse <= 0) {
    throw new RangeError('Cu reciprocal is outside finite positive float32 admission');
  }
  return inverse;
}

/** Serialisable frozen source input. Valid variants are 0..3; seed is an unsigned integer. */
export function createCuRecipe(variant = 0, seed = 1729) {
  if (!Number.isInteger(variant) || variant < 0 || variant >= PROFILES.length) throw new RangeError('Cu variant must be 0..3');
  if (!Number.isInteger(seed) || seed < 0 || seed > 0xffffffff) throw new RangeError('Cu seed must be uint32');
  const profile = PROFILES[variant];
  const cohorts = [];
  const makeNode = (data) => {
    const id = cohorts.length;
    const r = (n) => random01(seed, id * 19 + n);
    const dir = [mix(-1, 1, r(1)), mix(0.1, 0.8, r(2)), mix(-1, 1, r(3))];
    const length = Math.hypot(...dir);
    const node = {
      id, parent: data.parent, category: data.category || 'billow',
      birth_s: data.birth_s, grow_s: data.grow_s, stop_s: data.stop_s,
      dry_duration_s: data.dry_duration_s || 230,
      delay_base_s: 0, delay_span_s: data.delay_span_s || 130,
      start_m: data.start_m, end_m: data.end_m, radii_m: data.radii_m,
      sigma_m_inv: data.sigma_m_inv || 0.019,
      detail_m: mix(100, 155, r(4)),
      material_offset_m: [r(5) * 6000, r(6) * 6000, r(7) * 6000],
      erosion: mix(0.32, 0.6, r(8)), exponent: 1.5, edge_width: 0.24,
      secondary_frequency: 2.7, secondary_weight: 0.3,
      drying_direction: dir.map((v) => v / length),
      noise_seed: Math.floor(r(9) * 65536),
    };
    node.retire_s = node.stop_s + node.delay_base_s + node.delay_span_s + node.dry_duration_s;
    cohorts.push(node);
    return node;
  };
  const root = makeNode({
    parent: -1, category: 'condensation body', birth_s: 0, grow_s: 360, stop_s: 1250,
    dry_duration_s: 300, delay_span_s: 180,
    start_m: [0, 0, 0], end_m: [0, profile.height * 0.14, 0],
    radii_m: [profile.width * 0.31, profile.height * 0.43, profile.depth * 0.36], sigma_m_inv: 0.017,
  });
  for (let generation = 0; generation < profile.generations.length; generation++) {
    for (let branch = 0; branch < profile.branches.length; branch++) {
      const ordinal = cohorts.length;
      const r = (n) => random01(seed, ordinal * 23 + n + 800);
      const birth = profile.generations[generation] + branch * 19 + r(1) * 24;
      const rootPose = pose(root, birth);
      // Every filled child centre starts deep in the already filled parent's plateau.
      const start = rootPose.centre.map((v, k) => v + rootPose.radius[k] * (r(k + 2) - 0.5) * 0.12);
      const size = mix(0.84, 1.1, r(5));
      const horizontal = profile.branches[branch];
      const lateTilt = (generation / Math.max(1, profile.generations.length - 1) - 0.5) * 0.1;
      makeNode({
        parent: 0, birth_s: birth, grow_s: mix(160, 245, r(6)),
        stop_s: Math.min(1350, birth + mix(360, 490, r(7))),
        start_m: start,
        end_m: [profile.width * (horizontal * 0.32 + lateTilt), profile.height * (0.5 * profile.heightBias[branch] + 0.08 * r(8)), profile.depth * (r(9) - 0.5) * 0.47],
        radii_m: [profile.width * mix(0.18, 0.235, r(10)) * size, profile.height * mix(0.27, 0.36, r(11)) * size, profile.depth * mix(0.23, 0.31, r(12)) * size],
      });
    }
  }
  if (cohorts.length > 64) throw new RangeError('Cu cohort capacity exceeded');
  const lo = [Infinity, 0, Infinity], hi = [-Infinity, 0, -Infinity];
  for (const node of cohorts) {
    if (node.retire_s > DURATION || node.sigma_m_inv > SIGMA_LIMIT || node.sigma_m_inv < 0 || node.radii_m.some((v) => !(v > 0))) throw new RangeError('Cu source outside admitted optical/time/support range');
    for (let k = 0; k < 3; k++) {
      lo[k] = Math.min(lo[k], Math.min(node.start_m[k], node.end_m[k]) - node.radii_m[k] - 4);
      hi[k] = Math.max(hi[k], Math.max(node.start_m[k], node.end_m[k]) + node.radii_m[k] + 4);
    }
  }
  lo[1] = 0;
  return freezeTree({
    schema: 'thrum.authored-cu.v1', variant, seed, name: profile.name,
    provenance: 'Authored CF01 cohort candidate from CLOUDS.md; no fluid/thermodynamic or accepted-appearance claim.',
    duration_s: DURATION, stages: { formation: [0, 300], growth: [300, 650], mature: [650, 1250], breakup: [1250, 1800] },
    normal_intervals: [[120, 180], [510, 570], [990, 1050], [1440, 1500]],
    bounds: { min: lo, extent: hi.map((v, k) => v - lo[k]) },
    base_feather_m: BASE_FEATHER, sigma_limit_m_inv: SIGMA_LIMIT, cohorts,
    numerical_birth: {
      minimum_max_radius_m: MIN_RESOLVED_MAX_RADIUS_M,
      pose_precision: 'Current centre/radius are float32; support cutoff is checked before rounding.',
      // Outward rounding headroom for at most64 positive terms and two products.
      max_omitted_ray_tau: 2 * MIN_RESOLVED_MAX_RADIUS_M * cohorts.reduce((sum, n) => sum + n.sigma_m_inv, 0) * (1 + 128 * Number.EPSILON),
      scope: 'Omitted-support bound at the declared current pose; separate pose rounding is not part of this cutoff bound.',
    },
    frame_layout: { header_floats: 4, node_floats: STRIDE, binding: 2 },
  });
}

function coefficient(node, current, q, time) {
  if (!Number.isFinite(node.sigma_m_inv) || node.sigma_m_inv < 0 || node.sigma_m_inv > SIGMA_LIMIT) throw new RangeError('Invalid Cu node extinction; never clamp');
  if (!current.active || current.radius.some((v) => v <= 0)) return 0;
  const delta = q.map((v, k) => v - current.centre[k]);
  const radial = Math.hypot(...delta.map((v, k) => v / current.radius[k]));
  if (radial >= 1) return 0;
  const m = 1 - radial;
  const material = delta.map((v, k) => (v + node.material_offset_m[k]) / node.detail_m);
  const n0 = noise3(...material, node.noise_seed);
  const n1 = noise3(...material.map((v) => v * node.secondary_frequency), node.noise_seed + 17);
  const detail = mix(n0, n1, node.secondary_weight);
  const eta = node.erosion * (0.25 + 0.75 * detail);
  const projection = delta.reduce((sum, v, k) => sum + v / node.radii_m[k] * node.drying_direction[k], 0);
  const exposure = Math.max(0, Math.min(1, 0.5 + 0.45 * projection + 0.18 * (n0 - 0.5)));
  const departure = node.stop_s + node.delay_base_s + node.delay_span_s * exposure;
  const drying = (1 + node.edge_width) * smooth(departure, departure + node.dry_duration_s, time);
  return node.sigma_m_inv * smooth(0, node.edge_width, m - eta * Math.pow(1 - m, node.exponent) - drying);
}

/** Independent CPU source query: shared float32 pose, double density/noise math, no frame decoding. */
export function sampleCu(recipe, q, sourceTime) {
  if (!Number.isFinite(sourceTime) || q.length !== 3 || !q.every(Number.isFinite)) throw new TypeError('Cu query requires finite local metres/time');
  if (sourceTime <= 0 || sourceTime >= recipe.duration_s || q[1] <= 0) return 0;
  let sigma = 0;
  for (const node of recipe.cohorts) sigma = Math.max(sigma, coefficient(node, pose(node, sourceTime), q, sourceTime));
  const result = sigma * smooth(0, recipe.base_feather_m, q[1]);
  if (result > recipe.sigma_limit_m_inv || result < 0 || !Number.isFinite(result)) throw new RangeError('Cu coefficient outside admitted range; never clamp');
  return result;
}

/** Current node pose/material parameters, suitable for one readonly vec4f storage buffer. */
export function cuFrame(recipe, sourceTime) {
  if (!Number.isFinite(sourceTime)) throw new TypeError('Cu frame time must be finite');
  const out = new Float32Array(4 + STRIDE * recipe.cohorts.length);
  out.set([recipe.cohorts.length, sourceTime, recipe.duration_s, recipe.base_feather_m]);
  for (let i = 0; i < recipe.cohorts.length; i++) {
    const n = recipe.cohorts[i], p = pose(n, sourceTime), off = 4 + i * STRIDE;
    if (!Number.isFinite(n.sigma_m_inv) || n.sigma_m_inv < 0 || n.sigma_m_inv > SIGMA_LIMIT) throw new RangeError('Invalid Cu node extinction; never clamp');
    const active = p.active && sourceTime > 0 && sourceTime < recipe.duration_s;
    out.set([...p.centre, n.sigma_m_inv,
      ...(active ? p.radius.map(finiteReciprocal) : [0, 0, 0]), active ? 1 : 0,
      ...n.material_offset_m, finiteReciprocal(n.detail_m),
      ...n.drying_direction, n.stop_s,
      n.delay_base_s, n.delay_span_s, n.dry_duration_s, n.edge_width,
      n.erosion, n.exponent, n.secondary_frequency, n.secondary_weight,
      ...n.radii_m.map(finiteReciprocal), n.noise_seed,
      n.birth_s, n.parent, n.category === 'condensation body' ? 0 : 1, 0], off);
  }
  if (!out.every(Number.isFinite)) throw new RangeError('Cu frame contains a non-finite float32; upload rejected');
  return out;
}

export const CONVECTIVE_WGSL = /* wgsl */ `
@group(0) @binding(2) var<storage, read> cuNodeData: array<vec4f>;

fn cuHash(p: vec3i, seed: u32) -> u32 {
  var h = u32(p.x) * 374761393u + u32(p.y) * 668265263u + u32(p.z) * 2147483647u + seed * 1274126177u;
  h = (h ^ (h >> 13u)) * 1274126177u;
  return h ^ (h >> 16u);
}
fn cuCorner(p: vec3i, seed: u32) -> f32 { return f32(cuHash(p, seed) & 16777215u) / 16777216.0; }
fn cuNoise(q: vec3f, seed: u32) -> f32 {
  let lattice = vec3i(floor(q));
  let f = fract(q);
  let u = f * f * (vec3f(3.0) - 2.0 * f);
  let a = mix(cuCorner(lattice,seed), cuCorner(lattice+vec3i(1,0,0),seed),u.x);
  let b = mix(cuCorner(lattice+vec3i(0,1,0),seed),cuCorner(lattice+vec3i(1,1,0),seed),u.x);
  let c = mix(cuCorner(lattice+vec3i(0,0,1),seed),cuCorner(lattice+vec3i(1,0,1),seed),u.x);
  let d = mix(cuCorner(lattice+vec3i(0,1,1),seed),cuCorner(lattice+vec3i(1,1,1),seed),u.x);
  return mix(mix(a,b,u.y),mix(c,d,u.y),u.z);
}
fn canonicalSigma(q: vec3f, age: f32) -> f32 {
  let header = cuNodeData[0];
  if (age <= 0.0 || age >= header.z || q.y <= 0.0) { return 0.0; }
  var sigma = 0.0;
  for (var i=0u; i<u32(header.x); i=i+1u) {
    let k = 1u + i*8u;
    let centre = cuNodeData[k];
    let invR = cuNodeData[k+1u];
    if (invR.w < 0.5 || any(invR.xyz <= vec3f(0.0))) { continue; }
    let delta = q-centre.xyz;
    let radial = length(delta*invR.xyz);
    if (radial >= 1.0) { continue; }
    let m = 1.0-radial;
    let materialData = cuNodeData[k+2u];
    let dryingData = cuNodeData[k+3u];
    let terminal = cuNodeData[k+4u];
    let detail = cuNodeData[k+5u];
    let reference = cuNodeData[k+6u];
    let material = (delta+materialData.xyz)*materialData.w;
    let seed = u32(reference.w);
    let n0 = cuNoise(material,seed);
    let n1 = cuNoise(material*detail.z,seed+17u);
    let eta = detail.x*(0.25+0.75*mix(n0,n1,detail.w));
    let exposure = clamp(0.5+0.45*dot(delta*reference.xyz,dryingData.xyz)+0.18*(n0-0.5),0.0,1.0);
    let departure = dryingData.w+terminal.x+terminal.y*exposure;
    let drying = (1.0+terminal.w)*smoothstep(departure,departure+terminal.z,age);
    let rho = centre.w*smoothstep(0.0,terminal.w,m-eta*pow(1.0-m,detail.y)-drying);
    sigma = max(sigma,rho);
  }
  return sigma*smoothstep(0.0,header.w,q.y);
}
`;
