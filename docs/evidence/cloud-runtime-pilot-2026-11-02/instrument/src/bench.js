import { Scene } from "./scene.js";
import { runGpuReferenceChecks } from "./gpu-reference.js";
import { Weather, WorldClock, validateProvider } from "./weather.js";
const $ = (id) => document.getElementById(id),
  status = $("status");
let scene = null,
  weather = new Weather(),
  clock = new WorldClock(),
  playing = true,
  busy = false,
  fault = null,
  identity,
  last = performance.now(),
  lastHud = 0,
  frames = 0,
  skips = 0,
  advanceTarget = null,
  benchmark = null,
  configurationVersion = 0,
  opticalEpoch = 0,
  configurationPending = false;
const cpuSamples = [],
  gpuSamples = [];
let uiDirty = true,
  lastRenderWorldTime = -1,
  configurationRunning = false,
  lastGpuFrame = -1,
  previousHudTime = 0,
  previousHudWall = performance.now();
function options() {
  return {
    family: Number($("family").value),
    sun: Number($("sun").value),
    mode: Number($("mode").value),
    provider: Number($("provider").value),
    fog: $("fog").checked,
    mirror: $("mirror").checked,
    main: $("main").checked,
    alpha: 1,
  };
}
function inputs() {
  return {
    moist: Number($("moist").value),
    dry: Number($("dry").value),
    lift: Number($("lift").value),
    wind: Number($("wind").value),
  };
}
function info(s) {
  status.textContent = s;
}
function fail(e) {
  fault = e;
  info("Optics unavailable — " + e.message);
  console.error(e);
  $("receipt").textContent = e.stack || String(e);
}
function quantile(v, p) {
  if (!v.length) return null;
  const a = [...v].sort((a, b) => a - b);
  return a[Math.min(a.length - 1, Math.floor(a.length * p))];
}
function adapterInfo() {
  const a = scene?.gpu.adapter.info;
  return a
    ? {
        vendor: a.vendor,
        architecture: a.architecture,
        device: a.device,
        description: a.description,
      }
    : null;
}
async function receipt(name, data) {
  const record = {
    name,
    build: identity?.build,
    identity,
    host: {
      adapter: adapterInfo(),
      screen: [screen.width, screen.height],
      devicePixelRatio,
    },
    world: {
      time: clock.time,
      epoch: clock.epoch,
      weather: weather.snapshot(clock.time),
    },
    ...data,
  };
  $("receipt").textContent = JSON.stringify(record, null, 2);
  const response = await fetch("/receipt", {
    method: "POST",
    body: JSON.stringify(record),
  });
  if (!response.ok) throw Error("Receipt save failed: " + response.status);
  return record;
}
async function configure() {
  configurationVersion++;
  if (configurationRunning) return;
  configurationPending = true;
  if (busy) return;
  configurationPending = false;
  configurationRunning = true;
  busy = true;
  let applied = 0;
  try {
    while (applied !== configurationVersion) {
      const version = configurationVersion;
      fault = null;
      const old = scene;
      scene = null;
      if (old) await old.dispose();
      if (version !== configurationVersion) continue;
      if (!$("optical").checked) {
        $("view").style.visibility = "hidden";
        info("Optical resources retired · CPU weather running");
        applied = version;
        continue;
      }
      const ui = options();
      validateProvider(
        ["full", "shadow", "clear"][ui.provider],
        ["volume", "shadow", "clear"][ui.provider],
      );
      const next = await Scene.create($("view"), info, ui.provider);
      if (version !== configurationVersion) {
        await next.dispose();
        continue;
      }
      scene = next;
      $("view").style.visibility = "visible";
      scene.family = ui.family;
      scene.camera.distance =
        ui.family === 1 ? 25000 : ui.family === 4 ? 16000 : 7500;
      scene.camera.pitch = ui.family === 4 ? -0.28 : -0.18;
      scene.reset(weather, ui, false, clock.time);
      scene.epoch = ++opticalEpoch;
      await scene.gpu.device.queue.onSubmittedWorkDone();
      previousHudTime = scene.time;
      previousHudWall = performance.now();
      uiDirty = true;
      info(
        "Selected " +
          ["live 3D", "analytic shadow-only", "clear-air"][ui.provider] +
          " provider",
      );
      applied = version;
    }
  } catch (e) {
    fail(e);
  } finally {
    configurationRunning = false;
    busy = false;
    if (configurationPending) queueMicrotask(configure);
    last = performance.now();
  }
}

