import { sourceSections } from "./source-slices.js";
import { cirrusControls, CIRRUS_HEADS } from "./cirrus.js";
import { analyzeDensity } from "./morphology.js";
import { halfToFloat } from "./reference.js";
import { ThermalDriver, HEAD_STRIDE } from "./thermals.js";
import { Gpu, Timings } from "./gpu.js";
import { SourceDriver } from "./source.js";
const UNIFORM_BYTES = 160;
export class Scene {
  constructor(gpu, codes, provider = 0, family = 0) {
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
      dims:
        provider === 0
          ? family === 1
            ? [128, 80, 128]
            : [96, 80, 80]
          : [1, 1, 1],
      lightDims: provider === 0 ? [32, 24, 24] : [1, 1, 1],
      viewSize: [640, 360],
      family,
      lastControl: null,
      lastSubmission: null,
    });
    this.restDims = this.dims.map((v) => Math.max(1, v / 2));
    this.camera = { yaw: 0.3, pitch: -0.18, distance: 7500, panX: 0, panY: 0 };
    this.timings = new Timings(gpu);
    this.driver = new SourceDriver();
    this.thermalDriver = new ThermalDriver();
    this.groups = new Map();
    this.scratch = new Float32Array(40);
  }
  static async create(canvas, progress, provider = 0, family = 0) {
    const gpu = await Gpu.open(canvas);
    progress("Compiling the selected provider…");
    const names = [
      "source",
      "mix",
      "support",
      "dilate",
      "velocity",
      "noise",
      "mip",
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
    const s = new Scene(gpu, codes, provider, family);
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
      mipmapFilter: "linear",
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
      if (this.family === 1) this.mixPipeline = await g.compute("conservative optical mixing", this.codes.mix);
      this.velocityPipeline = await g.compute(
        "shared per-tick velocity",
        this.codes.source
          .replace("fn main(", "fn sourceStep(")
          .replace(
            "var<storage,read> velocityField",
            "var<storage,read_write> velocityField",
          ) + this.codes.velocity,
      );
      this.supportPipeline = await g.compute(
        "conservative source support",
        this.codes.support,
      );
      this.dilatePipeline = await g.compute(
        "source stencil guard",
        this.codes.dilate,
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
    const brickCount = this.dims.reduce(
      (count, n) => count * Math.ceil(n / 8),
      1,
    );
    this.support = g.buffer(
      "raw source support",
      brickCount * 4,
      GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
    );
    this.active = g.buffer(
      "guarded source support",
      brickCount * 4,
      GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
    );
    this.velocityField = g.buffer(
      "float32 shared velocity",
      16,
      GPUBufferUsage.STORAGE,
    );
    this.fwd = g.texture("advection forward", this.dims);
    this.rev = g.texture("advection reverse", this.dims);
    this.rest = [0, 1].map((i) =>
      g.texture("material coordinates " + i, this.restDims),
    );
    this.noise = g.texture(
      "native basis",
      this.provider === 0 ? [64, 64, 64] : [1, 1, 1],
      "rgba8unorm",
      0,
      this.provider === 0 ? 7 : 1,
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
      [1280, 720],
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
    if (this.sourcePipeline) {
      this.zeroHeads = g.buffer(
        "empty thermal heads",
        48 * HEAD_STRIDE * 4,
        GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      );
      this.zeroBands = g.buffer(
        "empty thermal bands",
        32 * 49 * 4,
        GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
      );
      this.thermalFrames = Array.from({ length: 3 }, (_, f) =>
        Array.from({ length: 8 }, (_, n) => ({
          heads: g.buffer(
            `heads ${f}/${n}`,
            48 * HEAD_STRIDE * 4,
            GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
          ),
          bands: g.buffer(
            `head bands ${f}/${n}`,
            32 * 49 * 4,
            GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
          ),
        })),
      );
      this.thermalBindings = new Map();
    }
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
    const mipPipeline = this.noisePipeline
      ? await g.compute("native material mips", this.codes.mip)
      : null;
    const e = d.createCommandEncoder();
    if (this.noisePipeline) {
      const bg = g.group(this.noisePipeline, [
        [0, this.noise.createView({ baseMipLevel: 0, mipLevelCount: 1 })],
      ]);
      const pass = e.beginComputePass();
      pass.setPipeline(this.noisePipeline);
      pass.setBindGroup(0, bg);
      pass.dispatchWorkgroups(16, 16, 16);
      pass.end();
      for (let level = 1; level < 7; level++) {
        const bg = g.group(mipPipeline, [
          [
            0,
            this.noise.createView({
              baseMipLevel: level - 1,
              mipLevelCount: 1,
            }),
          ],
          [1, this.noise.createView({ baseMipLevel: level, mipLevelCount: 1 })],
        ]);
        this.compute(e, mipPipeline, bg, [
          64 >> level,
          64 >> level,
          64 >> level,
        ]);
      }
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
        [-12000, 500, -12000],
        [12000, 15000, 12000],
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
    a.set(
      [
        ...this.cameraPosition(),
        stage === null
          ? ui.mirror
            ? 1
            : 0
          : ui.cachedVelocity !== false
            ? 1
            : 0,
      ],
      0,
    );
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
    a.set([...lo, 0.0045 * (ui.extinctionScale ?? 1)], 12);
    a.set([...hi, 0.0018 * (ui.extinctionScale ?? 1)], 16);
    a.set(
      [
        time,
        stage === null
          ? (ui.materialSpace === "world" ? 0 : 1) + (ui.materialFilter ? 2 : 0)
          : 0.25,
        snapshot.wind,
        snapshot.windIntegral,
      ],
      20,
    );
    // Optical material profile flag is private; source-stage family remains unflagged.
    a.set([snapshot.moist, snapshot.dry, snapshot.lift, this.family | (stage === null && ui.materialStructure && this.family === 1 ? 16 : 0)], 24);
    a.set(
      [
        snapshot.frontX,
        snapshot.storm,
        stage === null ? (ui.viewSteps ?? 96) : ui.thermals ? (ui.materialCore ? 2 : 1) : 0,
        stage === null ? (ui.main ? 1 : 0) : (ui.neckMix ? 1 : 0),
      ],
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
        stage === null ? 0 : ui.emptyWork !== false ? 1 : 0,
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
  sourceGroup(
    buffer,
    old,
    forward,
    reverse,
    out,
    rest,
    emitters,
    active = this.active,
  ) {
    const thermal = this.thermalBindings.get(buffer) ?? {
      heads: this.zeroHeads,
      bands: this.zeroBands,
    };
    const key = [
      thermal.heads.label,
      thermal.bands.label,
      buffer.label,
      old.label,
      forward.label,
      reverse.label,
      out.label,
      rest.label,
      emitters.label,
      active.label,
      this.velocityField.label,
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
      [11, active],
      [12, this.velocityField],
      [13, thermal.heads],
      [14, thermal.bands],
    ]);
  }
  ensureVelocityField() {
    if (!this.sourcePipeline || this.velocityField.size > 16) return;
    this.velocityField = this.gpu.buffer(
      "float32 shared velocity/full",
      this.dims.reduce((a, b) => a * b, 1) * 16,
      GPUBufferUsage.STORAGE,
    );
  }
  velocityGroup(uniform, q) {
    const thermal = this.thermalBindings.get(uniform) ?? {
      heads: this.zeroHeads,
      bands: this.zeroBands,
    };
    return this.cachedGroup(
      "velocity/" +
        uniform.label +
        "/" +
        q.label +
        "/" +
        this.velocityField.label +
        "/" +
        thermal.heads.label +
        "/" +
        thermal.bands.label,
      this.velocityPipeline,
      () => [
        [0, uniform],
        [1, q.createView()],
        [9, this.curl.createView()],
        [10, this.repeat],
        [11, this.active],
        [12, this.velocityField],
        [13, thermal.heads],
        [14, thermal.bands],
      ],
    );
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
    this.lastPresentation = null;
    this.lastSubmission = null;
    this.epoch++;
    this.current = 0;
    this.currentRest = 0;
    this.family = Number(ui.family);
    const state = weather.sample(time);
    this.lastControl = state;
    this.lastSourceInput = state;
    const ordinaryData = this.driver.reset(time, state, !clear);
    this.cirrusData = this.family === 4 ? cirrusControls(71, state.wind) : null;
    const data = this.cirrusData ?? ordinaryData;
    this.thermalDriver.reset(time, state, this.driver.pulses, Boolean(ui.materialCore));
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
    this.viewSize = ui.resolved ? [1280, 720] : [640, 360];
    if (this.sourcePipeline && ui.cachedVelocity) this.ensureVelocityField();
    // Reject stale forcing before mutating the driver or staging any source update.
    for (let n = 0; n < ticks; n++) weather.sample(this.time + n * 0.25);
    const sourceInputRange = [];
    const e = g.device.createCommandEncoder(),
      frameSlot = this.frame % 3,
      slot = this.uniforms[frameSlot];
    const sample =
      !ui.sourceOnly && this.frame % 12 === 0 && this.timings.begin();
    ticks = this.sourcePipeline ? ticks : 0;
    for (let n = 0; n < ticks; n++) {
      const nextTime = this.time + 0.25,
        snapshot = weather.sample(this.time);
      this.lastSourceInput = snapshot;
      sourceInputRange.push({
        time: snapshot.time,
        tick: snapshot.weatherTick,
        revision: snapshot.revision,
      });
      const emit = this.emitters[frameSlot][n];
      g.device.queue.writeBuffer(
        emit,
        0,
        this.cirrusData ?? this.driver.update(nextTime, 0.25, snapshot),
      );
      const q = this.q[this.current],
        next = this.q[1 - this.current],
        rest = this.rest[this.currentRest],
        off = n * 4;
      if (ui.thermals && this.family === 1) {
        const table = this.thermalDriver.update(
          nextTime,
          0.25,
          snapshot,
          this.driver.pulses,
        );
        const buffers = this.thermalFrames[frameSlot][n];
        g.device.queue.writeBuffer(buffers.heads, 0, table.heads);
        g.device.queue.writeBuffer(buffers.bands, 0, table.bands);
        for (let j = 0; j < 4; j++)
          this.thermalBindings.set(slot[off + j], buffers);
      } else {
        for (let j = 0; j < 4; j++) this.thermalBindings.delete(slot[off + j]);
      }
      this.write(slot[off], snapshot, ui, 1, nextTime);
      if (ui.emptyWork !== false) {
        const supportGroup = this.cachedGroup(
          "support/" + frameSlot + "/" + n + "/" + this.current,
          this.supportPipeline,
          () => [
            [0, slot[off]],
            [1, q.createView()],
            [2, emit],
            [3, this.support],
          ],
        );
        const pass = e.beginComputePass(
          sample && n === 0
            ? { timestampWrites: this.timings.descriptor(0, null) }
            : {},
        );
        pass.setPipeline(this.supportPipeline);
        pass.setBindGroup(0, supportGroup);
        pass.dispatchWorkgroups(...this.dims.map((v) => Math.ceil(v / 8)));
        pass.end();
        const guardGroup = this.cachedGroup(
          "guard/" + this.current,
          this.dilatePipeline,
          () => [
            [0, q.createView()],
            [1, this.support],
            [2, this.active],
          ],
        );
        this.compute(
          e,
          this.dilatePipeline,
          guardGroup,
          this.dims.map((v) => Math.ceil(v / 8)),
        );
      }

      if (ui.cachedVelocity !== false) {
        const vg = this.velocityGroup(slot[off], q);
        this.compute(
          e,
          this.velocityPipeline,
          vg,
          this.dims,
          sample && n === 0 && ui.emptyWork === false
            ? this.timings.descriptor(0, null)
            : null,
        );
      }
      this.compute(
        e,
        this.sourcePipeline,
        this.sourceGroup(slot[off], q, q, q, this.fwd, rest, emit),
        this.dims,
        sample &&
          n === 0 &&
          ui.emptyWork === false &&
          ui.cachedVelocity === false
          ? this.timings.descriptor(0, null)
          : null,
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
      if (this.family === 1 && ui.thermals && ui.materialCore && ui.neckMix) {
        const bg=this.cachedGroup("mix/"+slot[off+2].label+"/"+next.label, this.mixPipeline,()=>[
          [0,slot[off+2]],[1,next.createView()],[2,this.rev.createView()],
        ]);
        this.compute(e,this.mixPipeline,bg,this.dims);
        e.copyTextureToTexture({texture:this.rev},{texture:next},this.dims);
      }
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
    if (ui.sourceOnly) {
      g.submit(e);
      this.lastSubmission = {
        frame: this.frame++,
        time: this.time,
        committedSourceTime: this.time,
        sourceSteps: this.steps,
        sourceInputRange,
        worldTime,
        epoch: this.epoch,
        presented: false,
        weatherInput: this.lastSourceInput,
      };
      return true;
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
    pass.setViewport(0, 0, ...this.viewSize, 0, 1);
    pass.setPipeline(this.viewPipeline);
    pass.setBindGroup(0, bg);
    pass.draw(3);
    pass.end();
    const display = this.cachedGroup(
      "display/" + frameSlot,
      this.displayPipeline,
      () => [
        [0, this.view.createView()],
        [1, this.clamp],
        [2, v],
      ],
    );
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
      presented: true,
      time: visibleTime,
      committedSourceTime: this.time,
      sourceSteps: this.steps,
      worldTime,
      weatherInput: this.lastControl,
      sourceInputRange,
      camera: {
        ...this.camera,
        position: this.cameraPosition(),
        target: this.target(),
      },
      viewSize: this.viewSize,
      domain: this.bounds(),
      look: {
        kappa: [0.0045, 0.0018].map((k) => k * (ui.extinctionScale ?? 1)),
        MS: [1, 0.25, 0],
        exposure: 0.5,
      },
      epoch: this.epoch,
      family: this.family,
      controls: { ...ui },
      lightBuilt,
    };
    this.lastPresentation = this.lastSubmission;
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
  async probe(ui, queryWeather) {
    if (this.provider !== 0)
      return {
        time: this.lastSubmission?.time,
        scope: "No volume source in selected provider",
        allocatedBytes: this.gpu.bytes,
        errors: this.gpu.errors,
      };
    const g = this.gpu;
    await g.device.queue.onSubmittedWorkDone();
    const code = `@group(0) @binding(0) var q:texture_3d<f32>; @group(0) @binding(1) var<storage,read_write> counts:array<atomic<u32>,32>; @compute @workgroup_size(4,4,4) fn main(@builtin(global_invocation_id) id:vec3u){let n=textureDimensions(q);if(any(id>=n)){return;}let v=textureLoad(q,vec3i(id),0);let total=v.x+v.y;atomicAdd(&counts[0],1u);atomicAdd(&counts[1],u32(round(total*1000.)));atomicAdd(&counts[2],select(0u,1u,total>.015));atomicAdd(&counts[3],u32(round(v.x*1000.)));atomicAdd(&counts[4],u32(round(v.y*1000.)));atomicAdd(&counts[5],u32(round(v.w*100000.)));atomicAdd(&counts[6],select(0u,1u,total>.015&&id.y>n.y/2u));atomicAdd(&counts[7],select(0u,1u,total>.015&&(id.x<2u||id.y<2u||id.z<2u||id.x>n.x-3u||id.y>n.y-3u||id.z>n.z-3u)));atomicAdd(&counts[8],select(0u,1u,total>.015&&v.z<30.));atomicAdd(&counts[9],select(0u,1u,total>.015&&v.z>90.));atomicAdd(&counts[10],u32(round(v.z*total)));atomicMax(&counts[11],u32(round(total*100000.)));if(id.y>=n.y*3u/5u){atomicAdd(&counts[12],u32(round(v.x*1000.)));atomicAdd(&counts[13],u32(round(v.y*1000.)));}if(total>.015){atomicMax(&counts[14],id.y+1u);if(id.x<2u){atomicAdd(&counts[16],1u);}if(id.x>n.x-3u){atomicAdd(&counts[17],1u);}if(id.y<2u){atomicAdd(&counts[18],1u);}if(id.y>n.y-3u){atomicAdd(&counts[19],1u);}if(id.z<2u){atomicAdd(&counts[20],1u);}if(id.z>n.z-3u){atomicAdd(&counts[21],1u);}if(v.y>v.x){atomicAdd(&counts[15],1u);}}}`;
    this.probePipeline ??= await g.compute("finite source probe", code);
    this.probeCounts ??= g.buffer(
      "source probe counters",
      128,
      GPUBufferUsage.STORAGE |
        GPUBufferUsage.COPY_SRC |
        GPUBufferUsage.COPY_DST,
    );
    this.probeRead ??= g.buffer(
      "source probe readback",
      128,
      GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    );
    const e = g.device.createCommandEncoder();
    e.clearBuffer(this.probeCounts);
    const bg = g.group(this.probePipeline, [
      [0, this.q[this.current].createView()],
      [1, this.probeCounts],
    ]);
    this.compute(e, this.probePipeline, bg, this.dims);
    e.copyBufferToBuffer(this.probeCounts, 0, this.probeRead, 0, 128);
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
      weather: this.lastSourceInput,
      dims: this.dims,
      voxels: c[0],
      opticalProxySum: c[1] / 1000,
      occupied: c[2],
      liquidSum: c[3] / 1000,
      iceSum: c[4] / 1000,
      lastStepSource: c[5] / 100000,
      upperOccupied: c[6],
      boundaryOccupied: c[7],
      boundaryFacesXYZ: c.slice(16, 22),
      youngOccupied: c[8],
      oldOccupied: c[9],
      ageProxySum: c[10],
      maxProxy: c[11] / 100000,
      upperFortyPercentLiquidSum: c[12] / 1000,
      upperFortyPercentIceSum: c[13] / 1000,
      occupiedTopHeight:
        this.bounds()[0][1] +
        (c[14] / this.dims[1]) * (this.bounds()[1][1] - this.bounds()[0][1]),
      iceDominantOccupied: c[15],
      opticalColumns:
        this.family === 1 ? await this.probeColumns(ui, queryWeather) : null,
      morphology: this.family === 1 ? await this.probeMorphology(ui) : null,
      thermalHeads: this.thermalDriver.heads.length,
      thermalForceHeads: this.thermalDriver.forceHeads,
      emitterCount: this.family === 4 ? (queryWeather.moist > .001 && queryWeather.lift > .001 ? CIRRUS_HEADS : 0) : this.driver.pulses.length,
      cirrusHeads: this.family === 4 ? CIRRUS_HEADS : 0,
      emitterSerial: this.driver.serial,
      gpu: this.timings.latest,
      allocatedBytes: g.bytes,
      errors: g.errors,
    };
  }
  async probeMorphology(ui) {
    const g = this.gpu,
      [nx, ny, nz] = this.dims,
      bytes = nx * ny * nz * 8;
    this.morphologyRead ??= g.buffer(
      "morphology observer readback",
      bytes,
      GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    );
    const e = g.device.createCommandEncoder();
    e.copyTextureToBuffer(
      { texture: this.q[this.current] },
      { buffer: this.morphologyRead, bytesPerRow: nx * 8, rowsPerImage: ny },
      this.dims,
    );
    g.submit(e);
    await this.morphologyRead.mapAsync(GPUMapMode.READ);
    const raw = new Uint16Array(this.morphologyRead.getMappedRange());
    this.morphologyValues ??= new Float32Array(nx * ny * nz);
    for (let i = 0; i < this.morphologyValues.length; i++)
      this.morphologyValues[i] =
        halfToFloat(raw[i * 4]) + halfToFloat(raw[i * 4 + 1]);
    this.morphologyRead.unmap();
    this.morphologyScratch ??= {};
    const started = performance.now();
    const result = analyzeDensity(
      this.morphologyValues,
      this.dims,
      this.bounds(),
      ui.thermals ? this.thermalDriver.data : [],
      this.morphologyScratch,
    );
    return {
      ...result,
      sections: sourceSections(this.morphologyValues,this.dims,this.bounds(),this.thermalDriver.data),
      analysisCpuMs: performance.now() - started,
      observerReadbackBytes: bytes,
    };
  }
  async probeColumns(ui, queryWeather) {
    // Observer-only five vertical columns: total-q support versus actual shaped extinction.
    const g = this.gpu,
      count = 400,
      bytes = count * 16;
    this.columnPipeline ??= await g.compute(
      "storm optical columns",
      this.codes.optics +
        `@group(0) @binding(15) var<storage,read_write> result:array<vec4f>; @compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3u){let i=id.x;if(i>=400u){return;}let column=i/80u;let y=mix(p.bmin.y,p.bmax.y,(f32(i%80u)+.5)/80.);let x=vec3f(p.weather.w+(f32(column)-2.)*1000.,y,p.weather.x*1.1);let q=rawQ(x).xy;let c=coefficients(x,1.);result[i]=vec4f(q,c.x+c.y,0.);}`,
    );
    this.columnUniform ??= g.buffer(
      "column uniform",
      160,
      GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    );
    this.columnResult ??= g.buffer(
      "column result",
      bytes,
      GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
    );
    this.columnRead ??= g.buffer(
      "column read",
      bytes,
      GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    );
    this.write(
      this.columnUniform,
      queryWeather,
      { ...ui, alpha: 1 },
      null,
      this.time,
    );
    const q = this.q[this.current],
      rest = this.rest[this.currentRest];
    const bg = g.group(this.columnPipeline, [
      [0, this.columnUniform],
      [1, q.createView()],
      [2, this.noise.createView()],
      [3, this.clamp],
      [4, rest.createView()],
      [10, q.createView()],
      [11, rest.createView()],
      [14, this.repeat],
      [15, this.columnResult],
    ]);
    const e = g.device.createCommandEncoder();
    const pass = e.beginComputePass();
    pass.setPipeline(this.columnPipeline);
    pass.setBindGroup(0, bg);
    pass.dispatchWorkgroups(Math.ceil(count / 64));
    pass.end();
    e.copyBufferToBuffer(this.columnResult, 0, this.columnRead, 0, bytes);
    g.submit(e);
    await this.columnRead.mapAsync(GPUMapMode.READ);
    const data = Array.from(
      new Float32Array(this.columnRead.getMappedRange().slice(0)),
    );
    this.columnRead.unmap();
    return {
      queryWeather,
      controls: { ...ui },
      scope:
        "Five vertical columns at shared base-wind center and x offsets -2000,-1000,0,1000,2000m; RG=(qLiquid,qIce), B=shaped sigma/m;80 height midpoints",
      bounds: this.bounds(),
      data,
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
