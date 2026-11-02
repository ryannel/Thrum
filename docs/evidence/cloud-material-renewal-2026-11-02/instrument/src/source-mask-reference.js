// Whole-field differential check: optimized work selection must preserve decoded RGBA16F.
export async function compareSourceMask(scene) {
  if (
    scene.maskReference?.epoch === scene.epoch &&
    scene.maskReference?.sourceTime === scene.time
  )
    return { ...scene.maskReference, reusedForSameSourceSnapshot: true };
  if (scene.family !== 0 || !scene.sourcePipeline)
    throw Error("Mask oracle requires the Cu volume provider");
  const g = scene.gpu;
  await g.device.queue.onSubmittedWorkDone();
  const dims = scene.dims;
  scene.ensureVelocityField();
  scene.maskFixture ??= await makeFixture(scene);
  const {
    old,
    dense,
    masked,
    fwd,
    rev,
    uniform,
    emit,
    counters,
    read,
    pipeline,
    group,
  } = scene.maskFixture;
  const params = new Float32Array(44),
    pulse = new Float32Array(256);
  params.set([...scene.bounds()[0], 0.0045], 12);
  params.set([...scene.bounds()[1], 0.0018], 16);
  params.set([10, 0.25, 3, 0], 20);
  params.set([0.85, 0.08, 1, 0], 24);
  params.set([1e6, 0, 71, 1], 28);
  params.set([dims[0], 0.25, 0, 1], 36);
  const rest = scene.rest[scene.currentRest];
  const records = [];
  for (const fixture of ["empty-birth", "existing-source", "edge-birth"]) {
    const init = g.device.createCommandEncoder();
    if (fixture === "existing-source")
      init.copyTextureToTexture(
        { texture: scene.q[scene.current] },
        { texture: old },
        dims,
      );
    else {
      // WebGPU zero initialization for a new field; clear an already-used input explicitly.
      g.device.queue.writeTexture(
        { texture: old },
        new Uint16Array(dims.reduce((a, b) => a * b, 1) * 4),
        { bytesPerRow: dims[0] * 8, rowsPerImage: dims[1] },
        dims,
      );
    }
    g.submit(init);
    await g.device.queue.onSubmittedWorkDone();
    for (let step = 0; step < 12; step++) {
      pulse.fill(0);
      pulse.set([
        fixture === "edge-birth" ? -4900 : 0,
        0,
        600,
        1,
        0,
        1,
        30,
        10,
      ]);
      g.device.queue.writeBuffer(emit, 0, pulse);
      params[20] = 10 + step * 0.25;
      params[22] = step >= 6 ? -12 : 3;
      params[26] = step >= 6 ? 2 : 1;
      params[28] = step >= 6 ? -300 : 1e6;
      for (const skip of [false, true]) {
        const output = skip ? masked : dense;
        for (const stage of [1, 2, 3]) {
          params[3] = skip ? 1 : 0;
          params[38] = skip ? 1 : 0;
          params[39] = stage;
          g.device.queue.writeBuffer(uniform, 0, params);
          const e = g.device.createCommandEncoder();
          if (skip && stage === 1) {
            const sg = g.group(scene.supportPipeline, [
              [0, uniform],
              [1, old.createView()],
              [2, emit],
              [3, scene.support],
            ]);
            const sp = e.beginComputePass();
            sp.setPipeline(scene.supportPipeline);
            sp.setBindGroup(0, sg);
            sp.dispatchWorkgroups(...dims.map((v) => Math.ceil(v / 8)));
            sp.end();
            const dg = g.group(scene.dilatePipeline, [
              [0, old.createView()],
              [1, scene.support],
              [2, scene.active],
            ]);
            scene.compute(
              e,
              scene.dilatePipeline,
              dg,
              dims.map((v) => Math.ceil(v / 8)),
            );
          }
          if (skip && stage === 1)
            scene.compute(
              e,
              scene.velocityPipeline,
              scene.velocityGroup(uniform, old),
              dims,
            );
          scene.compute(
            e,
            scene.sourcePipeline,
            scene.sourceGroup(
              uniform,
              old,
              stage === 1 ? old : fwd,
              stage === 3 ? rev : old,
              stage === 1 ? fwd : stage === 2 ? rev : output,
              rest,
              emit,
            ),
            dims,
          );
          g.submit(e);
          await g.device.queue.onSubmittedWorkDone();
        }
      }
      const e = g.device.createCommandEncoder();
      e.clearBuffer(counters);
      scene.compute(e, pipeline, group, dims);
      e.copyBufferToBuffer(counters, 0, read, 0, 16);
      e.copyTextureToTexture({ texture: masked }, { texture: old }, dims);
      g.submit(e);
      await read.mapAsync(GPUMapMode.READ);
      const values = Array.from(
        new Uint32Array(read.getMappedRange().slice(0)),
      );
      read.unmap();
      records.push({
        fixture,
        step,
        differentVoxels: values[0],
        invalidVoxels: values[1],
        maxAbsDifference: new Float32Array(
          new Uint32Array([values[2]]).buffer,
        )[0],
        activeBricks: values[3],
      });
    }
  }
  const queued = await compareQueuedGraphs(scene, scene.maskFixture);
  records.push(...queued.records);
  scene.maskReference = {
    queuedGraphsHighWater: queued.highWater,
    epoch: scene.epoch,
    sourceTime: scene.time,
    scope:
      "36 serial plus24 batched full-resolution source ticks across clear birth, existing source, edge birth and changed wind/lift/front; three queued eight-tick graphs; mask plus shared float32 velocity equal direct dense decoded RGBA16F all four channels; carrier unchanged",
    pass:
      records.every((r) => !r.differentVoxels && !r.invalidVoxels) &&
      g.errors.length === 0,
    records,
    errors: [...g.errors],
    oracleLogicalBytes: scene.maskFixture.logicalBytes,
    sharedVelocityBytes: scene.velocityField.size,
  };
  return scene.maskReference;
}

