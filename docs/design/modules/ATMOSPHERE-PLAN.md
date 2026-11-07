# Atmosphere milestones

Design: [ATMOSPHERE.md](ATMOSPHERE.md)
Reviews: Optical design and plan over `a91cdd0e`, final design `f9d0c12a` and plan `c3cc5748` · [optical brief](#optical-review-brief) · initial changes required, final M1 implementation cleared. Frame graph and ownership at the same revisions · [graph brief](#graph-review-brief) · initial changes required, final design cleared. First valley captures, Metal build `dfae5b21` and Chrome Wasm `bc15ea7d` (SHA-256 prefixes) · [capture brief](#capture-review-brief) · held ground view passes, M1 remains open. Chrome late-twilight Wasm `134e271c` · [sweep brief](#sweep-review-brief) · silhouette and reflection visible, warm contour and live motion open. Packed reflection candidate over `c2ac94d8`, Wasm `132f992f` · [reflection brief](#reflection-review-brief) · visual improvement, rejected on measured Metal cost. Four-lane candidate over `4fecd095`, Metal `c6e7b102` and Chrome `4afa8b38` · [four-lane brief](#four-lane-review-brief) · held visuals pass with faint residual streaks; no code findings; live cadence open. Live captures over `16dc9da2` · [follow-up brief](#four-lane-review-brief) · late-twilight progression passes; early object shadow pop verified on both targets; physical cadence open. Shadow partition over `9f13c5f6`, Metal `510e62ce` and Chrome `c6500ef7` · [shadow brief](#shadow-review-brief) · fixed three ownership and coverage gaps; matched visual checks pass. SDR comparison over `1f4e6932`, Metal `41d11c1d` and `1baf73d5` · [tone-scale brief](#tone-scale-review-brief) · keep Neutral Highlights. Local ground material over `ec78a0d7`, Chrome `8d832924` · [ground brief](#ground-material-review-brief) · selected, existing lookout lip remains conspicuous. M1 closeout source `b8f230e7`, Chrome `46d3eb04` and Metal `4928bca0` · [closeout brief](#m1-closeout-review-brief) · sampled crossing and exposure pass on both targets, including the completed foreground Metal sweep through midnight. The callback-tail tripwire stays open. M2 look criterion over `590cee61` · [midnight look brief](#midnight-look-review-brief) · added an unlabelled shipping-frame witness and a same-build star toggle. M2 integration over `6fa34365`, corrected WIP `1fa3228` · [star integration brief](#star-integration-review-brief) · camera, opt-in and altitude findings fixed on recheck. Tight footprint over `a61bb80`, Chrome `568edbaa` · [tight footprint brief](#tight-footprint-review-brief) · compact held stars and sampled tracking pass. Required a positive central night witness. Live faint-star stability remains open. Harvard source pack `3a3536c7` and opt-in valley `98e0be1`, Metal `a7588eed` and Chrome `ced775e0` · [source and valley brief](#catalogue-and-valley-review-brief) · original values reproduced, sampled midnight, occlusion, pan and emergence pass. Late sky rebound and live cadence remained open. Twilight shoulder WIP `6a2dbf6`, Metal `c4700ab7` and Chrome `80cd702c` · [shoulder brief](#twilight-shoulder-review-brief) · sampled look passes for the checkpoint, with a small residual rise in one sky patch. Paired cost and live cadence remain open.
Updated: 2026-11-07
Milestone: M2 stars
Now: opt-in WIP `6a2dbf6` reduces deep-twilight exposure rebound on Metal and Chrome with the audited 6,632-star catalogue.
Next: collect paired Metal cost, qualify a conservative daylight selector and review a live pan of the catalogue.
Blocked: none. Physical cadence and the callback-tail tripwire remain with frame-runtime qualification.

At the next look pass, the cooked ground should read as mineral material at the fixed
showcase camera, with fine grain at chase distance and continuous detail into the
middle distance. Judge the lookout lip and texture transition at full size and
in a slow pan.

## M1 The valley remains legible at moonless midnight: done

Done when

- [x] On Metal and Chrome, the actual cooked valley at 0 h has a dark nonzero sky and a legible terrain silhouette, with diffuse rock and metal both responding to the same weak environment. Independent review passed both fixed cameras at native scale. [Captures](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] The raw sky zenith and horizontal integral are on the intended dark-site scale. The packed field matches the converged scalar reference within 5% for horizontal irradiance and 10% per sampled zenith, peak, and split-horizon ray at 0, 1.8 and 15 km. Interpolated night in-scatter at 1, 10 and 30 km agrees within 10%. The tolerances qualify the implementation of this optical approximation. Switching off nightglow affects sky, meter, diffuse and reflection together, and restoring it changes no direct sunlight. [Referees and source switch](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] A continuous sunset-to-night sweep has no exposure freeze while light still falls, storage-scale step, or broken shadow transition. Alternating bright and dark submissions across captures, drops, slot reuse and device replacement preserve the correct scale and pixels. A 1% Lambert control survives both devices' real HDR attachments and display paths. Dawn, noon and sunset retain their checked images. [Motion, held captures and storage evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] Pair whole-frame and pass cost with the old run and investigate tripwires. Workspace check and real window and browser motion runs pass independent visual review. [Paired costs, remaining tripwire and review](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).

Work

- [x] Reproduced a completely black midnight frame on the actual cooked valley at a91cdd0e. The scalar range check confirms the fixed fp16 scale loses a 10% Lambert surface.
- [x] The independently stepped scalar direct-plus-scatter probe resolves the positive-elevation peak and both limits of the curved horizon at 0, 1.8 and 15 km. The denser quadrature changes the sampled radiance by at most 0.063%.
- [x] A second scalar reference includes the finite shell, both components, the engine phase function, ground irradiance and finite paths. Raw source and rows are in `tmp/g3s-showcase/night-design/`.
- [x] The scratch RG16Float profile matches the dense reference on 59 distinct sky/ground rays, six irradiance rows and three finite paths. Maximum errors are 0.381%, 0.028% and 0.060% respectively. The expanded sweep includes 2 m and 6 m near-tangent rays. These are packed CPU reads.
- [x] Wire the night channel into sky, diffuse, reflection and finite aerial transport. Connect the per-view exposure sample to all forward colour writers. The GPU meter test reaches −12 EV100 and preserves a low-light arithmetic control.
- [x] Compare the production Metal sky, incident reduction and finite aerial decoder to the f64 reference. Toggle the source through the existing graph and check that its dependent caches refresh. [Observations](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] A 1% Lambert-equivalent radiance control survives the actual HDR and SDR attachments on Metal and Chrome. Metal covers dropped encodes, captures and both reused exposure sides. Paused midnight pixels survive real GPU replacement on both targets.
- [x] Compare held nightglow-off and on scenes, including the specular response. Both targets go from black to readable at midnight and noon differs by at most one RGB code.
- [x] Capture and inspect actual valley sequences. Review the raw data and record results.

## M2 Stars emerge as the observer turns and time advances: in progress

Done when

- [ ] From the same valley views, star appearance follows the dimming sky; star positions match the chosen observer rotation, stay stable under slow pans and are occluded by terrain. Hold bright day, late twilight and moonless night; watch motion on Mac and Chrome.
- [ ] At the fixed showcase camera, midnight reads unmistakably as a clear night while the terrain silhouette stays legible. Judge the unlabelled shipping field at native size before seeing the late twilight and M1 controls. Compare stars on and off at the same pose, exposure and display settings to see what the stars contribute. Review a slow pan across the skyline on Mac and Chrome.
- [ ] Integrated test-star flux stays within 5% across subpixel position and 720p/1080p/2160p. At matched night exposure, magnitude 6 is dimmer than magnitude 3 by a visible margin in the resolved footprints. Verify the rotation with known directions, including the time-slider wrap. A star centred behind the skyline leaves no sprite leak. Validate at image centre and edge.
- [ ] The catalogue provenance is complete, paired runs record frame cost and attachment traffic, and the workspace and independent in-game review pass.

Work

- [x] Verify data rights and derive the compact immutable star records. The original 1908
  Table I subset packs 6,632 stars across all 24 hours. The opt-in checkpoint
  retains the transcript, conversion format and source and pack checksums.
- [ ] Add the background draw with conservative footprint and qualify extent, depth and exposure changes.
  The scratch Metal referee conserves flux to 0.053% across 36 raster cases. The
  actual valley WIP adds three visible controls at midnight, with a fourth hidden
  by the ridge. Removing centre rejection leaks into 58 sky pixels.
  [Prototype evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [ ] Run the in-game digest and close with evidence. Lunar illumination and the broader G3s calibration and shadow checks remain separately tracked.

## Review briefs

The initial reviews found the EV floor, fp16 precision, shared scale selection,
graph timing order, finite aerial in-scatter and star occlusion/flux gaps. Each
reviewer rechecked the finite-shell revision. The GPU and in-game checks are open.

### Optical review brief

> Read-only, bounded independent design/plan review for Thrum (cwd /Users/ryannel/Workspace/thrum). First read AGENTS.md and derive requirements independently from docs/BUILD-PLAN.md G3s and docs/design/RENDER-SPEC.md §5: a clear Earth sky in the cooked V1 valley on Mac Metal and Chrome from daylight/twilight through readable moonless night, one atmosphere provider, physical incident meter with retained low-light key, source rights PD/CC0 only, performance tripwires not gates. Then examine draft docs/design/modules/ATMOSPHERE.md and ATMOSPHERE-PLAN.md, raw analytical range tmp/g3s-showcase/night-design/range.py and range.log, and primary sources if needed. Assess the night lighting/optical and celestial design, mechanisms vs promised cues, and independent done-when checks. Use engine-module-design references/review.md and review-agent defect-first discipline. Give verified findings with exact references or label suspected, plus remaining gaps. Do not modify files, run GPU tests, or delegate. Reply once findings are ready.

### Graph review brief

> Read-only independent Thrum design/plan review focused on frame graph, storage and cross-module ownership. Cwd /Users/ryannel/Workspace/thrum. Read AGENTS.md then independently derive requirements from docs/BUILD-PLAN.md G3s and docs/design/CONTRACTS.md C07/C08 (same scene Metal/Chrome, continuous time, one provider, same exposure per rendered slot, no authoritative impact or needless full frame work). Then read proposed docs/design/modules/ATMOSPHERE.md and ATMOSPHERE-PLAN.md, inspect code consumers sky/diffuse/reflection, synthetic forward/display and cooked pipeline and runtime lifecycle as needed. Use .agents/skills/engine-module-design/references/review.md and defect-first review-agent guidance. Identify any incomplete API, graph hazard or qualification gap, with exact references, verified/suspected. Do not edit, run GPU workloads or delegate. Reply once findings ready.

### Capture review brief

> Independent, read-only review of Thrum’s first M1 in-game night checkpoint. Repo
> /Users/ryannel/Workspace/thrum. Use .agents/skills/engine-module-design/references/review.md.
> Original criteria: same cooked valley on Metal/Chrome, dark nonzero midnight sky,
> terrain silhouette legible, both diffuse rock and metal respond to shared weak
> environment; continuous sunset to night, dawn/noon/sunset imagery retained,
> distinguish evidence gaps from failures. Inspect raw native Metal captures and
> sequence in tmp/g3s-showcase/night-design/night-first/ plus reference captures
> tmp/g3s-showcase/sky-contours/normal/. Do not take over GPU, alter repo, or
> delegate. Candidate build is identified by captures.json and night-first-midnight.png
> in parent folder. Inspect images directly and state per cue pass/fail/can’t tell,
> still versus motion and scale. Also read-only inspect night shader/exposure
> wiring as needed for consequential problems; exact evidence or label suspicion.
> Browser and live qualification may not exist yet. Give brief defect-first result.
>
> Chrome actual cooked valley raw artifacts just completed in
> tmp/g3s-showcase/night-design/m1-chrome-valley/ (Wasm SHA
> bc15ea7de14271d5332dfff37136e5d699a02ed5a195c86e8911d455552b5631).
> Include its phase-scene-midnight in the visual inspection if able, and scope
> evidence correctly.

### Sweep review brief

> Read-only independent visual review for Thrum M1. Repo /Users/ryannel/Workspace/thrum,
> .agents/skills/engine-module-design/references/review.md. Raw Chrome real cooked-valley
> samples at tmp/g3s-showcase/night-design/m1-transport/chrome-late-front/ (run.json,
> valley.json, valley-late-*.png, valley-phase-upslope-midnight.png, contact.png,
> sampled-late-twilight.mp4). Requirements: dark nonzero sky, legible ridge/ground,
> reflected sky and visible diffuse rock at midnight, and smooth solar colour and
> exposure transition from sunset through moonless night. Point out any nonphysical
> contours, shimmer, plateaus or jumps by image/time and distinguish cadence claims
> from sampled edits. Inspect at native scale and sequence directly where available.
> No file edits, GPU runs or delegation. Return defect-first per-cue pass/fail/can't tell.

### Reflection review brief

> Independent read-only Thrum reflection checkpoint review. Use the module-design
> review reference. Remove objectionable warm contours during the cooked valley’s
> late twilight while preserving a physical soft reflected horizon, readable
> midnight, daylight appearance and frame cost tripwires. Inspect the three
> atmosphere reflection files over c2ac94d8, the raw candidate Chrome capture
> and clip, its raw ROI rows, the matched baseline and dense captures under
> tmp/g3s-showcase/night-design/. Judge artifacts at native size, state whether
> you watch sequence, flag regressions and whether the two workgroups index
> the same packed record. Keep cadence and headless samples separate. No writes,
> GPU runs or delegation. Return verified/suspected findings and per-cue verdicts.
> The follow-up supplied matched offscreen Metal stills from the same game host.

### Four-lane review brief

> Review the three reflection files over `4fecd095` with review-agent and
> the module-design review reference. Check packed offsets, lane and edge
> synchronization, portable assumptions and call sites. Inspect candidate
> held Chrome and Metal captures and sweep, the matched shipping and dense
> references, and the raw paired timing runs in the handoff's reflection
> scratch directory. Judge the warm contour, reflected horizon, readable
> midnight, daylight and material regressions. Separate cadence from held
> frames. Give defect-first findings and per-cue verdicts. No GPU runs or edits.
> Follow-up at `16dc9da2`: inspect chronological native foreground and
> Chrome live images and timestamps, plus the same-Wasm no-screenshot
> run. Judge the direct-shadow transition, sky and meter progression,
> reflection and limits of the cadence evidence. Recheck the early handoff
> using consecutive Metal frames, closely spaced Chrome captures and the
> terrain-only ray reference. Judge each M1 done-when check independently.

### Shadow review brief

> Review the renderer diff over `9f13c5f6` with review-agent. Follow the
> cascade fit, caster clipping, sampling, fallback and GPU fixtures.
> Inspect the matching Metal and Chrome shadow-crossing sequences and
> daylight/night stills in the handoff's scratch paths. Judge the local
> tails, distant blockers, contact shadows and frame cost from raw pairs.
> Report defects and which milestone cues pass. Separate timed samples
> from presentation cadence. Read only.

### Tone-scale review brief

> Review the matched Metal valley captures and fixed sunset route in the
> [handoff](../../evidence/v1-workload/G3S-HANDOFF.md#exposure-audit-decision).
> Judge low-Sun colour, disk separation, ground contrast, midnight landmarks and
> motion continuity. Use the photos for visual cues and separate offscreen motion
> samples from live pacing. Read only.

### Ground material review brief

> Compare the matched native and Chrome valley captures referenced in the
> [handoff](../../evidence/v1-workload/G3S-HANDOFF.md#exposure-audit-decision).
> Judge contrast, haze, landmarks, repeat, mip stability and visible material
> boundaries from noon through midnight. Separate sampled motion from live
> cadence. Read only.

### M1 closeout review brief

> Independently review the cooked valley from sunset shadow crossing through
> moonless midnight. Use the raw Chrome and foreground Metal captures and
> timestamps for `b8f230e7` in `tmp/g3s-showcase/night-design/m1-closeout-b8f230e7/`,
> the earlier offscreen Metal crossing in `partition-valley-release-metal/`
> under `m1-reflection-isolation/`, and raw paired costs. Judge exposure
> continuity, storage steps, shadow handoff, midnight diffuse and specular
> landmarks, daylight retention and tripwires separately. State the build and
> whether you watched motion. Distinguish compositor samples from physical
> cadence and visible evidence from storage arithmetic. Read only.

### Midnight look review brief

> Independently judge whether the M2 acceptance check separates a clear
> moonless night from the M1 grey-dusk read while preserving a legible ridge.
> Use the M1 midnight still and the current star design. Identify a decisive
> visual witness at the shipped camera and separate that verdict from flux
> and placement correctness.

### Star integration review brief

> Review the exact WIP over `6fa34365` against the atmosphere design. Trace
> colour producer order, the catalogue and camera ABI, opaque occlusion,
> atmosphere and exposure ownership through slot reuse and replacement.
> Inspect the raw native on/off pairs and Chrome captures, then the wide and
> tight provisional fields at full size. Separate stills from motion and
> the optical controls from the shipping catalogue. Read only.

### Tight footprint review brief

> Inspect the diff over `a61bb80` and raw Chrome valley captures
> `valley-tight-chrome` and `valley-tight-chrome-pan` in
> `tmp/g3s-star-catalog/`, with the prior wide Chrome captures and
> `metal-star-tight-referee.log`. Check subpixel brightness, terrain
> occlusion, lit-sky retention, the above-horizon day and night witness
> and integrated flux at 720p, 1080p and 2160p. Inspect the images and
> state whether evidence is from stills or motion. Read only.

### Catalogue and valley review brief

> Inspect the original Table I scans, hour map, transcription, repairs and
> pack under `tmp/g3s-star-catalog/harvard-1908/layout-pages/`. Check
> rights, source-only values, epoch, hour boundaries, signs and bright gaps.
> Inspect the reviewed Metal on/off captures and Chrome Wasm `ced775e0`
> in `tmp/g3s-star-catalog/harvard-reviewed-*`. Check clear night at the
> showcase camera, the ridge mask, pan stability and twilight emergence.
> State which observations establish stills, sampled poses or live motion.

### Twilight shoulder review brief

> Inspect the shader diff over `98e0be1`, the physical GPU referee and the
> matched Metal and Chrome captures under `tmp/g3s-star-catalog/`. Check
> exposure and storage ownership, the night landmarks, late sky progression,
> and retention of the previously qualified light. Call out any residual
> rebound, and distinguish sampled poses from live presentation. Read only.

## Log

- 2026-11-07 — started the moonless-night milestones after the cubic LUT probe produced no appreciable SDR improvement.
- 2026-11-07 — the owner accepted M1 and added the midnight look check to M2 after inspecting the foreground sweep.
