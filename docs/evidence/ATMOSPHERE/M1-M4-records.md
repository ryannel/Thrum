# ATMOSPHERE M1–M4: records from before the loop

M1–M4 were planned and judged in prose before this module had a done-when file. These are their sections as the
module plan held them on 11 November 2026, moved here unchanged when the plans took the orchestrate 1.7.0 shape.
M1 and M2 are done. M3 is a parked spike and M4 a deferred one; their open criteria stay in
[the plan](../../plans/ATMOSPHERE/PLAN.md). Review verdicts are in [the log](../../plans/ATMOSPHERE/LOG.md) and the
briefs in [review briefs](review-briefs.md).

## M1 The valley remains legible at moonless midnight: done

Done when

- [x] On Metal and Chrome, the actual cooked valley at 0 h has a dark nonzero sky and a legible terrain silhouette, with diffuse rock and metal both responding to the same weak environment. Independent review passed both fixed cameras at native scale. [Captures](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] The raw sky zenith and horizontal integral are on the intended dark-site scale. The packed field matches the converged scalar reference within 5% for horizontal irradiance and 10% per sampled zenith, peak, and split-horizon ray at 0, 1.8 and 15 km. Interpolated night in-scatter at 1, 10 and 30 km agrees within 10%. The tolerances qualify the implementation of this optical approximation. Switching off nightglow affects sky, meter, diffuse and reflection together, and restoring it changes no direct sunlight. [Referees and source switch](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] A continuous sunset-to-night sweep has no exposure freeze while light still falls, storage-scale step, or broken shadow transition. Alternating bright and dark submissions across captures, drops, slot reuse and device replacement preserve the correct scale and pixels. A 1% Lambert control survives both devices' real HDR attachments and display paths. Dawn, noon and sunset retain their checked images. [Motion, held captures and storage evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] Pair whole-frame and pass cost with the old run and investigate tripwires. Workspace check and real window and browser motion runs pass independent visual review. [Paired costs, remaining tripwire and review](../v1-workload/G3S-HANDOFF.md#current-checkpoint).

Work

- [x] Reproduced a completely black midnight frame on the actual cooked valley at a91cdd0e. The scalar range check confirms the fixed fp16 scale loses a 10% Lambert surface.
- [x] The independently stepped scalar direct-plus-scatter probe resolves the positive-elevation peak and both limits of the curved horizon at 0, 1.8 and 15 km. The denser quadrature changes the sampled radiance by at most 0.063%.
- [x] A second scalar reference includes the finite shell, both components, the engine phase function, ground irradiance and finite paths. Raw source and rows are in `tmp/g3s-showcase/night-design/`.
- [x] The scratch RG16Float profile matches the dense reference on 59 distinct sky/ground rays, six irradiance rows and three finite paths. Maximum errors are 0.381%, 0.028% and 0.060% respectively. The expanded sweep includes 2 m and 6 m near-tangent rays. These are packed CPU reads.
- [x] Wire the night channel into sky, diffuse, reflection and finite aerial transport. Connect the per-view exposure sample to all forward colour writers. The GPU meter test reaches −12 EV100 and preserves a low-light arithmetic control.
- [x] Compare the production Metal sky, incident reduction and finite aerial decoder to the f64 reference. Toggle the source through the existing graph and check that its dependent caches refresh. [Observations](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] A 1% Lambert-equivalent radiance control survives the actual HDR and SDR attachments on Metal and Chrome. Metal covers dropped encodes, captures and both reused exposure sides. Paused midnight pixels survive real GPU replacement on both targets.
- [x] Compare held nightglow-off and on scenes, including the specular response. Both targets go from black to readable at midnight and noon differs by at most one RGB code.
- [x] Capture and inspect actual valley sequences. Review the raw data and record results.

## M2 Stars emerge as the observer turns and time advances: done

Done when