async function makeFixture(scene) {
  const g = scene.gpu;
  const byteStart = g.bytes;
  const dims = scene.dims;
  const old = g.texture("mask oracle input", dims),
    dense = g.texture("mask oracle dense", dims),
    masked = g.texture("mask oracle masked", dims),
    fwd = g.texture("mask oracle forward", dims),
    rev = g.texture("mask oracle reverse", dims);
  const uniform = g.buffer(
      "mask oracle uniform",
      176,
      GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    ),
    emit = g.buffer(
      "mask oracle emitters",
      1024,
      GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    ),
    counters = g.buffer(
      "mask differential counters",
      16,
      GPUBufferUsage.STORAGE |
        GPUBufferUsage.COPY_SRC |
        GPUBufferUsage.COPY_DST,
    ),
    read = g.buffer(
      "mask differential readback",
      16,
      GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    );
  const code = `@group(0) @binding(0) var a:texture_3d<f32>;@group(0) @binding(1) var b:texture_3d<f32>;
  @group(0) @binding(2) var<storage,read_write> counts:array<atomic<u32>,4>;
  @group(0) @binding(3) var<storage,read> mask:array<u32>;
  @compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){let n=textureDimensions(a);if(any(id>=n)){return;}
  let av=textureLoad(a,vec3i(id),0);let bv=textureLoad(b,vec3i(id),0);
  if(any(av!=bv)){atomicAdd(&counts[0],1u);}
  if(any(av!=av)||any(bv!=bv)||any(abs(av)>vec4f(65504))||any(abs(bv)>vec4f(65504))){atomicAdd(&counts[1],1u);}
  let d=abs(av-bv);atomicMax(&counts[2],bitcast<u32>(max(max(d.x,d.y),max(d.z,d.w))));
  let flat=(id.z*n.y+id.y)*n.x+id.x;if(flat<arrayLength(&mask)){atomicAdd(&counts[3],mask[flat]);}}
  `;
  const pipeline = await g.compute("exact source mask differential", code);
  const group = g.group(pipeline, [
    [0, masked.createView()],
    [1, dense.createView()],
    [2, counters],
    [3, scene.active],
  ]);

  return {
    logicalBytes: g.bytes - byteStart,
    old,
    dense,
    masked,
    fwd,
    rev,
    uniform,
    emit,
    counters,
    read,
    pipeline,
    group,
  };
}

