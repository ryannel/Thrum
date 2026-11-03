import { recordCanvas, validateMotion } from "./motion.js";
import { mergeReset } from "./requests.js";
import { Scene } from "./scene.js";
import { compareSourceMask } from "./source-mask-reference.js";
import { runGpuReferenceChecks } from "./gpu-reference.js";
import { Weather, WorldClock, validateProvider } from "./weather.js";
const observationRun = crypto.randomUUID().slice(0, 8);
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
  lastPresentedWall = 0,
  presentedFrames = 0,
  skips = 0,
  advanceTarget = null,
  benchmark = null,
  configurationVersion = 0,
  opticalEpoch = 0,
  observationSerial = 0,
  configurationPending = false,
  pendingReset = null;
const cpuSamples = [],
  gpuSamples = [];
let motionActive = false, fixtureRevision = 0;
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
    alpha: Number($("presentation-alpha").value),
    emptyWork: $("empty-work").checked,
    resolved: $("resolved").checked,
    extinctionScale: Number($("extinction").value),
    cachedVelocity: $("cached-velocity").checked,
    sourceDiagnostic: $("source-only").checked,
    viewSteps: Number($("view-steps").value),
    materialSpace: $("material-space").value,
    materialFilter: $("material-filter").checked,
    thermals: $("thermals").checked,
    materialCore: $("material-core").checked,
    materialStructure: $("material-structure").checked,
    traceFeed: $("trace-feed").checked,
    connectedFeed: $("connected-feed").checked,
    frontMorph: $("front-normal").checked,
    dualRest: $("dual-rest").checked,
    neckMix: $("neck-mix").checked,
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
function pauseAfterProof(){
  if(!$("pause-after-proof").checked)return;
  playing=false;$("play").textContent="Play";uiDirty=true;
}
function info(s) {
  status.textContent = s;
}
function abortMeasurement(reason) {
  fixtureRevision++;
  if (benchmark) {
    const aborted = benchmark;
    benchmark = null;
    receipt("measurement-aborted", {
      outcome: "aborted",
      reason,
      elapsedMs: performance.now() - aborted.started,
      sourceSeconds: (scene?.time ?? aborted.time) - aborted.time,
      rateRequested: aborted.rate,
      controls: aborted.settings,
      sourceSteps: (scene?.steps ?? aborted.steps) - aborted.steps,
      submittedImageGraphs: aborted.presentations,
      frames: aborted.frames,
    }).catch(console.error);
  }
}
function drainRequests() {
  if (configurationPending) {
    queueMicrotask(configure);
    return;
  }
  if (pendingReset) {
    const next = pendingReset;
    pendingReset = null;
    queueMicrotask(() => reset(next.clear, next.newWorld));
  }
}
function fail(e) {
  fault = e;
  abortMeasurement(e.message);
  info("Optics unavailable — " + e.message);
  console.error(e);
  receipt('optical-failure',{error:e.message,stack:e.stack,gpuErrors:scene?.gpu.errors??[],sourceTime:scene?.time,sourceSteps:scene?.steps,controls:options()}).catch(console.error);
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
    name:
      name +
      "-" +
      identity?.build +
      "-" +
      observationRun +
      "-" +
      ++observationSerial,
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
  abortMeasurement("Optical configuration changed");
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
      const next = await Scene.create($("view"), info, ui.provider, ui.family,ui.traceFeed);
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
    drainRequests();
  }
}

function startWorldEpoch() {
  weather = new Weather(71, {
    ...inputs(),
    storm: options().family === 1 ? 1 : 0,
  });
  clock.reset();
  advanceTarget = null;
}
async function reset(clear = false, newWorld = true, preserveClear = false) {
  if (busy) {
    pendingReset = mergeReset(pendingReset, { clear, newWorld, preserveClear });
    info("Source reset queued after observation/configuration");
    return;
  }
  if (
    scene?.sourcePipeline &&
    scene.dims[0] !== (options().family === 1 ? 128 : 96)
  ) {
    if (newWorld) startWorldEpoch();
    await configure();
    if (clear && scene) await reset(true, false);
    return;
  }
  if (!scene) {
    if (newWorld) {
      startWorldEpoch();
      fault = null;
      uiDirty = true;
      info("New CPU world epoch · optical source omitted");
    } else
      info(
        "Optical resources are omitted; enable the optical pipeline to create an epoch",
      );
    return;
  }
  abortMeasurement("Source epoch reset");
  busy = true;
  fault = null;
  try {
    await scene.gpu.device.queue.onSubmittedWorkDone();
    if (newWorld) startWorldEpoch();
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
    drainRequests();
  }
}
for (const id of ["moist", "dry", "lift", "wind"])
  $(id).oninput = () => {
    abortMeasurement("Weather target changed");
    try {
      weather.command(inputs());
      info("Weather command queued for next world commit");
    } catch (e) {
      fail(e);
    }
  };