async function reset(clear = false, newWorld = true) {
  if (busy || !scene) return;
  busy = true;
  fault = null;
  try {
    await scene.gpu.device.queue.onSubmittedWorkDone();
    if (newWorld) {
      weather = new Weather(71, {
        ...inputs(),
        storm: options().family === 1 ? 1 : 0,
      });
      clock.reset();
      advanceTarget = null;
    }
    scene.family = options().family;
    scene.camera.distance =
      scene.family === 1 ? 25000 : scene.family === 4 ? 16000 : 7500;
    scene.camera.pitch = scene.family === 4 ? -0.28 : -0.18;
    scene.reset(weather, options(), clear, clock.time);
    scene.epoch = ++opticalEpoch;
    await scene.gpu.device.queue.onSubmittedWorkDone();
    previousHudTime = scene.time;
    previousHudWall = performance.now();
    uiDirty = true;
    info(
      newWorld
        ? "New world epoch · " +
            (clear ? "empty source" : "seeded initial state")
        : "New optical epoch · weather preserved",
    );
  } catch (e) {
    fail(e);
  } finally {
    busy = false;
    if (configurationPending) queueMicrotask(configure);
    last = performance.now();
  }
}
for (const id of ["moist", "dry", "lift", "wind"])
  $(id).oninput = () => {
    try {
      weather.command(inputs());
      info("Weather command queued for next world commit");
    } catch (e) {
      fail(e);
    }
  };
$("play").onclick = () => {
  playing = !playing;
  $("play").textContent = playing ? "Pause" : "Play";
};
$("clear").onclick = () => reset(true);
$("reset").onclick = () => reset(false);
$("recover").onclick = () => reset(false, false);
$("family").onchange = () => reset(false);
$("front").onclick = () => {
  weather.sendFront();
  info("Dry front queued · world owns its advance");
};
$("storm").onclick = () => {
  weather.command({ storm: 1, lift: 1.8, moist: 0.95, dry: 0.02 });
  $("lift").value = "1.8";
  $("moist").value = ".95";
  $("dry").value = ".02";
  info(
    "Runtime storm forcing queued · use storm family for its tall-domain proof",
  );
};
$("provider").onchange = configure;
$("optical").onchange = configure;
$("advance").onclick = () => {
  if (busy || !scene) return;
  advanceTarget =
    scene.time +
    Math.max(
      0.25,
      Math.round(Number($("advance-seconds").value) / 0.25) * 0.25,
    );
  playing = false;
  $("play").textContent = "Play";
  info("Deterministic replay advance queued");
};
$("probe").onclick = async () => {
  if (busy || !scene || fault) return;
  busy = true;
  try {
    await receipt("source-" + Math.round(scene.time * 100), {
      probe: await scene.probe(),
      submittedFrame: scene.lastSubmission,
    });
    info("Source observation saved · separate from quiet timing");
  } catch (e) {
    fail(e);
  } finally {
    busy = false;
    if (configurationPending) queueMicrotask(configure);
    last = performance.now();
  }
};
$("capture").onclick = async () => {
  if (busy || !scene || fault) return;
  busy = true;
  try {
    await scene.gpu.device.queue.onSubmittedWorkDone();
    const ui = { ...options(), alpha: 1 };
    if (!scene.render(weather, ui, 0, clock.time))
      throw Error("Capture graph was not submitted");
    const submittedFrame = structuredClone(scene.lastSubmission);
    await scene.gpu.device.queue.onSubmittedWorkDone();
    const blob = await new Promise((r) => $("view").toBlob(r));
    if (!blob) throw Error("No canvas image");
    const name =
      "cloud-" +
      scene.family +
      "-" +
      Math.round(submittedFrame.time * 100) +
      "-e" +
      scene.epoch;
    const saved = await fetch("/capture", {
      method: "POST",
      headers: { "X-Capture-Name": name },
      body: blob,
    });
    if (!saved.ok) throw Error("Image save failed: " + saved.status);
    await receipt(name, { submittedFrame, capture: name + ".png" });
    info("Image saved · " + name);
  } catch (e) {
    fail(e);
  } finally {
    busy = false;
    if (configurationPending) queueMicrotask(configure);
    last = performance.now();
  }
};
$("bench").onclick = () => {
  if (busy || !scene || fault) return;
  benchmark = {
    started: performance.now(),
    frames: 0,
    steps: scene.steps,
    time: scene.time,
    skips,
    settings: options(),
    rate: Number($("rate").value),
  };
  cpuSamples.length = 0;
  gpuSamples.length = 0;
  info("Quiet 10s measurement · source/capture probes excluded");
};
$("tests").onclick = async () => {
  if (busy || !scene || fault) return;
  busy = true;
  try {
    const result = await runGpuReferenceChecks(scene);
    await receipt("gpu-reference-checks", result);
    info(
      result.pass
        ? "Independent finite GPU checks passed"
        : "Finite GPU check failed · see receipt",
    );
  } catch (e) {
    fail(e);
  } finally {
    busy = false;
    if (configurationPending) queueMicrotask(configure);
    last = performance.now();
    uiDirty = true;
  }
};

