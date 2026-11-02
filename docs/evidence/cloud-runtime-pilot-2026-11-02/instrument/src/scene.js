import { Gpu, Timings } from "./gpu.js";
import { SourceDriver } from "./source.js";
const UNIFORM_BYTES = 160;
export class Scene {
  constructor(gpu, codes, provider = 0) {
    Object.assign(this, {
      gpu,
      codes,
      provider,
      time: 0,
      steps: 0,
      epoch: 1,
      current: 0,
      currentRest: 0,
      frame: 0,
      dims: provider === 0 ? [96, 80, 80] : [1, 1, 1],
      lightDims: provider === 0 ? [32, 24, 24] : [1, 1, 1],
      viewSize: [640, 360],
      family: 0,
      lastControl: null,
      lastSubmission: null,
    });
    this.restDims = this.dims.map((v) => Math.max(1, v / 2));
    this.camera = { yaw: 0.3, pitch: -0.18, distance: 7500, panX: 0, panY: 0 };
    this.timings = new Timings(gpu);
    this.driver = new SourceDriver();
    this.groups = new Map();
    this.scratch = new Float32Array(40);
  }
  static async create(canvas, progress, provider = 0) {
    const gpu = await Gpu.open(canvas);
    progress("Compiling the selected provider…");
    const names = [
      "source",
      "noise",
      "curl",
      "optics",
      "light",
      "shadow",
      "air",
      "view",
      "display",
    ];
    const codes = Object.fromEntries(
      await Promise.all(
        names.map(async (n) => [
          n,
          await fetch("shaders/" + n + ".wgsl").then((r) => {
            if (!r.ok) throw Error("Shader load failed: " + n + " " + r.status);
            return r.text();
          }),
        ]),
      ),
    );
    const s = new Scene(gpu, codes, provider);
    try {
      await s.init(progress);
      return s;
    } catch (error) {
      await s.dispose();
      throw error;
    }
  }
  async init(progress) {
    const g = this.gpu,
      d = g.device;
    this.repeat = d.createSampler({
      magFilter: "linear",
      minFilter: "linear",
      addressModeU: "repeat",
      addressModeV: "repeat",
      addressModeW: "repeat",
    });
    this.clamp = d.createSampler({
      magFilter: "linear",
      minFilter: "linear",
      addressModeU: "clamp-to-edge",
      addressModeV: "clamp-to-edge",
      addressModeW: "clamp-to-edge",
    });
    if (this.provider === 0) {
      this.sourcePipeline = await g.compute(
        "source / bounded MacCormack",
        this.codes.source,
      );
      this.noisePipeline = await g.compute(
        "native volume basis",
        this.codes.noise,
      );
      this.curlPipeline = await g.compute(
        "coherent curl velocity",
        this.codes.curl,
      );
      this.lightPipeline = await g.compute(
        "current point lighting",
        this.codes.optics + this.codes.light,
      );
      this.shadowPipeline = await g.compute(
        "receiver Beer field",
        this.codes.optics + this.codes.shadow,
      );
    }
    this.airPipeline = await g.compute(
      "clear-air transmittance",
      this.codes.optics + this.codes.air,
    );
    this.viewPipeline = await g.render(
      "joint view and reflected rays",
      this.codes.optics + this.codes.view,
      "rgba16float",
    );
    this.displayPipeline = await g.render(
      "display",
      this.codes.display,
      g.format,
    );
    this.q = [0, 1].map((i) => g.texture("optical q " + i, this.dims));
    this.fwd = g.texture("advection forward", this.dims);
    this.rev = g.texture("advection reverse", this.dims);
    this.rest = [0, 1].map((i) =>
      g.texture("material coordinates " + i, this.restDims),
    );
    this.noise = g.texture(
      "native basis",
      this.provider === 0 ? [64, 64, 64] : [1, 1, 1],
      "rgba8unorm",
    );
    this.curl = g.texture(
      "coherent curl field",
      this.provider === 0 ? [64, 64, 64] : [1, 1, 1],
    );
    this.light = g.texture("current voxel light", this.lightDims);
    this.receiver = g.texture(
      "receiver Beer field",
      this.provider === 0 ? [128, 128] : [1, 1],
    );
    this.air = g.texture("air transmittance", [256, 64]);
    this.view = g.texture(
      "current optical image",
      this.viewSize,
      "rgba16float",
      GPUTextureUsage.RENDER_ATTACHMENT,
    );
    this.uniforms = Array.from({ length: 3 }, (_, f) =>
      Array.from({ length: 33 }, (_, s) =>
        g.buffer(
          `uniform ${f}/${s}`,
          UNIFORM_BYTES,
          GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
        ),
      ),
    );
    this.emitters = Array.from({ length: 3 }, (_, f) =>
      Array.from({ length: 8 }, (_, s) =>
        g.buffer(
          `emitters ${f}/${s}`,
          1024,
          GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        ),
      ),
    );
    progress("Preparing clear-air and source ingredients…");
    const e = d.createCommandEncoder();
    if (this.noisePipeline) {
      const bg = g.group(this.noisePipeline, [[0, this.noise.createView()]]);
      const pass = e.beginComputePass();
      pass.setPipeline(this.noisePipeline);
      pass.setBindGroup(0, bg);
      pass.dispatchWorkgroups(16, 16, 16);
      pass.end();
      const cg = g.group(this.curlPipeline, [[0, this.curl.createView()]]);
      const cp = e.beginComputePass();
      cp.setPipeline(this.curlPipeline);
      cp.setBindGroup(0, cg);
      cp.dispatchWorkgroups(16, 16, 16);
      cp.end();
    }
    const bg = g.group(this.airPipeline, [[8, this.air.createView()]]);
    const pass = e.beginComputePass();
    pass.setPipeline(this.airPipeline);
    pass.setBindGroup(0, bg);
    pass.dispatchWorkgroups(32, 8);
    pass.end();
    g.submit(e);
    await d.queue.onSubmittedWorkDone();
  }
  bounds() {
    if (this.family === 1)
      return [
        [-7000, 500, -7000],
        [7000, 15000, 7000],
      ];
    if (this.family === 4)
      return [
        [-6500, 7800, -6000],
        [6500, 12500, 6000],
      ];
    return [
      [-5000, 400, -4000],
      [5000, 5600, 4000],
    ];
  }
  target() {
    if (this.family === 1) return [0, 6000, 0];
    if (this.family === 4) return [0, 9700, 0];
    return [this.camera.panX, 2200 + this.camera.panY, 0];
  }
  cameraPosition() {
    const t = this.target(),
      c = this.camera;
    return [
      t[0] + Math.sin(c.yaw) * Math.cos(c.pitch) * c.distance,
      Math.max(2, t[1] + Math.sin(c.pitch) * c.distance),
      t[2] + Math.cos(c.yaw) * Math.cos(c.pitch) * c.distance,
    ];
  }
  write(buffer, snapshot, ui, stage = null, time = this.time) {
    const a = this.scratch,
      [lo, hi] = this.bounds();
    a.set([...this.cameraPosition(), ui.mirror ? 1 : 0], 0);
    a.set([...this.target(), ui.mode], 4);
    const elev = (ui.sun * Math.PI) / 180,
      az = -0.7;
    a.set(
      [
        Math.cos(elev) * Math.cos(az),
        Math.sin(elev),
        Math.cos(elev) * Math.sin(az),
        1.0,
      ],
      8,
    );
    a.set([...lo, 0.0045], 12);
    a.set([...hi, 0.0018], 16);
    a.set([time, 0.25, snapshot.wind, snapshot.windIntegral], 20);
    a.set([snapshot.moist, snapshot.dry, snapshot.lift, this.family], 24);
    a.set(
      [snapshot.frontX, snapshot.storm, snapshot.seed, ui.main ? 1 : 0],
      28,
    );
    a.set(
      [...this.viewSize, Math.tan((48 * Math.PI) / 360), ui.fog ? 1 : 0],
      32,
    );
    a.set(
      [
        stage === null ? (ui.alpha ?? 1) : this.dims[0],
        0.25,
        0,
        stage === null ? ui.provider : stage,
      ],
      36,
    );
    this.gpu.device.queue.writeBuffer(buffer, 0, a);
    return buffer;
  }
  cachedGroup(key, pipeline, entries) {
    if (!this.groups.has(key))
      this.groups.set(key, this.gpu.group(pipeline, entries()));
    return this.groups.get(key);
  }
  sourceGroup(buffer, old, forward, reverse, out, rest, emitters) {
    const key = [
      buffer.label,
      old.label,
      forward.label,
      reverse.label,
      out.label,
      rest.label,
      emitters.label,
    ].join("|");
    return this.cachedGroup(key, this.sourcePipeline, () => [
      [0, buffer],
      [1, old.createView()],
      [2, forward.createView()],
      [3, reverse.createView()],
      [4, out.createView()],
      [5, this.noise.createView()],
      [6, this.clamp],
      [7, rest.createView()],
      [8, emitters],
      [9, this.curl.createView()],
      [10, this.repeat],
    ]);
  }
  compute(e, pipeline, group, dims, timestamp) {
    const pass = e.beginComputePass(
      timestamp ? { timestampWrites: timestamp } : {},
    );
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, group);
    pass.dispatchWorkgroups(...dims.map((v) => Math.ceil(v / 4)));
    pass.end();
  }
  reset(weather, ui, clear = false, time = 0) {
    if (this.gpu.pending !== 0)
      throw Error("Source reset requires a drained explicit epoch boundary");
    this.time = time;
    this.steps = 0;
    this.epoch++;
    this.current = 0;
    this.currentRest = 0;
    this.family = Number(ui.family);
    const state = weather.sample(time);
    this.lastControl = state;
    const data = this.driver.reset(time, state, !clear);
    const emit = this.emitters[this.frame % 3][0];
    this.gpu.device.queue.writeBuffer(emit, 0, data);
    if (!this.sourcePipeline) return;
    const e = this.gpu.device.createCommandEncoder(),
      slot = this.uniforms[this.frame % 3];
    this.write(slot[0], state, ui, clear ? -1 : 0, time);
    this.compute(
      e,
      this.sourcePipeline,
      this.sourceGroup(
        slot[0],
        this.q[1],
        this.q[1],
        this.q[1],
        this.q[0],
        this.rest[1],
        emit,
      ),
      this.dims,
    );
    this.write(slot[1], state, ui, 5, time);
    this.compute(
      e,
      this.sourcePipeline,
      this.sourceGroup(
        slot[1],
        this.q[0],
        this.q[0],
        this.q[0],
        this.rest[0],
        this.rest[1],
        emit,
      ),
      this.restDims,
    );
    this.gpu.submit(e);
    this.frame++;
  }
  render(weather, ui, ticks = 0, worldTime = this.time) {
    const g = this.gpu;
    if (g.pending >= 3 || g.lost) return false;
    const e = g.device.createCommandEncoder(),
      frameSlot = this.frame % 3,
      slot = this.uniforms[frameSlot];
    const sample = this.frame % 12 === 0 && this.timings.begin();
    ticks = this.sourcePipeline ? ticks : 0;
    for (let n = 0; n < ticks; n++) {
      const nextTime = this.time + 0.25,
        snapshot = weather.sample(this.time);
      const emit = this.emitters[frameSlot][n];
      g.device.queue.writeBuffer(
        emit,
        0,
        this.driver.update(nextTime, 0.25, snapshot),
      );
      const q = this.q[this.current],
        next = this.q[1 - this.current],
        rest = this.rest[this.currentRest],
        off = n * 4;
      this.write(slot[off], snapshot, ui, 1, nextTime);
      this.compute(
        e,
        this.sourcePipeline,
        this.sourceGroup(slot[off], q, q, q, this.fwd, rest, emit),
        this.dims,
        sample && n === 0 ? this.timings.descriptor(0, null) : null,
      );
      this.write(slot[off + 1], snapshot, ui, 2, nextTime);
      this.compute(
        e,
        this.sourcePipeline,
        this.sourceGroup(slot[off + 1], q, this.fwd, q, this.rev, rest, emit),
        this.dims,
      );
      this.write(slot[off + 2], snapshot, ui, 3, nextTime);
      this.compute(
        e,
        this.sourcePipeline,
        this.sourceGroup(
          slot[off + 2],
          q,
          this.fwd,
          this.rev,
          next,
          rest,
          emit,
        ),
        this.dims,
      );
      this.write(slot[off + 3], snapshot, ui, 4, nextTime);
      this.compute(
        e,
        this.sourcePipeline,
        this.sourceGroup(
          slot[off + 3],
          q,
          q,
          q,
          this.rest[1 - this.currentRest],
          rest,
          emit,
        ),
        this.restDims,
        sample && n === ticks - 1 ? this.timings.descriptor(null, 1) : null,
      );
      this.current = 1 - this.current;
      this.currentRest = 1 - this.currentRest;
      this.time = nextTime;
      this.steps++;
    }
    const alpha = this.steps > 0 ? (ui.alpha ?? 1) : 1;
    ui = { ...ui, alpha };
    const visibleTime = this.sourcePipeline
      ? this.time - (this.steps > 0 ? (1 - alpha) * 0.25 : 0)
      : worldTime;
    this.lastControl = weather.sample(visibleTime);
    const v = slot[32];
    this.write(v, this.lastControl, ui, null, visibleTime);
    const inputs = () => [
      [0, v],
      [1, this.q[this.current].createView()],
      [2, this.noise.createView()],
      [3, this.clamp],
      [4, this.rest[this.currentRest].createView()],
      [10, this.q[1 - this.current].createView()],
      [11, this.rest[1 - this.currentRest].createView()],
      [14, this.repeat],
    ];
    const key = `${frameSlot}/${this.current}/${this.currentRest}`;
    let lightBuilt = false;
    if (this.lightPipeline) {
      const sb = this.cachedGroup("shadow/" + key, this.shadowPipeline, () => [
        ...inputs(),
        [12, this.receiver.createView()],
      ]);
      const sp = e.beginComputePass(
        sample ? { timestampWrites: this.timings.descriptor(2, null) } : {},
      );
      sp.setPipeline(this.shadowPipeline);
      sp.setBindGroup(0, sb);
      sp.dispatchWorkgroups(16, 16);
      sp.end();
      const bg = this.cachedGroup("light/" + key, this.lightPipeline, () => [
        ...inputs(),
        [6, this.light.createView()],
      ]);
      this.compute(
        e,
        this.lightPipeline,
        bg,
        this.lightDims,
        sample ? this.timings.descriptor(null, 3) : null,
      );
      lightBuilt = true;
    }
    const bg = this.cachedGroup("view/" + key, this.viewPipeline, () => [
      ...inputs(),
      [5, this.light.createView()],
      [7, this.air.createView()],
      [13, this.receiver.createView()],
    ]);
    let pass = e.beginRenderPass({
      colorAttachments: [
        {
          view: this.view.createView(),
          loadOp: "clear",
          storeOp: "store",
          clearValue: { r: 0, g: 0, b: 0, a: 1 },
        },
      ],
      ...(sample ? { timestampWrites: this.timings.descriptor(4, 5) } : {}),
    });
    pass.setPipeline(this.viewPipeline);
    pass.setBindGroup(0, bg);
    pass.draw(3);
    pass.end();
    const display = this.cachedGroup("display", this.displayPipeline, () => [
      [0, this.view.createView()],
      [1, this.clamp],
    ]);
    pass = e.beginRenderPass({
      colorAttachments: [
        {
          view: g.context.getCurrentTexture().createView(),
          loadOp: "clear",
          storeOp: "store",
          clearValue: { r: 0, g: 0, b: 0, a: 1 },
        },
      ],
      ...(sample ? { timestampWrites: this.timings.descriptor(6, 7) } : {}),
    });
    pass.setPipeline(this.displayPipeline);
    pass.setBindGroup(0, display);
    pass.draw(3);
    pass.end();
    if (sample) this.timings.encode(e);
    g.submit(e);
    this.lastSubmission = {
      frame: this.frame,
      time: visibleTime,
      committedSourceTime: this.time,
      sourceSteps: this.steps,
      worldTime,
      weatherInput: this.lastControl,
      epoch: this.epoch,
      family: this.family,
      controls: { ...ui },
      lightBuilt,
    };
    if (sample)
      this.timings.collect(this.frame, {
        sourceTicks: ticks,
        lightBuilt,
        time: visibleTime,
        epoch: this.epoch,
      });
    this.frame++;
    return true;
  }
  async probe() {
    if (this.provider !== 0)
      return {
        time: this.lastSubmission?.time,
        scope: "No volume source in selected provider",
        allocatedBytes: this.gpu.bytes,
        errors: this.gpu.errors,
      };
    const g = this.gpu;
    await g.device.queue.onSubmittedWorkDone();
    const code = `@group(0) @binding(0) var q:texture_3d<f32>; @group(0) @binding(1) var<storage,read_write> counts:array<atomic<u32>,16>; @compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){let n=textureDimensions(q);if(any(id>=n)){return;}let v=textureLoad(q,vec3i(id),0);let total=v.x+v.y;atomicAdd(&counts[0],1u);atomicAdd(&counts[1],u32(round(total*1000.)));atomicAdd(&counts[2],select(0u,1u,total>.015));atomicAdd(&counts[3],u32(round(v.x*1000.)));atomicAdd(&counts[4],u32(round(v.y*1000.)));atomicAdd(&counts[5],u32(round(v.w*100000.)));atomicAdd(&counts[6],select(0u,1u,total>.015&&id.y>n.y/2u));atomicAdd(&counts[7],select(0u,1u,total>.015&&(id.x<2u||id.y<2u||id.z<2u||id.x>n.x-3u||id.y>n.y-3u||id.z>n.z-3u)));atomicAdd(&counts[8],select(0u,1u,total>.015&&v.z<30.));atomicAdd(&counts[9],select(0u,1u,total>.015&&v.z>90.));atomicAdd(&counts[10],u32(round(v.z*total)));atomicMax(&counts[11],u32(round(total*100000.)));}`;
    this.probePipeline ??= await g.compute("finite source probe", code);
    this.probeCounts ??= g.buffer(
      "source probe counters",
      64,
      GPUBufferUsage.STORAGE |
        GPUBufferUsage.COPY_SRC |
        GPUBufferUsage.COPY_DST,
    );
    this.probeRead ??= g.buffer(
      "source probe readback",
      64,
      GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    );
    const e = g.device.createCommandEncoder();
    e.clearBuffer(this.probeCounts);
    const bg = g.group(this.probePipeline, [
      [0, this.q[this.current].createView()],
      [1, this.probeCounts],
    ]);
    this.compute(e, this.probePipeline, bg, this.dims);
    e.copyBufferToBuffer(this.probeCounts, 0, this.probeRead, 0, 64);
    g.submit(e);
    await this.probeRead.mapAsync(GPUMapMode.READ);
    const c = Array.from(
      new Uint32Array(this.probeRead.getMappedRange().slice(0)),
    );
    this.probeRead.unmap();
    return {
      time: this.time,
      steps: this.steps,
      epoch: this.epoch,
      weather: this.lastControl,
      dims: this.dims,
      voxels: c[0],
      opticalProxySum: c[1] / 1000,
      occupied: c[2],
      liquidSum: c[3] / 1000,
      iceSum: c[4] / 1000,
      lastStepSource: c[5] / 100000,
      upperOccupied: c[6],
      boundaryOccupied: c[7],
      youngOccupied: c[8],
      oldOccupied: c[9],
      ageProxySum: c[10],
      maxProxy: c[11] / 100000,
      emitterCount: this.driver.pulses.length,
      emitterSerial: this.driver.serial,
      gpu: this.timings.latest,
      allocatedBytes: g.bytes,
      errors: g.errors,
    };
  }
  async dispose() {
    await this.gpu.device.queue.onSubmittedWorkDone();
    this.groups.clear();
    this.gpu.context.unconfigure();
    this.gpu.device.destroy();
    this.gpu.retired = true;
  }
}
