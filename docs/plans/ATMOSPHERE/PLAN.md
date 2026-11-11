# Atmosphere milestones

Design: [ATMOSPHERE.md](ATMOSPHERE.md) · Done when: [ATMOSPHERE-DONE.json](ATMOSPHERE-DONE.json) (M5 onward) · Module list: [ARCHITECTURE.md](ARCHITECTURE.md)
Reviews: design review of M5–M6 at `4bbe44a4`: changes requested, two Must Fix · [first review](../../evidence/ATMOSPHERE/design-review-1.md). Second design review at `27473469`: changes requested, one Must Fix · [second review](../../evidence/ATMOSPHERE/design-review.md). The design and items were revised on 11 November; a third design review is owed. Reviews of M1–M4 are in the log.
Updated: 2026-11-11 (revised after the second design review)
Milestone: M5 Other modules read the one sky and Sun, and one cloud optical depth dims sunlight once
Now: M5 and M6 revised after the second design review; nothing built under them yet. The stock sunset graph ships; M3 stays a parked spike and M4 deferred (since 2026-11-10)
Next: design review of the revision, then M5. M6 waits on `APPLE-PRESENTATION:M3`.
Blocked: none for M5. The parked M3 spike has no production candidate; its re-entry terms are under M3 below.

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

## M2 Stars emerge as the observer turns and time advances: done

Done when

- [x] **D1.** From the same valley views, star appearance follows the dimming sky; star positions match the chosen observer rotation, stay stable under slow pans and are occluded by terrain. Hold bright day, late twilight and moonless night; watch motion on Mac and Chrome. [Motion evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] **D2.** At the fixed showcase camera, midnight reads unmistakably as a clear night while the terrain silhouette stays legible. Judge the unlabelled shipping field at native size before seeing the late twilight and M1 controls. Compare stars on and off at the same pose, exposure and display settings to see what the stars contribute. Review a slow pan across the skyline on Mac and Chrome. [Matched views](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] **D3.** Integrated test-star flux stays within 5% across subpixel position and 720p/1080p/2160p. At matched night exposure, magnitude 6 is dimmer than magnitude 3 by a visible margin in the resolved footprints. Verify the rotation with known directions, including the time-slider wrap. A star centred behind the skyline leaves no sprite leak. Validate at image centre and edge. [Metal and wrap recheck](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] **D4.** The catalogue provenance is complete, paired runs record frame cost and logical attachment area, and the workspace and independent in-game review pass. Physical bandwidth is tracked separately below. [Costs and scope](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).

Work