document.addEventListener("input", () => {
  uiDirty = true;
});
document.addEventListener("change", () => {
  uiDirty = true;
});
const canvas = $("view");
let drag = null;
canvas.onpointerdown = (e) => {
  drag = { x: e.clientX, y: e.clientY, pan: e.shiftKey || e.button === 1 };
  canvas.setPointerCapture(e.pointerId);
};
canvas.onpointerup = () => (drag = null);
canvas.onpointermove = (e) => {
  if (!drag || !scene) return;
  uiDirty = true;
  const dx = e.clientX - drag.x,
    dy = e.clientY - drag.y;
  if (drag.pan) {
    scene.camera.panX -= dx * scene.camera.distance * 0.001;
    scene.camera.panY += dy * scene.camera.distance * 0.001;
  } else {
    scene.camera.yaw -= dx * 0.004;
    scene.camera.pitch = Math.max(
      -0.7,
      Math.min(0.8, scene.camera.pitch + dy * 0.004),
    );
  }
  drag.x = e.clientX;
  drag.y = e.clientY;
};
canvas.addEventListener(
  "wheel",
  (e) => {
    e.preventDefault();
    if (!scene) return;
    uiDirty = true;
    const dx = Math.max(-120, Math.min(120, e.deltaX)),
      dy = Math.max(-120, Math.min(120, e.deltaY));
    if (e.ctrlKey || e.metaKey)
      scene.camera.distance = Math.max(
        1800,
        Math.min(40000, scene.camera.distance * Math.exp(dy * 0.006)),
      );
    else if (e.shiftKey) {
      scene.camera.panX -= dx * scene.camera.distance * 0.0005;
      scene.camera.panY += dy * scene.camera.distance * 0.0005;
    } else {
      scene.camera.yaw -= dx * 0.003;
      scene.camera.pitch = Math.max(
        -0.7,
        Math.min(0.8, scene.camera.pitch + dy * 0.003),
      );
    }
  },
  { passive: false },
);
canvas.oncontextmenu = (e) => e.preventDefault();
function finishBenchmark(now) {
  const b = benchmark;
  benchmark = null;
  const g = gpuSamples;
  receipt("measurement-" + Math.round(scene.time), {
    scope: "Available-host standalone pilot; not M2/full-sky admission",
    elapsedMs: now - b.started,
    frames: b.frames,
    callbackFps: b.frames / ((now - b.started) / 1000),
    sourceSeconds: scene.time - b.time,
    sourceSteps: scene.steps - b.steps,
    skips: skips - b.skips,
    controls: b.settings,
    rateRequested: b.rate,
    cpu: {
      p50: quantile(cpuSamples, 0.5),
      p95: quantile(cpuSamples, 0.95),
      p99: quantile(cpuSamples, 0.99),
    },
    gpu: {
      samples: g.length,
      orderedTotalSamples: g.filter((v) => v.wholeMs !== null).length,
      rejectedTotalSamples: g.filter((v) => v.wholeMs === null).length,
      wholeP50: quantile(g.map((v) => v.wholeMs).filter(Number.isFinite), 0.5),
      wholeP95: quantile(g.map((v) => v.wholeMs).filter(Number.isFinite), 0.95),
      sourceP95: quantile(
        g.map((v) => v.sourceMs),
        0.95,
      ),
      lightP95: quantile(
        g.map((v) => v.lightMs),
        0.95,
      ),
      viewP95: quantile(
        g.map((v) => v.viewMs),
        0.95,
      ),
    },
    logicalCreatedBytes: scene.gpu.bytes,
    allocations: scene.gpu.allocations,
    errors: scene.gpu.errors,
  }).then(() => info("Measurement saved"));
}
function frame(now) {
  requestAnimationFrame(frame);
  const elapsed = Math.max(0, (now - last) / 1000);
  last = now;
  const cpuStart = performance.now();
  let target = null;
  if (advanceTarget !== null)
    target = Math.min(advanceTarget, (scene?.time ?? clock.time) + 8);
  clock.step(
    elapsed,
    Number($("rate").value),
    playing && advanceTarget === null,
    weather,
    target,
  );
  if (
    !busy &&
    scene &&
    !fault &&
    !scene.gpu.lost &&
    (uiDirty ||
      benchmark ||
      clock.time !== lastRenderWorldTime ||
      (scene.provider === 0 && clock.time - scene.time >= 0.25))
  ) {
    const gap = clock.time - scene.time;
    const ticks =
      scene.provider === 0
        ? Math.max(0, Math.min(8, Math.floor((gap + 1e-7) / 0.25)))
        : 0;
    const residual = Math.max(0, gap - ticks * 0.25);
    const ui = {
      ...options(),
      alpha: playing ? Math.min(1, residual / 0.25) : 1,
    };
    try {
      if (scene.render(weather, ui, ticks, clock.time)) {
        frames++;
        uiDirty = false;
        lastRenderWorldTime = clock.time;
        const stamp = scene.timings.latest;
        if (stamp && stamp.frame !== lastGpuFrame) {
          lastGpuFrame = stamp.frame;
          if (benchmark) gpuSamples.push(stamp);
        }
        if (benchmark) {
          benchmark.frames++;
          if (now - benchmark.started >= 10000) finishBenchmark(now);
        }
      } else skips++;
    } catch (e) {
      fail(e);
    }
  }
  if (advanceTarget !== null && scene && scene.time >= advanceTarget - 1e-6) {
    advanceTarget = null;
    info("Deterministic advance complete");
  }
  if (benchmark) cpuSamples.push(performance.now() - cpuStart);
  if (now - lastHud > 500) {
    const wall = (now - previousHudWall) / 1000,
      sourceTime = scene?.time ?? previousHudTime,
      delivered = (sourceTime - previousHudTime) / Math.max(wall, 0.001);
    previousHudWall = now;
    previousHudTime = sourceTime;
    lastHud = now;
    const g = scene?.timings.latest;
    $("stats").textContent =
      `World ${clock.time.toFixed(2)}s · tick ${weather.tick} · epoch ${clock.epoch}\nSource ${scene?.time.toFixed(2) ?? "retired"}s · ${scene?.steps ?? 0} steps\nRequested ${playing ? $("rate").value : 0}x · source delivered ${delivered.toFixed(1)}x\nOptical epoch ${scene?.epoch ?? "none"} · age ${scene ? Math.max(0, clock.time - scene.time).toFixed(2) : "—"}s\nFrames ${frames} · queue ${scene?.gpu.pending ?? 0}/3 · skipped ${skips}\nGPU ${g ? (g.wholeMs === null ? "cross-pass timing unqualified" : g.wholeMs.toFixed(2) + "ms / ordered query interval") : "unavailable"}\n${g ? "source " + (g.sourceMs ?? 0).toFixed(2) + " · light " + (g.lightMs ?? 0).toFixed(2) + " · view " + (g.viewMs ?? 0).toFixed(2) : ""}\nLogical live textures/buffers ${scene ? (scene.gpu.bytes / 1048576).toFixed(1) : "0"}MiB\n${scene?.provider === 0 ? scene.dims.join("×") + " dense source reference" : "Volume source retired"}\nWeather rain ${weather.state.storm * 0.002} kg/m²/s · causes ${weather.events.length}\nBuild ${identity?.build ?? "loading"}\n${fault ? "UNAVAILABLE " + fault.message : scene?.gpu.errors.length ? "ERRORS " + scene.gpu.errors.join("\n") : "No GPU errors"}\nPilot: full families/air/environment/TAA unqualified`;
  }
}
try {
  identity = await fetch(new URL("../identity.json", import.meta.url)).then(
    (r) => r.json(),
  );
  requestAnimationFrame(frame);
  await configure();
} catch (e) {
  fail(e);
}