$("play").onclick = () => {
  abortMeasurement("Playback changed");
  playing = !playing;
  $("play").textContent = playing ? "Pause" : "Play";
  uiDirty = true;
};
$("clear").onclick = () => reset(true);
$("reset").onclick = () => reset(false);
$("recover").onclick = () =>
  scene?.gpu.lost ? configure() : reset(false, false, true);
$("family").onchange = () => reset(false, true, true);
$("thermals").onchange = () => reset(false);
$("material-core").onchange = () => reset(false);
$("neck-mix").onchange = () => reset(false);
$("dual-rest").onchange = () => reset(false);
$("connected-feed").onchange = () => reset(false);
$("front-normal").onchange = () => reset(false);
$("trace-feed").onchange=configure;
$("trace-pause").onclick=()=>{
 if(!scene?.witness){info("Enable the trajectory proof first");return;}
 abortMeasurement("Trajectory proof paused");scene.witness.running=false;uiDirty=true;info("Tracing paused · snapshot retained; source continues");
};
$("camera-proof").onchange=()=>{
  if(!scene)return;
  const views={baseline:[.3,-.18],sunlit:[2.27,-.18],side:[-1.3,-.18],high:[.3,.20]};
  const [yaw,pitch]=views[$("camera-proof").value];
  Object.assign(scene.camera,{yaw,pitch,distance:scene.family===1?25000:7500,panX:0,panY:0});uiDirty=true;
};
$("front").onclick = () => {
  abortMeasurement("Weather front changed");
  uiDirty = true;
  weather.sendFront();
  info("Dry front queued · world owns its advance");
};
$("storm").onclick = () => {
  abortMeasurement("Storm forcing changed");
  uiDirty = true;
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
  abortMeasurement("Replay advance requested");
  advanceTarget =
    (scene.sourcePipeline ? scene.time : clock.time) +
    Math.max(
      0.25,
      Math.round(Number($("advance-seconds").value) / 0.25) * 0.25,
    );
  playing = false;
  $("play").textContent = "Play";
  info("Deterministic replay advance queued");
};
$("motion").onclick = async () => {
  if (busy || !scene || fault || motionActive || !playing) {
    info("Play a healthy scene before recording motion");
    return;
  }
  abortMeasurement("Motion recording requested");
  motionActive = true;
  const current = () => ({
    revision: fixtureRevision,
    fault: fault?.message ?? null,
    gpuErrors: scene?.gpu.errors.length ?? 0,
    rate: Number($("rate").value),
    imageFrame: scene?.lastPresentation?.frame ?? -1,
    epoch: scene?.epoch,
    sourceTime: scene?.time,
    worldTime: clock.time,
    settings: options(),
    weatherTargets: inputs(),
    camera: structuredClone(scene?.camera),
    playing,
    submittedFrame: scene?.lastSubmission,
  });
  const start = structuredClone(current()),
    name = `motion-${$("rate").value}x-${Math.round(scene.time * 100)}-e${scene.epoch}-${identity.build}-${observationRun}-${++observationSerial}`;
  const durationMs=Number($("motion-seconds").value)*1000;
  info(`Recording actual canvas motion for${durationMs/1000}s · excluded from timing`);
  try {
    const result = await recordCanvas($("view"),durationMs);
    const end = structuredClone(current());
    pauseAfterProof();
    const outcome = validateMotion(start, end);
    const valid = outcome === "fixed-fixture";
    const response = await fetch("/motion", {
      method: "POST",
      headers: { "Content-Type": result.mimeType, "X-Capture-Name": name },
      body: result.blob,
    });
    if (!response.ok) throw Error("Motion save failed: " + response.status);
    await receipt(name, {
      scope:
        "Actual canvas stream, not displayed-FPS or whole-GPU timing; start/end source times are queued snapshot metadata",
      capture: name + ".webm",
      outcome,
      start,
      end,
      elapsedMs: result.elapsedMs,
      requestedFps: result.requestedFps,
      mimeType: result.mimeType,
      bytes: result.blob.size,
    });
    info(
      "Motion saved · " +
        name +
        (valid ? "" : " · fixture changed, diagnostic only"),
    );
  } catch (e) {
    info("Motion unavailable · " + e.message);
  } finally {
    motionActive = false;
  }
};
$("probe").onclick = async () => {
  if (motionActive) {
    info("Finish the motion recording before observing or timing");
    return;
  }
  if (busy || !scene || fault) return;
  abortMeasurement("Observation requested: probe");
  busy = true;
  try {
    await receipt("source-" + Math.round(scene.time * 100), {
      probe: await scene.probe(
        options(),
        scene.sourcePipeline ? weather.sample(scene.time) : null,
      ),
      submittedFrame: scene.lastSubmission,
    });
    info("Source observation saved · separate from quiet timing");
  } catch (e) {
    fail(e);
  } finally {
    busy = false;
    drainRequests();
  }
};
$("capture").onclick = async () => {
  if (motionActive) {
    info("Finish the motion recording before observing or timing");
    return;
  }
  if (busy || !scene || fault) return;
  abortMeasurement("Observation requested: capture");
  busy = true;
  try {
    await scene.gpu.device.queue.onSubmittedWorkDone();
    const ui = options();
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
      scene.epoch +
      "-v" +
      scene.viewSize.join("x") +
      "-f" +
      scene.frame +
      "-" +
      identity.build +
      "-" +
      observationRun;
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
    drainRequests();
  }
};
$("bench").onclick = () => {
  if (motionActive) {
    info("Finish the motion recording before observing or timing");
    return;
  }
  if(scene?.witness?.running){info("Pause trajectory tracing before quiet timing");return;}
  if (busy || !scene || fault) return;
  abortMeasurement("Measurement restarted");
  benchmark = {
    started: performance.now(),
    frames: 0,
    presentations: 0,
    steps: scene.steps,
    frameStart: scene.frame,
    epoch: scene.epoch,
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
  if (motionActive) {
    info("Finish the motion recording before observing or timing");
    return;
  }
  if (busy || !scene || fault) return;
  abortMeasurement("Observation requested: tests");
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
    drainRequests();
    uiDirty = true;
  }
};