- [x] D4. Audit and pack 6,632 original 1908 Table I stars, preserving the source rows and checksums.
- [x] D1–D2. Run the normal valley launch and same-build controls on Metal and Chrome. Review held E composition and the continuous Chrome stream. [Evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Decode and inspect both 1× Metal offscreen sequences for pan and emergence, with submitted-frame GPU timings. [Evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Play both Metal 1× sequences through and inspect the moving images. The all-frame decode remains the native-size masking and stability evidence. [Playback](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D3. Qualify the candidate's production Metal flux, centroid, two-axis depth masking and central SDR contrast. Exercise both Chrome slider endpoints and adjacent night settings. [Evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D4. Recheck the admitted route and record paired Metal costs and logical attachment area. [Raw-run summary](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D4. Record paired Chrome full-frame costs at midnight and twilight. Recheck the full evidence independently.
- [x] D1–D3. Requalify the corrected sky with named catalogue stars, the slider wrap and matched Metal/Chrome night and pan captures. [Requalification](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).

Apple tile M0.4 replayed the matched offscreen star traces in Xcode on the M3 Max. The single pair shows 46.20 to 48.63 MiB of device-memory traffic, with about ±1 MiB per encoder uncertainty. No time comparison comes from the replay. [Counter evidence](../../evidence/render/G3T-TRAFFIC-PROBE-2026-11-08.md).

<a id="m3-sunset-crossing"></a>

## M3 The west lookout’s shadow crosses the ground without a flash: parked spike

Owner review: [Half Dome sunset from Glacier Point, fixed-camera clip by G. Edward Johnson](https://commons.wikimedia.org/wiki/File:Sunset_on_Halfdome_timelapse_Yosemite_CA_2023-07-15_20-11-06_1.webm) (proposed, pending).

Use the moving shadow edge on the rock as the reference for continuity and
texture through the transition. The camera, terrain scale, palette and
time-lapse speed differ. Compare the clip side by side with the fixed
17.40–17.80 h west-lookout run at 1× game playback. Inspect the separate
±20° game pan at native size for camera-dependent ribbing. The clip is
an external CC BY 4.0 reference. Only its link is tracked.

**Diagnosis handoff.** The 16:40 CET overseer ruling parks this outcome
at 0/3 and assigns a fresh-context Claude session. The same-field replay
isolates an in-cascade sampling failure on the authored spur, while the
shipped Ground E pan has a separate terrain lip. The optional resolve
reduced the measured step and introduced visible ribbing on Metal and
Chrome. Keep stock, the fixed motion and both distinct observations.
Start with the [stock control and motion handoff](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint)
at `282cc039`. The full return criteria below remain open.

Done when

- [ ] **D1.** At the west lookout on the cooked valley, the near ground and middle distance read as continuous terrain during the 17.40–17.80 h crossing and a slow ±20° pan on Metal and Chrome. Independent side-by-side motion review matches the reference’s continuous moving edge. The timed 1× replay shows no newly introduced flecks, ribbing or detached bright bands. The fixed 1080p spur witness removes the known row-665 in-cascade flash at frames 226–227, and the six-stop route and both controls pass.
- [ ] **D2.** Compare against current main at the same receiver, source and Sun. The open Sun and full occlusion controls retain their expected result. The broad edge, close terrain, distant crest, represented compact-caster control and overlap exhibit no new visible false light or shadow across near/last/far range, camera and azimuth sweeps, or a cooked asset swap. Review full-resolution visibility and shaded images, with signed difference masks registered by receiver and time. Report new lit/umbra regions separately from the existing errors and compare asset replacement with a fresh run of the replacement. Record signed disk-reference error on the fixed witnesses as a measured open item. The original sub-texel caster stays in M4.
- [ ] **D3.** The regular graph preserves aerial and surface ownership, recovery and strict discarded-frame image equality. Record paired whole-frame and shadow costs, and pass workspace and independent code and visual reviews on the integrated result.

Work

- [x] D1. Freeze the 1080p crossing and 1× pan, route and controls, plus the exact D32 inputs for frames 226–227. The failing receivers are inside the last cascade in both frames. [Evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Review and screen contour-area PCSS on the eight saved D32 rows. Preserve the literal failure. The revised merged-interval preflight recovers the limb within the original work caps. [Evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Replay 83–85 byte for byte and capture its D32 fields. Preserve the failed transition screen and its stock-plateau diagnosis. [Evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Run the separately fixed extinction trajectory. The bounded query rejects its fallback on entry. [Evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D3. Review the production PCSS combination against the fixed trajectories and the composed graph. The independent review conditionally cleared one screen with the protocol below. [Brief](#m3-production-pcss-review-brief).
- [x] D1–D3. Freeze the crossing and pan inputs and prepare the stock, fixed and rotated builds. Reproduce the stock crossing on Metal. [Baseline](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D3. Run the first matched production PCSS screen. Both targets reproduce the original flash and reject rotated-32 for regular penumbra crosshatching. [Results](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D3. Review same-frame receiver-guided filtering and its cost. Reviewers clear one raw diagnostic and hold the live graph.
- [x] D1–D2. Record the frozen Metal receiver guides and evaluate the fixed radius-one filter. The visibility mask is smoother and the strongest row-averaged edge stays in place. The authored compact and wider control are silent in this evaluator. [Diagnostic](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D2. Trace the existing wide-caster comparator through the current fitted D32 atlas. Near clips it, last keeps a closer surface at its two projected centres, and the height layer contains no covered texel centre. Keep the historical split-map result scoped to that renderer.
- [x] D2. Freeze and qualify the separate stock-positive caster at 160 m. The original projects to 0.20 × 0.47 far texels, so retain it in M4. The new control wins 25 D32 samples and shows wide-filter edge, lit and umbra rows on the same receiver depth. [Evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D2. Prove the fixed 3×3 offline filter preserves the frozen stock-positive shadow profile and clean regions without broader support or new guides. [Evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Establish the same coincident synthetic/cooked winner in isolated Metal pipelines. The two solo draws, both tie orders and a distinct-depth control pass with the production vertex entries and cooked Uint32 multidraw.
- [x] D1–D2. Verify the fixed GPU filter against the CPU oracle on the saved caster, crossing and extinction regions. Both backends pass without widening the filter or changing guard thresholds. [Evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Prove exact live cooked/synthetic D32 ties and winner identity on Metal and Chrome, with reversed order and later-nearer controls. Both forced ±2 ULP arms render stock inside the fixed crop and the resolved image outside.
- [x] D1–D2. Compare live mask acceptance in both draw orders on 235 filterable 160 m ties in Metal and Chrome. Their raw tied centres are zero. The shaded edge and overlap still need independent controls for any future graph.
- [x] D1–D3. Screen the optional live resolve on the three frozen 1080p/60 trajectories and retain the failed result. [Evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D2–D3. Establish stock baselines for the low-Sun Chrome cooked swap, native discarded-frame image equality and device restaging, and Chrome valley recovery. [Baseline](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [ ] D1–D3. Resolve cooked and tile ownership before a live candidate. Keep the submitted-slot history rules and three-screen stop limit.
- [ ] D1–D3. Integrate one shared surface evaluator, then capture the shipped ground and spur replays, range and cooked replacement controls, and paired frame costs. Review both targets in motion before promotion.

## M4 Exact finite-disk shadow reference match: deferred bounded spike

Done when

- [ ] Compare one reviewed representation with the original-geometry disk reference at the broad edge, distant narrow crest including moving receivers, the 64 m close terrain exclusion, the 24 cm compact caster at 160 m and the overlap. Keep the open Sun control and one coverage union. Use the frozen raised, distant, broad and close 4 m patches and the four row-665 receiver/phase domains at x=480, 720, 960 and 1200. Keep the crest-only ablation, close umbra, compact-only and overlap controls. Retain a 0.5 percentage-point continuous omission bound. Prove the retained blocked set is a subset of the reference or separately bound overcoverage to the same tolerance. Freeze finite sample and reference integration tolerances before the run.
- [ ] Publish the verdict and the complete source, receiver and caster cost for the measured domain. Only a passing candidate can propose a separate in-game landing.

Work

- [x] Preserve all fixed fields, receiver paths, phases, source hashes, triangle/ray references, and failed variants. [Witness inventory and reruns](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint). The prior work-item list is retained at `9d1cabbb`.
- [ ] Review at most one representation with a distinct source-side reuse mechanism and explicit producer, receiver and caster work caps fixed before running. Run it once against the frozen witnesses. Park it if it exceeds the cap or fails admission, retain the failure and stop. M3 and other V1 gates can proceed independently.

## M5 Other modules read the one sky and Sun, and one cloud optical depth dims sunlight once: not started

Design: [Environment identity and products](ATMOSPHERE.md#environment-identity-and-products). Items and proofs are
in the done-when file. This is the first of [the port's readiness conditions](ARCHITECTURE.md#where-the-ports-readiness-conditions-are-met).

- [ ] Record the base commit and the per-pipeline sampled-texture counts before changing the light group.
- [ ] The environment identity, with the true Sun kept apart from the apparent surface light, lent to
      contributions each frame.
- [ ] The lent layout and WGSL library with the five operations, reflection included, each documented with the
      support and limits the design's table gives it.
- [ ] The handoff referees H1–H5 of [the numerical acceptance](ATMOSPHERE.md#numerical-acceptance) on Metal, run
      from a contribution's own pipeline; the preserved referees P1–P11 untouched.
- [ ] For H2 and H4, the f64 filtered decode written apart from the library, its live and known-field arms, and
      the wrong lookups that must fail; one case of each row repeated at a non-unit exposure scale.
- [ ] The browser page and the `environment` case of `scripts/check_v1_browser.mjs` that run H1–H5 in Chrome,
      with the same filtered decode, wrong lookups and exposure control.
- [ ] The Sun-path optical-depth product, the one cloud-factor function, and the shader variant in which the
      shared direct-Sun term calls it; the factor is exactly 1 outside the field's support; no provider composes
      today's shader.
- [ ] The cooked volume's march calls the same cloud-factor function at each lit march sample's world position.
      With no provider its shader text and lighting are today's.
- [ ] GPU tests for M5.6 on every receiver kind, including a field that differs between two world points.
- [ ] `scripts/environment_products.py`: the off-parity arm against the base on Mac and Chrome, and the paired
      cost arm on Mac and Chrome.
- [ ] The labelled diagnostic provider in `examples/v1-game`, opt-in, and its captures at noon and low Sun.

The light group and forward pipelines are also edited by `APPLE-PRESENTATION:M2` (the third forward target).
Whichever lands second rebases; neither waits on the other.
The cooked volume's shader is in `thrum-asset-gpu`, which the same `APPLE-PRESENTATION:M2` work also touches; M5
changes only the Sun term of its march.

## M6 The valley's sky and sunlight pass the route in motion, and G3s closes: waiting

Design: [Closing G3s](ATMOSPHERE.md#closing-g3s). Needs `APPLE-PRESENTATION:M3`.

- [ ] Recapture the route and the dawn-to-night scrub with TAAU on, on Mac and Chrome, and list which recorded
      defects still show.
- [ ] Fix what shows, one defect at a time, with the referees green after each and no reference, case or bound
      changed.
- [ ] Record the commit M6 starts from, and rerun it beside the final build for the cost comparison; quote the
      M1 and M2 records as history.
- [ ] Maturity rows and the build plan's G3s line, naming solar sky and aerial accuracy as not qualified.

## Review briefs

### M3 live tie and fallback review brief

Read the source and independent captures in `live-game-tie/` and
`live-shaded-fallback/` under the M3 opaque-winner scratch folder.
Decode actual receiver and forward planes for each tied and
strictly nearer arm. Check all positive solo ties and guide flags.
For the shaded witness, compare inside and outside the frozen
crop to stock and the unforced resolve. Require the same cooked
manifest, matched time and repeat controls on each target.

### M3 live depth guard review brief

Review the raw forward and receiver D32 planes, RG16 tags and
named timestamps under
`tmp/g3s-sunset-lip/graph/production-pcss/opaque-winner/`
over `d7738029`. Check exact stored depth, the signed fragment Z
difference and the fixed crop. Compare the guard at one and two
ULPs with the same data and coverage. Distinguish a predicate
refusal from observed shaded fallback. Recompute matched timing
deltas before attributing the tripwire.

### M3 live receiver review brief

Review the exact diff over `293c0524` in
`tmp/g3s-sunset-lip/graph/production-pcss/opaque-winner/`.
Check raw raster eligibility, exact graph dependencies and timing,
the receiver and forward layout split, and the per-pixel D32 guard.
For cooked meshes, verify one tentative list and draw order,
whole-view fallback on pending or installed hooks, and submission
as the only commit. Startup capture qualifies graph execution.
The integrated guard and frozen motion checks are separate.

### M3 packed GPU resolve review brief

Check both WGSL forms, saved inputs, original CPU PGM and
hardware Chrome and native Metal masks in
`tmp/g3s-sunset-lip/graph/production-pcss/receiver-resolve-gpu/`
over `81f55ef2`. Repack the original half guides and decode the
masks independently. Verify the homogeneous displacement algebra,
the fixed 12-bit tolerance and the partial-workgroup controls.
Flag any depth sampling or viewport assumptions for integration.

### M3 opaque winner review brief

Verify `opaque-winner/brief.md`, the shader source hashes, fixture
source and saved tags and depth under
`tmp/g3s-sunset-lip/graph/production-pcss/` over `25ebe50c`.
Require both solo draws to cover a frozen interior with exact
nonzero depth, reverse the tie order, and check that a strictly
nearer draw wins when recorded last. Compare full receiver and
forward planes in both camera entries. Review what this
diagnostic can admit before a game capture.

### M3 offline preservation review brief

Independently compare the four arms in
`tmp/g3s-sunset-lip/graph/production-pcss/receiver-diagnostic/`
over `d5768222`. Verify submitted frame identities and the per-arm
stock and raw guide fits, atlases and receiver depth. Compare both
edge locations, transition widths and clear and umbra values on the
frozen row. Check any raised raw-zero samples and establish what the
offline mask qualifies before allowing the two-pass proof.

### M3 parking review brief

Review the 16:40 CET ruling and parking diff over `282cc039`.
Verify that D1–D3 stay open, M4 stays deferred, the stock graph
ships, and G3b Mac is the first unowned open gate. Confirm the
fresh session's boundary and its distinction between Ground E
and the authored spur.

### M3 scope review brief

Read the M3 boxes, frozen visual witnesses and stock asset and lifecycle
controls over `c0ae64dc`. Determine whether any smaller scope produces
a complete player-visible outcome. Keep the fixed motion, reference
and known showcase defect in the plan if proposing a split. Separate
the shipped valley's terrain lip from the authored spur's in-cascade
flash. No new filter trial.

### M3 frozen resolve motion review brief

Use the raw `frozen-native` and `frozen-chrome` clips and frame logs in
`tmp/g3s-sunset-lip/graph/production-pcss/opaque-winner/` from
`77e84128`. Check the paired frame identities. Examine the original
spur crossing, earlier extinction and the ±20° pan at native
resolution. Track the original step, adjacent frames and any new
flecks, bands, shifted boundaries or persistent grid. Separately
check the guard and winner readbacks in `live-filterable-tie`.

### M3 resolvable caster review brief

Review the footprint and control in
`tmp/g3s-sunset-lip/graph/production-pcss/receiver-diagnostic/positive-control-review.md`
over `b4ec6eac`. Keep the frozen D1 game views and the original
overlap evidence. Require an independently reproducible stock
on/off shadow on equal-depth floor at 160 m, including a partial
wide-filter edge, lit floor and umbra. The on arm must show the
caster in D32. This supplies a preservation control for D2.
M4 retains the original 24 cm finite-disk and caster LOD limit.

### M3 current-frame visibility review brief

Review the exact draft in
`tmp/g3s-sunset-lip/graph/production-pcss/spatial-resolve-review.md`.
Use D1–D3, the frozen frame identities and the native-pixel artifact
evidence. Check the opaque winner, cooked contribution and discard
invariants before judging the filter. Challenge its ability to
remove regular noise while retaining the shallow grazing boundary,
near and far handoffs and compact-caster silhouette. Compare
attachment cost with G3t's planned graph and with the no-history
baseline. Hold shader integration until the diagnostic is reviewed.

### M3 production PCSS review brief

Review `tmp/g3s-sunset-lip/graph/production-shadow-review.md` against
D1's fixed views and native-size motion, and the 226–227, 83–85 and
74–94 archived trajectories. Distinguish the in-cascade flash from
near/last/far transitions. Check that the production filtering change
addresses correlated taps while keeping geometry ownership, costs
and the three-screen stop rule. Require a discriminator for a flash
displaced in space or time or turned into pan shimmer. Keep disk
accuracy recorded separately. Give the verdict before any screen.

**Screen protocol from the review.** Prepare stock 16, fixed 32 and
rotated 32 on one source and cooked generation. The 32-tap arms use
identical call predicates, exits and support. The archived wider
filter changed the near-owned complement and serves as a historical
negative result. Reproduce stock's visible defect before judging the
candidate. Match submitted Sun, camera, interpolation alpha, cooked
generation and render settings frame by frame. A missing affected
frame is inconclusive, including in Chrome.

Use the frozen row-665 receiver pairs at 226–227 and the adjacent
rows across the same span. For the 74–94 trajectory, retain the
frozen receiver list, including x=1000–1039 at y=675–679 for
83–85. Report signed frame differences, extrema and their positions
across the native-size patches. Track position and width of the
visible edge through extinction so a moved flash stays in view.
Compare signed steps against the dense stored-field trend. Keep
the original zero-plateau result and the independent disk error as
separate rows. Watch the entire shipped 1080p shaded crossing and
±20° pan at 1× on both targets, then inspect isolated pan outliers
and adjacent rows at native size for flecks or a displaced band.
Keep map-edge pans, near complement, far fade, compact overlap and
cooked replacement controls. Record path frequency and paired
forward and full-frame costs, and preserve strict discard equality.
A new visible flash, fleck, band or false-light region fails the
screen. A single pixel's disk error is diagnostic.

### M3 early crossing review brief

Derive the discriminator from D1–D3, then inspect the frozen 83–85 screen, source, replay and results in `tmp/g3s-sunset-lip/graph/early-depth/` and the original terrain ray result. Keep its verdict. Check whether the registered step limit measures the desired visible effect. A later fixed trajectory covers entry through extinction, exposes coherent edge displacement and fallback switches, and compares reference trend separately from stock motion. It cannot replace the prior result or the final shaded motion review.

### M3 contour-area review brief

Use the unchanged D1–D3 requirements and `tmp/g3s-sunset-lip/graph/row665-probe/contour-review.md`. Review the tile encoding, centre-domain comparison bounds, disk integration, work caps and frozen row. Give separate saved-field and runtime verdicts. Check that the producer can remain same-frame and that an unsupported row cannot certify light or move a visible discontinuity to a fallback transition.

### M3 production shadow review brief

Over `9d1cabbb`, derive M3's requirements from the cooked crossing, moving camera and named transition controls. Review the plan split and the proposed surface evaluator against the stored-field decode and the original-geometry witnesses. Check that it can distinguish a recovered penumbra from a shifted shadow, that outside-map and hidden-depth cases are explicit, and that the cost cap includes every depth fetch. Walk the surface and aerial owners, discard, recovery and asset replacement. Identify any later consumer that needs the exact triangle union before V1. Give a design verdict and separate conditions for shader integration.

### Frozen source-edge packet review brief

Check M3 D1–D3 against the proposed finite packet screen in
`tmp/g3s-sunset-lip/graph/disk-edge-packet-review.md`. Challenge source
identity, interval and union composition, and whether a passing row
would distinguish this representation from the already rejected work.
Give the decision for the offline test separately from shader admission.

For the continuous follow-up, inspect
`tmp/g3s-sunset-lip/graph/disk-edge-packet-continuous.md` and the scratch
certifier. Check angular, receiver and phase domains and whether the
bound uses the evaluator's original near geometry. The same-ray review also
checks complete residual charge and fixed work limits.

### Bounded disk coverage review brief

On the fixed 1080p route, keep the visible solar limb through the archived
crossing and retain compact blockers. Review the proposed D1 comparison-field
enclosure, continuity, bounded work and graph lifetime independently. Separate
what this can prove from the D2 original-triangle reference. Name any verified
failure, its consequence and the evidence needed before implementation.

### Current-Sun sweep review brief

Use the fixed 1080p crossing, pan, close terrain, thin crest, compact caster and
overlap witnesses. Challenge the continuous strip and angular enclosure, seam
behaviour, terrain/caster union and the work and residency of the producer plus
its complete residual. Identify conditions for one offline proof before any
shader change.

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

### Daylight selector review brief

> Review the candidate over `6a2dbf6` and the raw catalogue/projection bounds
> in `tmp/g3s-star-catalog/daylight-{envelope,grid}.*`. Falsify the footprint,
> transmission, blend, tone, dither and exposure assumptions. Trace graph
> selection, slot and device replacement ownership. Separate setup cost,
> elapsed GPU time and the sampled image comparison.

### Main star graph integration review brief

> Review the merge of `3407173` into `3f18bdf8`. Trace both scene and capture
> graphs, culling counts, slot timings and reconstruction. Inspect the
> two native Metal capture sets and exact browser field/recovery results.
> Identify any change to default content or first and replacement frames.

### Star motion integration review brief

> Inspect the native-size held Metal and Chrome captures and the continuous
> Chrome sequences at `00c2645`. Check night readability, ridge occlusion,
> coherent motion, faint-star phase variation and the twilight transition.
> Keep held Metal images separate from the Chrome compositor stream.

### Normal valley launch review brief

> Review the native and Chrome launch selection, same-build off control,
> benchmark scope and device reconstruction. Inspect the canonical route
> with the old baseline and the held composition with ground E. Separate
> intentional night exposure drift from stars-only changes and parity.

### Production star optics review brief

> Inspect the exact production shader, test fixture and raw Metal rows.
> Check whether flux can pass with wrong placement, depth can pass with
> wrong coordinates, or the SDR contrast can pass with a vanishing star.
> Distinguish supplied-basis placement from the game's time-slider route.

### Reconciled star graph review brief

> Compare the merged graph with both `9d23bdd2` and `9d440836`. Trace day,
> night and capture allocation and pass order with Direct, Frustum and
> Occlusion. Check count copies, timing capacity and slot acceptance, and
> preserve launch defaults and device recovery.

### Canonical star route review brief

> Inspect the six held cooked route stops and the same-build stars-off
> controls in `tmp/g3s-star-catalog/current-main/`. Compare raw frames at
> native size with the saved route, judge night legibility and skyline
> coverage, and separate exposure drift from local star contrast. State
> which motion and device claims the held captures leave open.
