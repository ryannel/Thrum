import { airTransmittance, halfToFloat, floatToHalf } from "./reference.js";
export async function runGpuReferenceChecks(scene) {
  if (scene.referenceResult)
    return { ...scene.referenceResult, reusedWithinOpticalEpoch: true };
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
        for (let x = 0; x < 8; x++)
          data[((z * 8 + y) * 8 + x) * 4] = floatToHalf(
            typeof value === "function" ? value(x, y, z) : value,
          );
    g.device.queue.writeTexture(
      { texture: tex },
      data,
      { bytesPerRow: 64, rowsPerImage: 8 },
      [8, 8, 8],
    );
  }
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
  g.device.queue.writeBuffer(uniform, 0, params);
  const code =
    scene.codes.optics +
    `@group(0) @binding(15) var<storage,read_write> result:array<vec4f>; @compute @workgroup_size(1) fn main(){let tau=tauRay(vec3f(-2,0,0),vec3f(1,0,0),32u,1.);result[0]=vec4f(tau,exp(-tau),0,0);result[1]=vec4f(airRayT(vec3f(0,1000,0),vec3f(sqrt(.75),.5,0)),0);var integral=0.;for(var i=0u;i<2048u;i++){integral+=phase(-1.+(f32(i)+.5)*2./2048.,0.,1.)*2./2048.*2.*PI;}result[2]=vec4f(integral,0,0,0);}`;
  const pipeline = await g.compute("independent optical probes", code),
    result = g.buffer(
      "oracle optical result",
      48,
      GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
    ),
    mapped = g.buffer(
      "oracle optical readback",
      48,
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
  e.copyBufferToBuffer(result, 0, mapped, 0, 48);
  g.submit(e);
  await mapped.mapAsync(GPUMapMode.READ);
  const optical = Array.from(
    new Float32Array(mapped.getMappedRange().slice(0)),
  );
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
    analyticBeer: {
      actualTau: optical[0],
      expectedTau: 0.009,
      maxError: Math.abs(optical[0] - 0.009),
      tolerance: 1e-5,
    },
    clearAir: {
      actual: optical.slice(4, 7),
      reference: referenceAir,
      maxError: Math.max(
        ...referenceAir.map((v, i) => Math.abs(v - optical[4 + i])),
      ),
      tolerance: 0.005,
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