for (const [id, close] of [
  ["frame-cell", false],
  ["inspect-billows", true],
]) {
  $(id).onclick = () => {
    if (!scene) return;
    abortMeasurement("Camera changed");
    $("camera-proof").value="baseline";
    Object.assign(scene.camera, {
      yaw: 0.3,
      pitch: -0.18,
      panX: 0,
      panY: 0,
      distance: close
        ? (scene.family === 1 ? 15000 : 4000)
        : scene.family === 1
          ? 25000
          : scene.family === 4
            ? 16000
            : 7500,
    });
    uiDirty = true;
  };
}
$("mask-tests").onclick = async () => {
  if (motionActive) {
    info("Finish the motion recording before observing or timing");
    return;
  }
  if (playing || advanceTarget !== null) {
    info("Pause and finish any advance before the full-field observation");
    return;
  }
  if (busy || !scene || fault) return;
  abortMeasurement("Observation requested: mask-tests");
  busy = true;
  try {
    const result = await compareSourceMask(scene);
    await receipt("source-mask-comparison", result);
    info(
      result.pass
        ? "Whole-field source mask comparison passed"
        : "Source work mask differs from dense oracle",
    );
  } catch (e) {
    fail(e);
  } finally {
    busy = false;
    drainRequests();
    uiDirty = true;
  }
};