- [x] **D1.** From the same valley views, star appearance follows the dimming sky; star positions match the chosen observer rotation, stay stable under slow pans and are occluded by terrain. Hold bright day, late twilight and moonless night; watch motion on Mac and Chrome. [Motion evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] **D2.** At the fixed showcase camera, midnight reads unmistakably as a clear night while the terrain silhouette stays legible. Judge the unlabelled shipping field at native size before seeing the late twilight and M1 controls. Compare stars on and off at the same pose, exposure and display settings to see what the stars contribute. Review a slow pan across the skyline on Mac and Chrome. [Matched views](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] **D3.** Integrated test-star flux stays within 5% across subpixel position and 720p/1080p/2160p. At matched night exposure, magnitude 6 is dimmer than magnitude 3 by a visible margin in the resolved footprints. Verify the rotation with known directions, including the time-slider wrap. A star centred behind the skyline leaves no sprite leak. Validate at image centre and edge. [Metal and wrap recheck](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] **D4.** The catalogue provenance is complete, paired runs record frame cost and logical attachment area, and the workspace and independent in-game review pass. Physical bandwidth is tracked separately below. [Costs and scope](../v1-workload/G3S-HANDOFF.md#current-checkpoint).

Work

- [x] D4. Audit and pack 6,632 original 1908 Table I stars, preserving the source rows and checksums.
- [x] D1–D2. Run the normal valley launch and same-build controls on Metal and Chrome. Review held E composition and the continuous Chrome stream. [Evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Decode and inspect both 1× Metal offscreen sequences for pan and emergence, with submitted-frame GPU timings. [Evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Play both Metal 1× sequences through and inspect the moving images. The all-frame decode remains the native-size masking and stability evidence. [Playback](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D3. Qualify the candidate's production Metal flux, centroid, two-axis depth masking and central SDR contrast. Exercise both Chrome slider endpoints and adjacent night settings. [Evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D4. Recheck the admitted route and record paired Metal costs and logical attachment area. [Raw-run summary](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D4. Record paired Chrome full-frame costs at midnight and twilight. Recheck the full evidence independently.
- [x] D1–D3. Requalify the corrected sky with named catalogue stars, the slider wrap and matched Metal/Chrome night and pan captures. [Requalification](../v1-workload/G3S-HANDOFF.md#current-checkpoint).

Apple tile M0.4 replayed the matched offscreen star traces in Xcode on the M3 Max. The single pair shows 46.20 to 48.63 MiB of device-memory traffic, with about ±1 MiB per encoder uncertainty. No time comparison comes from the replay. [Counter evidence](../render/G3T-TRAFFIC-PROBE-2026-11-08.md).

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
Start with the [stock control and motion handoff](../v1-workload/G3S-HANDOFF.md#current-checkpoint)
at `282cc039`. The full return criteria below remain open.

Done when

- [ ] **D1.** At the west lookout on the cooked valley, the near ground and middle distance read as continuous terrain during the 17.40–17.80 h crossing and a slow ±20° pan on Metal and Chrome. Independent side-by-side motion review matches the reference’s continuous moving edge. The timed 1× replay shows no newly introduced flecks, ribbing or detached bright bands. The fixed 1080p spur witness removes the known row-665 in-cascade flash at frames 226–227, and the six-stop route and both controls pass.
- [ ] **D2.** Compare against current main at the same receiver, source and Sun. The open Sun and full occlusion controls retain their expected result. The broad edge, close terrain, distant crest, represented compact-caster control and overlap exhibit no new visible false light or shadow across near/last/far range, camera and azimuth sweeps, or a cooked asset swap. Review full-resolution visibility and shaded images, with signed difference masks registered by receiver and time. Report new lit/umbra regions separately from the existing errors and compare asset replacement with a fresh run of the replacement. Record signed disk-reference error on the fixed witnesses as a measured open item. The original sub-texel caster stays in M4.
- [ ] **D3.** The regular graph preserves aerial and surface ownership, recovery and strict discarded-frame image equality. Record paired whole-frame and shadow costs, and pass workspace and independent code and visual reviews on the integrated result.

Work

- [x] D1. Freeze the 1080p crossing and 1× pan, route and controls, plus the exact D32 inputs for frames 226–227. The failing receivers are inside the last cascade in both frames. [Evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Review and screen contour-area PCSS on the eight saved D32 rows. Preserve the literal failure. The revised merged-interval preflight recovers the limb within the original work caps. [Evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Replay 83–85 byte for byte and capture its D32 fields. Preserve the failed transition screen and its stock-plateau diagnosis. [Evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Run the separately fixed extinction trajectory. The bounded query rejects its fallback on entry. [Evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D3. Review the production PCSS combination against the fixed trajectories and the composed graph. The independent review conditionally cleared one screen with the protocol below. [Brief](review-briefs.md#m3-production-pcss-review-brief).
- [x] D1–D3. Freeze the crossing and pan inputs and prepare the stock, fixed and rotated builds. Reproduce the stock crossing on Metal. [Baseline](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D3. Run the first matched production PCSS screen. Both targets reproduce the original flash and reject rotated-32 for regular penumbra crosshatching. [Results](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D3. Review same-frame receiver-guided filtering and its cost. Reviewers clear one raw diagnostic and hold the live graph.
- [x] D1–D2. Record the frozen Metal receiver guides and evaluate the fixed radius-one filter. The visibility mask is smoother and the strongest row-averaged edge stays in place. The authored compact and wider control are silent in this evaluator. [Diagnostic](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D2. Trace the existing wide-caster comparator through the current fitted D32 atlas. Near clips it, last keeps a closer surface at its two projected centres, and the height layer contains no covered texel centre. Keep the historical split-map result scoped to that renderer.
- [x] D2. Freeze and qualify the separate stock-positive caster at 160 m. The original projects to 0.20 × 0.47 far texels, so retain it in M4. The new control wins 25 D32 samples and shows wide-filter edge, lit and umbra rows on the same receiver depth. [Evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D2. Prove the fixed 3×3 offline filter preserves the frozen stock-positive shadow profile and clean regions without broader support or new guides. [Evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Establish the same coincident synthetic/cooked winner in isolated Metal pipelines. The two solo draws, both tie orders and a distinct-depth control pass with the production vertex entries and cooked Uint32 multidraw.
- [x] D1–D2. Verify the fixed GPU filter against the CPU oracle on the saved caster, crossing and extinction regions. Both backends pass without widening the filter or changing guard thresholds. [Evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D1–D2. Prove exact live cooked/synthetic D32 ties and winner identity on Metal and Chrome, with reversed order and later-nearer controls. Both forced ±2 ULP arms render stock inside the fixed crop and the resolved image outside.
- [x] D1–D2. Compare live mask acceptance in both draw orders on 235 filterable 160 m ties in Metal and Chrome. Their raw tied centres are zero. The shaded edge and overlap still need independent controls for any future graph.
- [x] D1–D3. Screen the optional live resolve on the three frozen 1080p/60 trajectories and retain the failed result. [Evidence](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [x] D2–D3. Establish stock baselines for the low-Sun Chrome cooked swap, native discarded-frame image equality and device restaging, and Chrome valley recovery. [Baseline](../v1-workload/G3S-HANDOFF.md#current-checkpoint).
- [ ] D1–D3. Resolve cooked and tile ownership before a live candidate. Keep the submitted-slot history rules and three-screen stop limit.
- [ ] D1–D3. Integrate one shared surface evaluator, then capture the shipped ground and spur replays, range and cooked replacement controls, and paired frame costs. Review both targets in motion before promotion.

## M4 Exact finite-disk shadow reference match: deferred bounded spike

Done when

- [ ] Compare one reviewed representation with the original-geometry disk reference at the broad edge, distant narrow crest including moving receivers, the 64 m close terrain exclusion, the 24 cm compact caster at 160 m and the overlap. Keep the open Sun control and one coverage union. Use the frozen raised, distant, broad and close 4 m patches and the four row-665 receiver/phase domains at x=480, 720, 960 and 1200. Keep the crest-only ablation, close umbra, compact-only and overlap controls. Retain a 0.5 percentage-point continuous omission bound. Prove the retained blocked set is a subset of the reference or separately bound overcoverage to the same tolerance. Freeze finite sample and reference integration tolerances before the run.
- [ ] Publish the verdict and the complete source, receiver and caster cost for the measured domain. Only a passing candidate can propose a separate in-game landing.

Work

- [x] Preserve all fixed fields, receiver paths, phases, source hashes, triangle/ray references, and failed variants. [Witness inventory and reruns](../v1-workload/G3S-HANDOFF.md#current-checkpoint). The prior work-item list is retained at `9d1cabbb`.
- [ ] Review at most one representation with a distinct source-side reuse mechanism and explicit producer, receiver and caster work caps fixed before running. Run it once against the frozen witnesses. Park it if it exceeds the cap or fails admission, retain the failure and stop. M3 and other V1 gates can proceed independently.
