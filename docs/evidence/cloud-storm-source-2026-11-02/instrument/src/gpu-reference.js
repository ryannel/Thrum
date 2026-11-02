import { airTransmittance, halfToFloat, floatToHalf } from "./reference.js";
export async function runGpuReferenceChecks(scene) {
  if (scene.referenceResult)
    return { ...scene.referenceResult, reusedForSameDeviceAndShader: true };
  const g = scene.gpu;
  if (!scene.sourcePipeline)
    throw Error("Numerical source checks need the live volume provider");
  await g.device.queue.onSubmittedWorkDone();
  const old = g.texture("oracle old", [8, 8, 8]),
    forward = g.texture("oracle forward", [8, 8, 8]),
    reverse = g.texture("oracle reverse", [8, 8, 8]),
    out = g.texture("oracle result", [8, 8, 8]);
  const uniform = g.buffer(
      "oracle uniform",
      160,
      GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    ),
    emitters = g.buffer(
      "oracle emitters",
      1024,
      GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    ),
    read = g.buffer(
      "oracle source readback",
      16384,
      GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    );
  const params = new Float32Array(40);
  params.set([-4000, 1200, -4000, 0.0045], 12);
  params.set([4000, 1240, 4000, 0.0018], 16);
  params.set([10, 0.25, 1, 0], 20);
  params.set([0, 0, 0, 0], 24);
  params.set([1e6, 0, 71, 1], 28);
  params.set([1, 0.25, 0, 3], 36);
  function upload(tex, value) {
    const data = new Uint16Array(8 * 8 * 8 * 4);
    for (let z = 0; z < 8; z++)
      for (let y = 0; y < 8; y++)
        for (let x = 0; x < 8; x++) {
          const sample = typeof value === "function" ? value(x, y, z) : value;
          const channels = Array.isArray(sample) ? sample : [sample, 0, 0, 0];
          data.set(channels.map(floatToHalf), ((z * 8 + y) * 8 + x) * 4);
        }
    g.device.queue.writeTexture(
      { texture: tex },
      data,
      { bytesPerRow: 64, rowsPerImage: 8 },
      [8, 8, 8],
    );
  }
  let lastSourceRaw;
  async function execute() {
    g.device.queue.writeBuffer(uniform, 0, params);
    const e = g.device.createCommandEncoder();
    scene.compute(
      e,
      scene.sourcePipeline,
      scene.sourceGroup(
        uniform,
        old,
        forward,
        reverse,
        out,
        scene.rest[scene.currentRest],
        emitters,
      ),
      [8, 8, 8],
    );
    e.copyTextureToBuffer(
      { texture: out },
      { buffer: read, bytesPerRow: 256, rowsPerImage: 8 },
      [8, 8, 8],
    );
    g.submit(e);
    await read.mapAsync(GPUMapMode.READ);
    const raw = new Uint16Array(read.getMappedRange().slice(0));
    read.unmap();
    lastSourceRaw = raw;
    const values = [];
    for (let z = 2; z < 6; z++)
      for (let y = 2; y < 6; y++)
        for (let x = 2; x < 6; x++)
          values.push(halfToFloat(raw[(z * 8 + y) * 128 + x * 4]));
    return values;
  }
  upload(old, 0.4);
  upload(forward, 0.4);
  upload(reverse, 0.4);
  const neutral = await execute(),
    q = halfToFloat(floatToHalf(0.4)),
    expectedNeutral = q * Math.exp(-0.004 * 0.25);
  params[28] = -1e6;
  const dry = await execute(),
    expectedDry = q * Math.exp(-0.049 * 0.25);
  params.set([0, 0, 0, 0.0045], 12);
  params.set([8, 8, 8, 0.0018], 16);
  params[28] = 1e6;
  params[39] = 1;
  upload(old, (x) => (x + 0.5) / 8);
  const linear = await execute();
  let linearError = 0,
    index = 0;
  for (let z = 2; z < 6; z++)
    for (let y = 2; y < 6; y++)
      for (let x = 2; x < 6; x++)
        linearError = Math.max(
          linearError,
          Math.abs(linear[index++] - (x + 0.5 - 0.25) / 8),
        );
  params.set([-4000, 1200, -4000, 0.0045], 12);
  params.set([4000, 1240, 4000, 0.0018], 16);
  params.set([1, 0, 0, 0], 24);
  params[39] = 3;
  upload(old, 0);
  upload(forward, 0);
  upload(reverse, 0);
  const pulse = new Float32Array(256);
  pulse.set([0, 0, 4000, 1, 0, 1, 30, 10]);
  g.device.queue.writeBuffer(emitters, 0, pulse);
  const noLift = await execute();
  params[26] = 1;
  const birth = await execute();
  params.set([-5000, 1200, -4000, 0.0045], 12);
  params.set([5000, 1240, 4000, 0.0018], 16);
  const banks = {};
  for (const kind of [2, 3]) {
    params[27] = kind;
    params[39] = 0;
    const interior = await execute();
    let edgePeak = 0;
    for (let z = 0; z < 8; z++)
      for (let y = 0; y < 8; y++)
        for (let x = 0; x < 8; x++)
          if (x === 0 || x === 7 || z === 0 || z === 7)
            edgePeak = Math.max(
              edgePeak,
              halfToFloat(lastSourceRaw[(z * 8 + y) * 128 + x * 4]),
            );
    banks[kind] = { edgePeak, interiorPeak: Math.max(...interior) };
  }
  params[27] = 1;
  params.set([-2500, 1100, -2500, 0.0045], 12);
  params.set([2500, 12500, 2500, 0.0018], 16);
  await execute();
  let seededStormHighPeak = 0,
    seededStormLowPeak = 0;
  for (let z = 0; z < 8; z++)
    for (let y = 0; y < 8; y++)
      for (let x = 0; x < 8; x++) {
        const v = halfToFloat(lastSourceRaw[(z * 8 + y) * 128 + x * 4]);
        if (1100 + ((y + 0.5) * 11400) / 8 > 3200)
          seededStormHighPeak = Math.max(seededStormHighPeak, v);
        else seededStormLowPeak = Math.max(seededStormLowPeak, v);
      }
  params.set([-100, 500, -100, 0.0045], 12);
  params.set([100, 800, 100, 0.0018], 16);
  params.set([1, 0, 1, 1], 24);
  params[39] = 3;
  upload(old, 0);
  upload(forward, 0);
  upload(reverse, 0);
  const belowBaseBirth = await execute();
  params.set([-100, 9400, -100, 0.0045], 12);
  params.set([100, 9800, 100, 0.0018], 16);
  params.set([10, 0.25, 0, 0], 20);
  params.set([0, 0, 1, 1], 24);
  params.set([1e6, 1, 71, 1], 28);
  params[39] = 3;
  upload(old, 0.4);
  upload(forward, 0.4);
  upload(reverse, 0.4);
  await execute();
  let freezeSumError = 0,
    freezeIcePeak = 0;
  const decayedLiquid =
    halfToFloat(floatToHalf(0.4)) * Math.exp(-0.0015 * 0.25);
  for (let z = 2; z < 6; z++)
    for (let y = 2; y < 6; y++)
      for (let x = 2; x < 6; x++) {
        const offset = (z * 8 + y) * 128 + x * 4;
        const liquid = halfToFloat(lastSourceRaw[offset]),
          ice = halfToFloat(lastSourceRaw[offset + 1]);
        freezeSumError = Math.max(
          freezeSumError,
          Math.abs(liquid + ice - decayedLiquid),
        );
        freezeIcePeak = Math.max(freezeIcePeak, ice);
      }
  params.set([-100, 5000, -100, 0.0045], 12);
  params.set([100, 5400, 100, 0.0018], 16);
  params.set([0, 0.7, 0, 1], 24);
  upload(old, [0.2, 0.2, 100, 0]);
  upload(forward, [0.2, 0.2, 100, 0]);
  upload(reverse, [0.2, 0.2, 100, 0]);
  await execute();
  let noSourceMixedPeak = 0,
    mixedAgeError = 0;
  for (let z = 2; z < 6; z++)
    for (let y = 2; y < 6; y++)
      for (let x = 2; x < 6; x++) {
        const offset = (z * 8 + y) * 128 + x * 4;
        noSourceMixedPeak = Math.max(
          noSourceMixedPeak,
          Math.abs(halfToFloat(lastSourceRaw[offset + 3])),
        );
        mixedAgeError = Math.max(
          mixedAgeError,
          Math.abs(halfToFloat(lastSourceRaw[offset + 2]) - 100.25),
        );
      }
  const mipFixture = g.texture(
    "mip mean fixture",
    [2, 2, 2],
    "rgba8unorm",
    0,
    2,
  );
  const mipInput = new Uint8Array(32);
  for (let i = 0; i < 8; i++) mipInput.set([i * 32, 64, 128, 255], i * 4);
  g.device.queue.writeTexture(
    { texture: mipFixture },
    mipInput,
    { bytesPerRow: 8, rowsPerImage: 2 },
    [2, 2, 2],
  );
  const mipPipeline = await g.compute("independent mip mean", scene.codes.mip);
  const mipRead = g.buffer(
    "mip readback",
    256,
    GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
  );
  const mipEncoder = g.device.createCommandEncoder();
  scene.compute(
    mipEncoder,
    mipPipeline,
    g.group(mipPipeline, [
      [0, mipFixture.createView({ baseMipLevel: 0, mipLevelCount: 1 })],
      [1, mipFixture.createView({ baseMipLevel: 1, mipLevelCount: 1 })],
    ]),
    [1, 1, 1],
  );
  mipEncoder.copyTextureToBuffer(
    { texture: mipFixture, mipLevel: 1 },
    { buffer: mipRead, bytesPerRow: 256 },
    [1, 1, 1],
  );
  g.submit(mipEncoder);
  await mipRead.mapAsync(GPUMapMode.READ);
  const mipMean = Array.from(
    new Uint8Array(mipRead.getMappedRange().slice(0, 4)),
  );
  mipRead.unmap();
  params[27] = 0;
  const ones = g.texture("oracle constant material", [1, 1, 1], "rgba8unorm");
  g.device.queue.writeTexture(
    { texture: ones },
    new Uint8Array([255, 255, 255, 255]),
    { bytesPerRow: 4, rowsPerImage: 1 },
    [1, 1, 1],
  );
  upload(old, 1);
  params.set([-1, -1, -1, 0.0045], 12);
  params.set([1, 1, 1, 0.0018], 16);
  params.set([1, 0.25, 0, 0], 36);
  params.set([0.8, 0.6, 0, 1], 8);
  params.set([0.85, 0.08, 0, 0], 24);
  params[23] = 25;
  g.device.queue.writeBuffer(uniform, 0, params);
  const code =
    scene.codes.optics +
    scene.codes.view +
    `@group(0) @binding(15) var<storage,read_write> result:array<vec4f>; @compute @workgroup_size(1) fn main(){let tau=tauRay(vec3f(-2,0,0),vec3f(1,0,0),32u,1.);result[0]=vec4f(tau,exp(-tau),0,0);result[1]=vec4f(airRayT(vec3f(0,1000,0),vec3f(sqrt(.75),.5,0)),0);var integral=0.;for(var i=0u;i<2048u;i++){integral+=phase(-1.+(f32(i)+.5)*2./2048.,0.,1.)*2./2048.*2.*PI;}result[2]=vec4f(integral,0,0,0);let j=vec3f(.2,.1,.03);let sigma=vec3f(.4,.2,.1);let full=beer(j,sigma,2.);let part=beer(j,sigma,1.);let split=join(part,part);result[3]=vec4f(full.s,0.);result[4]=vec4f(full.t,0.);result[5]=vec4f(max(max(abs(full.s-split.s).x,abs(full.s-split.s).y),abs(full.s-split.s).z),max(max(abs(full.t-split.t).x,abs(full.t-split.t).y),abs(full.t-split.t).z),0,0);result[6]=vec4f(beer(j,vec3f(0.,1e-9,1e-6),2.).s,0);result[7]=vec4f(analyticShadowT(vec3f(0)),analyticShadowT(vec3f(800,600,0)),analyticShadowT(vec3f(0,2300,0)),0);let mixture=shapeMatter(vec2f(.2,.2),.6,.5,1.);let liquid=shapeMatter(vec2f(.4,0),.6,.5,1.);let ice=shapeMatter(vec2f(0,.4),.6,.5,1.);result[8]=vec4f(mixture,liquid.x,ice.y);}`;
  const pipeline = await g.compute("independent optical probes", code),
    result = g.buffer(
      "oracle optical result",
      144,
      GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
    ),
    mapped = g.buffer(
      "oracle optical readback",
      144,
      GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    );
  const bg = g.group(pipeline, [
    [0, uniform],
    [1, old.createView()],
    [2, ones.createView()],
    [3, scene.clamp],
    [4, scene.rest[scene.currentRest].createView()],
    [10, old.createView()],
    [11, scene.rest[scene.currentRest].createView()],
    [14, scene.repeat],
    [15, result],
  ]);
  const e = g.device.createCommandEncoder();
  scene.compute(e, pipeline, bg, [1, 1, 1]);
  e.copyBufferToBuffer(result, 0, mapped, 0, 144);
  g.submit(e);
  await mapped.mapAsync(GPUMapMode.READ);
  const optical = Array.from(
    new Float32Array(mapped.getMappedRange().slice(0)),
  );
  mapped.unmap();
  params[27] = 1;
  g.device.queue.writeBuffer(uniform, 0, params);
  const cbEncoder = g.device.createCommandEncoder();
  scene.compute(cbEncoder, pipeline, bg, [1, 1, 1]);
  cbEncoder.copyBufferToBuffer(result, 0, mapped, 0, 144);
  g.submit(cbEncoder);
  await mapped.mapAsync(GPUMapMode.READ);
  const cbMorphology = Array.from(
    new Float32Array(mapped.getMappedRange().slice(0)),
  ).slice(32, 36);
  mapped.unmap();
  const referenceAir = airTransmittance(1000, 0.5);
  const error = (values, expected) =>
    Math.max(...values.map((v) => Math.abs(v - expected)));
  const checks = {
    neutralReaction: {
      maxError: error(neutral, expectedNeutral),
      tolerance: 5e-4,
    },
    crossedFrontReaction: {
      maxError: error(dry, expectedDry),
      tolerance: 5e-4,
    },
    linearAdvection: { maxError: linearError, tolerance: 5e-4 },
    noBirthWithoutLift: {
      maxError: Math.max(...noLift.map(Math.abs)),
      tolerance: 1e-7,
    },
    birthInEmptyField: {
      max: Math.max(...birth),
      pass: birth.some((v) => v > 0.002),
    },
    finiteLayerSupport: {
      banks,
      pass: Object.values(banks).every(
        (b) => b.edgePeak === 0 && b.interiorPeak > 0.05,
      ),
    },
    cbNoBirthBelowCondensationBase: {
      maxError: Math.max(...belowBaseBirth.map(Math.abs)),
      tolerance: 1e-7,
    },
    mixedPhaseNoPhantomBirth: { maxError: noSourceMixedPeak, tolerance: 1e-7 },
    mixedPhaseAgeWithoutBirth: { maxError: mixedAgeError, tolerance: 0.063 },
    phaseTransferConservesProxy: {
      maxError: freezeSumError,
      tolerance: 5e-4,
      icePeak: freezeIcePeak,
      pass: freezeSumError <= 5e-4 && freezeIcePeak > 1e-4,
    },
    nativeMipMean: {
      actual: mipMean,
      pass: mipMean.every((v, i) => v === [112, 64, 128, 255][i]),
    },
    lowBodyStormSeed: {
      highPeak: seededStormHighPeak,
      lowPeak: seededStormLowPeak,
      pass: seededStormHighPeak === 0 && seededStormLowPeak > 0.05,
    },
    cbPhaseCoherentMorphology: {
      actual: cbMorphology,
      maxError: Math.max(
        ...[0.16168, 0.16168, 0.32336, 0.32336].map((v, i) =>
          Math.abs(v - cbMorphology[i]),
        ),
      ),
      tolerance: 1e-6,
    },
    phaseCoherentMorphology: {
      actual: optical.slice(32, 36),
      maxError: Math.max(
        ...[0.15, 0.15, 0.3, 0.3].map((v, i) => Math.abs(v - optical[32 + i])),
      ),
      tolerance: 1e-6,
    },
    opticalDepth: {
      actualTau: optical[0],
      expectedTau: 0.009,
      maxError: Math.abs(optical[0] - 0.009),
      tolerance: 1e-5,
    },
    beerTransmittance: {
      maxError: Math.abs(optical[1] - Math.exp(-0.009)),
      tolerance: 1e-6,
    },
    homogeneousSegment: {
      maxError: Math.max(
        ...[0.4, 0.2, 0.1].map((s, i) =>
          Math.abs(
            optical[12 + i] -
              ([0.2, 0.1, 0.03][i] / s) * (1 - Math.exp(-s * 2)),
          ),
        ),
      ),
      tolerance: 1e-6,
    },
    thinAndVacuumLimit: {
      maxError: Math.max(
        ...[0, 1e-9, 1e-6].map((s, i) =>
          Math.abs(
            optical[24 + i] -
              [0.2, 0.1, 0.03][i] * (s === 0 ? 2 : -Math.expm1(-s * 2) / s),
          ),
        ),
      ),
      tolerance: 1e-6,
    },
    segmentComposition: {
      maxError: Math.max(optical[20], optical[21]),
      tolerance: 1e-6,
    },
    clearAir: {
      actual: optical.slice(4, 7),
      reference: referenceAir,
      maxError: Math.max(
        ...referenceAir.map((v, i) => Math.abs(v - optical[4 + i])),
      ),
      tolerance: 0.005,
    },
    analyticAttenuationProvider: {
      maxError: Math.max(
        Math.abs(
          optical[28] -
            Math.exp(
              -(0.55 + 0.35 * Math.sin(((2200 * 0.8) / 0.6 - 25) * 0.0012)) *
                0.85 *
                0.92 *
                1.8,
            ),
        ),
        Math.abs(optical[28] - optical[29]),
        Math.abs(optical[30] - 1),
      ),
      tolerance: 5e-5,
    },
    phaseNormalization: {
      integral: optical[8],
      maxError: Math.abs(optical[8] - 1),
      tolerance: 0.001,
    },
  };
  for (const c of Object.values(checks))
    if (c.pass === undefined)
      c.pass = Number.isFinite(c.maxError) && c.maxError <= c.tolerance;
  scene.referenceResult = {
    scope:
      "Finite interior GPU advection/reaction/front/source and optical-unit checks; not full morphology/cache/temporal qualification",
    pass: Object.values(checks).every((c) => c.pass) && g.errors.length === 0,
    checks,
    errors: [...g.errors],
  };
  return scene.referenceResult;
}