document.addEventListener("input", () => {
  abortMeasurement("Fixture input changed");
  uiDirty = true;
});
document.addEventListener("change", () => {
  abortMeasurement("Fixture controls changed");
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
  abortMeasurement("Camera changed");
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
    abortMeasurement("Camera changed");
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
    submittedGraphsHz: b.frames / ((now - b.started) / 1000),
    cadenceScope:
      "Successful graph/image submissions; source progress is queued for GPU execution; not completed-GPU or actual display cadence",
    submittedImageGraphs: b.presentations,
    submittedImageHz: b.presentations / ((now - b.started) / 1000),
    previewPolicy:
      "At16x at most20Hz and64x at most5Hz presentation while every required source tick executes; camera/control edits request immediate presentation; normal1x unchanged",
    sourceSeconds: scene.time - b.time,
    sourceSteps: scene.steps - b.steps,
    sourceAgeAtEnd: Math.max(0, clock.time - scene.time),
    pendingGraphsAtEnd: scene.gpu.pending,
    skips: skips - b.skips,
    controls: b.settings,
    rateRequested: b.rate,
    cpu: {
      p50: quantile(cpuSamples, 0.5),
      p95: quantile(cpuSamples, 0.95),
      p99: quantile(cpuSamples, 0.99),
    },
    gpu: {
      scope:
        "Presentation-graph queries only; source-only graphs excluded; accelerated total GPU work unqualified",
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
  pauseAfterProof();
}
function frame(now) {
  requestAnimationFrame(frame);
  const elapsed = Math.max(0, (now - last) / 1000);
  last = now;
  const cpuStart = performance.now();
  if (scene?.gpu.lost && !fault)
    fail(Error("Optical device lost; recreate optical epoch"));
  let target = null;
  if (advanceTarget !== null)
    target = Math.min(
      advanceTarget,
      (scene?.sourcePipeline ? scene.time : clock.time) + 8,
    );
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
    const paced =
      (playing || advanceTarget !== null) && Number($("rate").value) >= 16;
    const needsPresentation =
      !$("source-only").checked &&
      (uiDirty ||
        !paced ||
        now - lastPresentedWall >= (Number($("rate").value) >= 64 ? 200 : 50));
    const ui = {
      ...options(),
      alpha: playing ? Math.min(1, residual / 0.25) : Number($("presentation-alpha").value),
      sourceOnly: !needsPresentation,
    };
    if (ticks > 0 || needsPresentation) {
      try {
        if (scene.render(weather, ui, ticks, clock.time)) {
          frames++;
          if (scene.lastSubmission.presented) {
            lastPresentedWall = now;
            presentedFrames++;
          }
          uiDirty = false;
          lastRenderWorldTime = clock.time;
          const stamp = scene.timings.latest;
          if (stamp && stamp.frame !== lastGpuFrame) {
            lastGpuFrame = stamp.frame;
            if (
              benchmark &&
              stamp.epoch === benchmark.epoch &&
              stamp.frame >= benchmark.frameStart
            )
              gpuSamples.push(stamp);
          }
          if (benchmark) {
            benchmark.frames++;
            if (scene.lastSubmission.presented) benchmark.presentations++;
          }
        } else skips++;
      } catch (e) {
        fail(e);
      }
    } else {
      uiDirty = false;
      lastRenderWorldTime = clock.time;
    }
  }
  if (
    advanceTarget !== null &&
    scene &&
    (scene.sourcePipeline ? scene.time : clock.time) >= advanceTarget - 1e-6
  ) {
    advanceTarget = null;
    uiDirty = true;
    info("Deterministic advance complete");
  }
  if (benchmark) cpuSamples.push(performance.now() - cpuStart);
  if (benchmark && now - benchmark.started >= 10000) finishBenchmark(now);
  if (now - lastHud > 500) {
    const wall = (now - previousHudWall) / 1000,
      sourceTime = scene?.time ?? previousHudTime,
      delivered = (sourceTime - previousHudTime) / Math.max(wall, 0.001);
    previousHudWall = now;
    previousHudTime = sourceTime;
    lastHud = now;
    const g = scene?.timings.latest;
    $("stats").textContent =
      `World ${clock.time.toFixed(2)}s · tick ${weather.tick} · epoch ${clock.epoch}\nQueued source ${scene?.sourcePipeline ? scene.time.toFixed(2) + "s" : "not used"} · ${scene?.steps ?? 0} steps\nRequested ${playing ? $("rate").value : 0}x · source delivered ${delivered.toFixed(1)}x\nSubmitted image ${scene?.lastPresentation?.time.toFixed(2) ?? "—"}s · preview20Hz/5Hz at16x/64x\nOptical epoch ${scene?.epoch ?? "none"} · age ${scene?.sourcePipeline ? Math.max(0, clock.time - scene.time).toFixed(2) + "s" : "—"}\nGraphs ${frames} · images ${presentedFrames} · queue ${scene?.gpu.pending ?? 0}/3 · skipped ${skips}\nGPU ${g ? (g.wholeMs === null ? "cross-pass timing unqualified" : g.wholeMs.toFixed(2) + "ms / ordered query interval") : "unavailable"}\n${g ? "source " + (g.sourceMs ?? 0).toFixed(2) + " · light " + (g.lightMs ?? 0).toFixed(2) + " · view " + (g.viewMs ?? 0).toFixed(2) : ""}\nLogical live textures/buffers ${scene ? (scene.gpu.bytes / 1048576).toFixed(1) : "0"}MiB\n${scene?.provider === 0 ? scene.dims.join("×") + " dense source reference" : "Volume source retired"}\nWeather rain ${weather.state.storm * 0.002} kg/m²/s · causes ${weather.events.length}\nBuild ${identity?.build ?? "loading"}\n${fault ? "UNAVAILABLE " + fault.message : scene?.gpu.errors.length ? "ERRORS " + scene.gpu.errors.join("\n") : "No GPU errors"}\nPilot: full families/air/environment/TAA unqualified`;
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
