const textureBytes = { rgba16float: 8, rgba8unorm: 4 };
export class Gpu {
  constructor(device, adapter, context, format) {
    this.device = device;
    this.adapter = adapter;
    this.context = context;
    this.format = format;
    this.allocations = [];
    this.errors = [];
    this.pending = 0;
    this.completed = 0;
    this.submitted = 0;
    this.epoch = 1;
    device.addEventListener("uncapturederror", (e) =>
      this.errors.push(String(e.error)),
    );
    device.lost.then((v) => {
      this.errors.push("Device lost: " + v.message);
      this.lost = true;
    });
  }
  static async open(canvas) {
    if (!navigator.gpu) throw Error("WebGPU is unavailable in this browser.");
    const adapter = await navigator.gpu.requestAdapter({
      powerPreference: "high-performance",
    });
    if (!adapter) throw Error("No WebGPU adapter");
    const features = adapter.features.has("timestamp-query")
      ? ["timestamp-query"]
      : [];
    const device = await adapter.requestDevice({ requiredFeatures: features });
    const context = canvas.getContext("webgpu");
    const format = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format, alphaMode: "opaque" });
    return new Gpu(device, adapter, context, format);
  }
  texture(label, size, format = "rgba16float", extra = 0) {
    const tex = this.device.createTexture({
      label,
      size,
      dimension: size.length === 3 ? "3d" : "2d",
      format,
      usage:
        GPUTextureUsage.TEXTURE_BINDING |
        GPUTextureUsage.STORAGE_BINDING |
        GPUTextureUsage.COPY_SRC |
        GPUTextureUsage.COPY_DST |
        extra,
    });
    this.allocations.push({
      label,
      bytes: size.reduce((a, b) => a * b, 1) * textureBytes[format],
      format,
      size,
    });
    return tex;
  }
  buffer(label, size, usage) {
    const b = this.device.createBuffer({ label, size, usage });
    this.allocations.push({ label, bytes: size });
    return b;
  }
  async module(label, text) {
    const module = this.device.createShaderModule({ label, code: text });
    const info = await module.getCompilationInfo();
    const errors = info.messages.filter((m) => m.type === "error");
    if (errors.length)
      throw Error(
        label +
          ": " +
          errors
            .map((m) => `${m.lineNum}:${m.linePos} ${m.message}`)
            .join("\n"),
      );
    return module;
  }
  async compute(label, code) {
    return this.device.createComputePipelineAsync({
      label,
      layout: "auto",
      compute: { module: await this.module(label, code), entryPoint: "main" },
    });
  }
  async render(label, code, format) {
    const module = await this.module(label, code);
    return this.device.createRenderPipelineAsync({
      label,
      layout: "auto",
      vertex: { module, entryPoint: "vs" },
      fragment: { module, entryPoint: "fs", targets: [{ format }] },
      primitive: { topology: "triangle-list" },
    });
  }
  group(pipeline, entries) {
    return this.device.createBindGroup({
      layout: pipeline.getBindGroupLayout(0),
      entries: entries.map(([binding, value]) => ({
        binding,
        resource: value instanceof GPUBuffer ? { buffer: value } : value,
      })),
    });
  }
  submit(encoder) {
    if (this.pending >= 3) throw Error("In-flight graph capacity exceeded");
    const submittedAt = performance.now();
    this.device.queue.submit([encoder.finish()]);
    this.pending++;
    this.submitted++;
    this.device.queue.onSubmittedWorkDone().then(
      () => {
        this.pending--;
        this.completed++;
        this.lastCompletionLatencyMs = performance.now() - submittedAt;
      },
      (e) => {
        this.pending--;
        this.errors.push(String(e));
      },
    );
  }
  get bytes() {
    return this.allocations.reduce((a, b) => a + b.bytes, 0);
  }
}
export class Timings {
  constructor(gpu) {
    this.gpu = gpu;
    this.supported = gpu.device.features.has("timestamp-query");
    this.latest = null;
    this.busy = false;
    if (this.supported) {
      this.query = gpu.device.createQuerySet({ type: "timestamp", count: 8 });
      this.resolve = gpu.buffer(
        "timestamp resolve",
        256,
        GPUBufferUsage.QUERY_RESOLVE | GPUBufferUsage.COPY_SRC,
      );
      this.read = gpu.buffer(
        "timestamp readback",
        256,
        GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
      );
    }
  }
  begin() {
    if (!this.supported || this.busy) return false;
    this.busy = true;
    return true;
  }
  descriptor(start, end) {
    return {
      querySet: this.query,
      ...(start === null ? {} : { beginningOfPassWriteIndex: start }),
      ...(end === null ? {} : { endOfPassWriteIndex: end }),
    };
  }
  encode(encoder) {
    encoder.resolveQuerySet(this.query, 0, 8, this.resolve, 0);
    encoder.copyBufferToBuffer(this.resolve, 0, this.read, 0, 64);
  }
  collect(frame, meta) {
    this.read.mapAsync(GPUMapMode.READ).then(
      () => {
        const q = new BigUint64Array(this.read.getMappedRange().slice(0, 64));
        this.read.unmap();
        const pairs = [
          ...(meta.sourceTicks ? [[0, 1]] : []),
          ...(meta.lightBuilt ? [[2, 3]] : []),
          [4, 5],
          [6, 7],
        ];
        const positive = pairs.every(([a, b]) => q[b] >= q[a]);
        const ordered =
          positive &&
          pairs.every((pair, i) => i === 0 || q[pair[0]] >= q[pairs[i - 1][1]]);
        const passMs = (a, b) =>
          q[b] >= q[a] ? Number(q[b] - q[a]) / 1e6 : null;
        this.latest = {
          frame,
          sourceMs: meta.sourceTicks ? passMs(0, 1) : 0,
          lightMs: meta.lightBuilt ? passMs(2, 3) : 0,
          viewMs: passMs(4, 5),
          displayMs: passMs(6, 7),
          wholeMs: ordered ? passMs(pairs[0][0], 7) : null,
          timingStatus: ordered
            ? "ordered query interval"
            : "cross-pass timestamps overlap/reorder; total GPU time unqualified",
          rawNanoseconds: Array.from(q, String),
          ...meta,
        };
        this.busy = false;
      },
      () => {
        this.busy = false;
      },
    );
  }
}