async function compareQueuedGraphs(scene, f) {
  const g = scene.gpu,
    dims = scene.dims;
  await g.device.queue.onSubmittedWorkDone();
  if (!f.batches) {
    const before = g.bytes;
    f.batches = Array.from({ length: 3 }, (_, job) => ({
      uniforms: Array.from({ length: 48 }, (_, i) =>
        g.buffer(
          `queued oracle uniform ${job}/${i}`,
          176,
          GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
        ),
      ),
      emitters: Array.from({ length: 8 }, (_, i) =>
        g.buffer(
          `queued oracle emitter ${job}/${i}`,
          1024,
          GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        ),
      ),
      read: g.buffer(
        `queued oracle read ${job}`,
        16,
        GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
      ),
    }));
    f.logicalBytes += g.bytes - before;
  }
  const records = [],
    p = new Float32Array(44),
    pulse = new Float32Array(256);
  p.set([...scene.bounds()[0], 0.0045], 12);
  p.set([...scene.bounds()[1], 0.0018], 16);
  p.set([10, 0.25, 3, 0], 20);
  p.set([0.85, 0.08, 1, 0], 24);
  p.set([1e6, 0, 71, 1], 28);
  p.set([dims[0], 0.25, 0, 1], 36);
  let highWater = 0;
  for (let job = 0; job < 3; job++) {
    const b = f.batches[job],
      e = g.device.createCommandEncoder();
    e.clearBuffer(f.counters);
    if (job === 1)
      e.copyTextureToTexture(
        { texture: scene.q[scene.current] },
        { texture: f.old },
        dims,
      );
    else
      g.device.queue.writeTexture(
        { texture: f.old },
        new Uint16Array(dims.reduce((a, b) => a * b, 1) * 4),
        { bytesPerRow: dims[0] * 8, rowsPerImage: dims[1] },
        dims,
      );
    for (let step = 0; step < 8; step++) {
      pulse.fill(0);
      pulse.set([job === 2 ? -4900 : 0, 0, 600, 1, 0, 1, 30, 10]);
      g.device.queue.writeBuffer(b.emitters[step], 0, pulse);
      p[20] = 10 + step * 0.25;
      p[22] = step >= 4 ? -12 : 3;
      p[26] = step >= 4 ? 2 : 1;
      p[28] = step >= 4 ? -300 : 1e6;
      for (let variant = 0; variant < 2; variant++)
        for (let stage = 1; stage <= 3; stage++) {
          const u = b.uniforms[step * 6 + variant * 3 + stage - 1];
          p[3] = variant;
          p[38] = variant;
          p[39] = stage;
          g.device.queue.writeBuffer(u, 0, p);
          if (variant === 1 && stage === 1) {
            const sg = g.group(scene.supportPipeline, [
              [0, u],
              [1, f.old.createView()],
              [2, b.emitters[step]],
              [3, scene.support],
            ]);
            const sp = e.beginComputePass();
            sp.setPipeline(scene.supportPipeline);
            sp.setBindGroup(0, sg);
            sp.dispatchWorkgroups(...dims.map((v) => Math.ceil(v / 8)));
            sp.end();
            const dg = g.group(scene.dilatePipeline, [
              [0, f.old.createView()],
              [1, scene.support],
              [2, scene.active],
            ]);
            scene.compute(
              e,
              scene.dilatePipeline,
              dg,
              dims.map((v) => Math.ceil(v / 8)),
            );
            scene.compute(
              e,
              scene.velocityPipeline,
              scene.velocityGroup(u, f.old),
              dims,
            );
          }
          scene.compute(
            e,
            scene.sourcePipeline,
            scene.sourceGroup(
              u,
              f.old,
              stage === 1 ? f.old : f.fwd,
              stage === 3 ? f.rev : f.old,
              stage === 1
                ? f.fwd
                : stage === 2
                  ? f.rev
                  : variant === 1
                    ? f.masked
                    : f.dense,
              scene.rest[scene.currentRest],
              b.emitters[step],
            ),
            dims,
          );
        }
      // Keep every mismatch even if a later tick reconverges. Equal paths may share the next input.
      scene.compute(e, f.pipeline, f.group, dims);
      e.copyTextureToTexture({ texture: f.masked }, { texture: f.old }, dims);
    }
    e.copyBufferToBuffer(f.counters, 0, b.read, 0, 16);
    g.submit(e);
    highWater = Math.max(highWater, g.pending);
  }
  await Promise.all(
    f.batches.map(async (b, job) => {
      await b.read.mapAsync(GPUMapMode.READ);
      const a = Array.from(new Uint32Array(b.read.getMappedRange().slice(0)));
      b.read.unmap();
      records.push({
        fixture: ["queued-empty", "queued-existing", "queued-edge"][job],
        queuedGraph: job,
        steps: 8,
        differentVoxels: a[0],
        invalidVoxels: a[1],
        maxAbsDifference: new Float32Array(new Uint32Array([a[2]]).buffer)[0],
        activeBrickVisits: a[3],
      });
    }),
  );
  records.sort((a, b) => a.queuedGraph - b.queuedGraph);
  return { records, highWater };
}
