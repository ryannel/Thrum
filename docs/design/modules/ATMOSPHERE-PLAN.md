# Atmosphere milestones

Design: [ATMOSPHERE.md](ATMOSPHERE.md)
Reviews: Optical design and plan over `a91cdd0e`, final design `f9d0c12a` and plan `c3cc5748` · [optical brief](#optical-review-brief) · initial changes required, final M1 implementation cleared. Frame graph and ownership at the same revisions · [graph brief](#graph-review-brief) · initial changes required, final design cleared. First valley captures, Metal build `dfae5b21` and Chrome Wasm `bc15ea7d` (SHA-256 prefixes) · [capture brief](#capture-review-brief) · held ground view passes, M1 remains open. Chrome late-twilight Wasm `134e271c` · [sweep brief](#sweep-review-brief) · silhouette and reflection visible, warm contour and live motion open. Packed reflection candidate over `c2ac94d8`, Wasm `132f992f` · [reflection brief](#reflection-review-brief) · visual improvement, rejected on measured Metal cost. Four-lane candidate over `4fecd095`, Metal `c6e7b102` and Chrome `4afa8b38` · [four-lane brief](#four-lane-review-brief) · held visuals pass with faint residual streaks; no code findings; live cadence open. Live captures over `16dc9da2` · [follow-up brief](#four-lane-review-brief) · late-twilight progression passes; early object shadow pop verified on both targets; physical cadence open. Shadow partition over `9f13c5f6`, Metal `510e62ce` and Chrome `c6500ef7` · [shadow brief](#shadow-review-brief) · fixed three ownership and coverage gaps; matched visual checks pass. SDR comparison over `1f4e6932`, Metal `41d11c1d` and `1baf73d5` · [tone-scale brief](#tone-scale-review-brief) · keep Neutral Highlights. Local ground material over `ec78a0d7`, Chrome `8d832924` · [ground brief](#ground-material-review-brief) · selected, existing lookout lip remains conspicuous. M1 closeout source `b8f230e7`, Chrome `46d3eb04` and Metal `4928bca0` · [closeout brief](#m1-closeout-review-brief) · sampled crossing and exposure pass on both targets, including the completed foreground Metal sweep through midnight. The callback-tail tripwire stays open. M2 look criterion over `590cee61` · [midnight look brief](#midnight-look-review-brief) · added an unlabelled shipping-frame witness and a same-build star toggle. M2 integration over `6fa34365`, corrected WIP `1fa3228` · [star integration brief](#star-integration-review-brief) · camera, opt-in and altitude findings fixed on recheck. Tight footprint over `a61bb80`, Chrome `568edbaa` · [tight footprint brief](#tight-footprint-review-brief) · compact held stars and sampled tracking pass. Required a positive central night witness. Live faint-star stability remains open. Harvard source pack `3a3536c7` and opt-in valley `98e0be1`, Metal `a7588eed` and Chrome `ced775e0` · [source and valley brief](#catalogue-and-valley-review-brief) · original values reproduced, sampled midnight, occlusion, pan and emergence pass. Late sky rebound and live cadence remained open. Twilight shoulder WIP `6a2dbf6`, Metal `c4700ab7` and Chrome `80cd702c` · [shoulder brief](#twilight-shoulder-review-brief) · sampled look passes for the checkpoint, with a small residual rise in one sky patch. The inline twilight cost overlaps the baseline. Daylight selector WIP `3407173`, Metal `564cccbc` and Chrome `4109e560` · [daylight brief](#daylight-selector-review-brief) · bound and held image comparisons pass; live motion and production integration remained open. Main integration `00c2645`, Metal `0acd58ff` and Chrome `fcc21034` · [integration brief](#main-star-graph-integration-review-brief) · cross-graph ownership passes and default twilight changes are isolated on the integration branch. Continuous Chrome build `fcc21034` · [motion brief](#star-motion-integration-review-brief) · tracked stars persist through the pan, with measurable core brightness variation. Native window motion remains open. Normal valley `197d152`, Metal `1384a38a` and Chrome `b870c1ee` · [normal launch brief](#normal-valley-launch-review-brief) · same-build controls and the held composition with G7 ground E pass, with the expected old midnight baseline drift recorded. Production star optics over `197d152`, revised fixture SHA-256 prefix `aacba07b` · [optics brief](#production-star-optics-review-brief) · centroid and two-axis depth fixes cleared on recheck. The remaining one-code contrast allowance was raised to eight. The Chrome slider witness supplies game continuity separately. Reconciled star graph `d2a5342` · [merge review](#reconciled-star-graph-review-brief) · no merge-induced findings. The final Metal test exercises timed Frustum and Occlusion capture. Canonical valley Metal `c5c5b84a` and Chrome `b497346f` · [held route review](#canonical-star-route-review-brief) · clear night and same-build controls pass. Midnight refreshed at the unchanged route tolerance. Metal emergence build `7ecc56cc` · [offscreen brief](#offscreen-emergence-review-brief) · timing and encoded frame pairing pass. Held and decoded frames show emergence. Continuous playback remains open. Ground E pan build `313d423c` and held route `check-20261107-230802` · [pan brief](#ground-e-pan-review-brief) · decoded pan and terrain masking pass. Midnight-only baseline approved by review. Sunset prototype build `bc237e22` · [sunset brief](#sunset-horizon-review-brief) · foreground depth edge and thin stripe improve, shadow pop still visible. Metal builds `7ecc56cc` and `313d423c` · [all-frame decode brief](#independent-all-frame-decode-brief) · held night and decoded emergence and pan pass, with a one-code late sky lift. Playback remains open. The authored spur on clean `ba45ff60` · [paired route brief](#authored-spur-review-brief) · held route and controls pass; source discontinuity, flecks and crossing band hold promotion. The vertex-baked far-horizon Metal proof over `ba45ff60` · [far-horizon brief](#far-horizon-review-brief) · fixed-camera band removed; caster union and baked eligibility hold promotion. M2 review on Metal builds `313d423c` and `7ecc56cc`, Chrome Wasm `e64183ed` and timing revision `59e76f3d` · [M2 closeout brief](#m2-closeout-review-brief) · D1–D3 pass, with timing and bandwidth outstanding at that revision. D4 over `172ce6fe`, Chrome Wasm `429ade00` and Metal pan build `313d423c` · [timing brief](#m2-timing-review-brief) · scope and evidence pass with wording changes. Scene-only Metal rows and physical counter follow-up are distinct from full-frame timestamps. M3 first design draft over `007e8f88` · [coverage brief](#m3-coverage-design-review-brief) · hold runtime. Aperture union sound; closure, precision, capacity and publication need explicit proof. M3 edge support follow-up over `79b726ec` · [edge brief](#m3-edge-support-review-brief) · real-arithmetic pruning passes. Closure, portable precision and capacity remain open. M3 receiver-face proposal over `290d53da` · [face brief](#m3-receiver-face-review-brief) · dominance proof passes with fixed world sectors and an enclosed in-footprint origin. Packed capacity and caster work remain open. M3 directional reuse over `c7937efb` · [sweep brief](#m3-directional-reuse-brief) · advance source-side hull sharing to offline capacity proof. Fixed-line samples do not certify a sector. M3 coverage split over `4461bff6` and the block-join/caster census · [split brief](#m3-terrain-coverage-split-brief) · residual composition is sound. Decoded coverage, capacity and complete caster admission need proof. M3 fit witness over `d063f2e8` · [fit brief](#m3-decoded-fit-brief) · broad holdout passes. Refine the raised crest and independently bound the terrain cut used by the residual. M3 fit stop over `9e811aa2` · [roof brief](#m3-terrain-roof-brief) · select nested terrain bounds and original geometry for an offline cost screen. Hold runtime. Ray reference over `9e811aa2` · [ray brief](#m3-ray-reference-brief) · geometry and Metal tolerance accepted. Added coincident and scaled controls for union and intersection precision. Projected envelope over the same checkpoint · [projection brief](#m3-projected-envelope-brief) · proceed to one paired CPU proof, with near clipping and above-cap terrain coverage kept. Disk clipping over the raised-crest projection screen · [disk reference brief](#m3-disk-reference-brief) · CPU reference candidate accepted with finite window and apex conditions. Runtime capacity remains open. M2 requalification at `9c1021bb`, Metal `56948ba5`, Chrome `f50141b4` · [orientation brief](#m2-orientation-review-brief) · D1–D3 pass at the captured route’s scope. Physical cadence remains separate. Bounded disk coverage over `846c4fba`, initial diff `4ce8cc61` and revised dyadic mechanism · [review brief](#bounded-disk-coverage-review-brief) · numerical and work bounds required, revised offline evaluator cleared, GPU integration held. Current-Sun strip sweep over `360f93c1`, initial diff `90481778` and revised endpoint/transition proof · [review brief](#current-sun-sweep-review-brief) · one bounded offline proof cleared; raw-band rejection independently checked, disk-aware representation remains open. Frozen source-edge packet draft `bfbbdd36` (SHA-256 prefix) · [packet brief](#frozen-source-edge-packet-review-brief) · revised finite screen cleared. Continuous omission bound over `524eb86f` · [packet brief](#frozen-source-edge-packet-review-brief) · independent-q refinement inconclusive at the distant crest; fixed near partition and same-ray screen over `86750ebd` · [packet brief](#frozen-source-edge-packet-review-brief) · bounded residual inconclusive at all eight domains; stop packet trials. M3 rescope and first SMRT draft over `9d1cabbb` · [production brief](#m3-production-shadow-review-brief) · scope split approved. Reject the binary ray screen on quantization and hold shader work for a defined fractional resolve, field support and composed cost. M3 contour-area design over `000ffcdb`, drafts `073e32d9` and `ece6c04a` (SHA-256 prefixes) · [contour brief](#m3-contour-area-review-brief) · literal query fails the expansion cap. Merged-interval preflight cleared and passes the saved-field screen. Shader admission held. Earlier crossing archive over `50d33835`, screen `ce431c37` and log `543b1893` (SHA-256 prefixes) · [trajectory brief](#m3-early-crossing-review-brief) · frozen check fails on stock's zero-motion plateau. Preserve the failure and use a separately fixed extinction and handoff witness. Production PCSS over `268f8a93`, draft SHA-256 prefix `8bce8a15` · [review brief](#m3-production-pcss-review-brief) · conditionally clear one game screen once the matched arms and full-frame motion witnesses below are frozen. No promotion. Matched PCSS screen over `efa82534` · [review brief](#m3-production-pcss-review-brief) · reject rotated-32 on native-size Metal and Chrome crossing. The original coherent step is smaller and new crosshatching is visible. Current-frame resolve over `8a4cdfbd`, draft SHA-256 prefix `6d559395` · [resolve brief](#m3-current-frame-visibility-review-brief) · independent and Apple tile reviews clear raw receiver diagnostics only. Require exact tied-winner, compact-shadow and variant traffic proofs before integration. Cooked seam review pending. Raw visibility diagnostic over `5984ca51` · [resolve brief](#m3-current-frame-visibility-review-brief) · grain reduction confirmed on the fixed Metal record. Compact caster and independent opaque-winner control remain open. Hold integration. Caster atlas trace over `bded4d69` · [resolve brief](#m3-current-frame-visibility-review-brief) · independent projection matches both blocking depths to the original terrain. Retain the upstream miss and hold the live resolve. Caster scope and stock control over `b4ec6eac` · [control brief](#m3-resolvable-caster-review-brief) · keep D1 and the historical overlap. Approve one separately authored stock qualification at 160 m, with an observed wide-filter edge and clean controls.
Updated: 2026-11-08
Milestone: M3 sunset shadows
Now: the original 24 cm caster is sub-texel in the last map. The separate 160 m stock control shows a resolved penumbra.
Next: test preservation on the frozen positive control, then review the independent opaque-winner proof.
Blocked: the live resolve still needs the preservation and two-pass opaque-winner checks. Physical bandwidth replay still needs an assigned Xcode slot.

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

Queued separately by the overseer on 8 November: replay the matched offscreen Metal traces in Xcode and inspect physical attachment traffic with [Apple tile M0.4](APPLE-TILE-PLAN.md#m0-proof-route-counters-and-capture-probe-pending). Screen replay does not gate the star closeout. Lunar illumination, atmosphere calibration and the M3 shadow outcome remain separate.

## M3 The west lookout’s shadow crosses the ground without a flash: in progress

Done when

- [ ] **D1.** At the west lookout on the cooked valley, the near ground and middle distance read as continuous terrain during the 17.40–17.80 h crossing and a slow ±20° pan on Metal and Chrome. The timed 1× replay shows no newly introduced flecks or detached bright bands. The fixed 1080p spur witness removes the known row-665 cascade step at frames 226–227, and the six-stop route and both controls pass.
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
- [ ] D1–D2. Prove the fixed filter preserves the stock-positive shadow profile and clean regions. Keep the original overlap comparator, and establish the coincident synthetic/cooked winner separately. Reject if the fixed filter requires broader support or new guides to pass.
- [ ] D1–D3. Resolve cooked and tile ownership before a live candidate. Keep the submitted-slot history rules and three-screen stop limit.
- [ ] D1–D3. Integrate one shared surface evaluator, then capture the shipped ground and spur replays, range and cooked replacement controls, and paired frame costs. Review both targets in motion before promotion.

## M4 Exact finite-disk shadow reference match: deferred bounded spike

Done when

- [ ] Compare one reviewed representation with the original-geometry disk reference at the broad edge, distant narrow crest including moving receivers, the 64 m close terrain exclusion, the 24 cm compact caster at 160 m and the overlap. Keep the open Sun control and one coverage union. Use the frozen raised, distant, broad and close 4 m patches and the four row-665 receiver/phase domains at x=480, 720, 960 and 1200. Keep the crest-only ablation, close umbra, compact-only and overlap controls. Retain a 0.5 percentage-point continuous omission bound. Prove the retained blocked set is a subset of the reference or separately bound overcoverage to the same tolerance. Freeze finite sample and reference integration tolerances before the run.
- [ ] Publish the verdict and the complete source, receiver and caster cost for the measured domain. Only a passing candidate can propose a separate in-game landing.

Work

- [x] Preserve all fixed fields, receiver paths, phases, source hashes, triangle/ray references, and failed variants. [Witness inventory and reruns](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint). The prior work-item list is retained at `9d1cabbb`.
- [ ] Review at most one representation with a distinct source-side reuse mechanism and explicit producer, receiver and caster work caps fixed before running. Run it once against the frozen witnesses. Park it if it exceeds the cap or fails admission, retain the failure and stop. M3 and other V1 gates can proceed independently.

## Review briefs

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

## Log

- 8 November, 13:05 CET steer. Exact matching consumed roughly five hours with no M3 done-when closed. Move that admission and its witnesses to M4. Keep M3's visible no-flash crossing, transition controls and frame integrity. The exact union is an offline referee for M3. No later V1 consumer is known to require it: aerial visibility has its own interval contract, and tile, reconstruction and the showcase consume the resolved image. Independent review agreed. It rejected the first fixed binary-ray adaptation before a run and held shader integration.
- 8 November contour follow-up. The literal four-contour cap rejects all eight rows. The independently rechecked preflight resolves uniform tiles first, with every attempt charged to the unchanged cap. All eight pass the D32 screen. This establishes fractional support on the stored field only.
- 8 November extinction check. The matched 21-frame trajectory rejects the whole-receiver fallback. Stop after three failed screens and retain the fractional-area results. The visual cue is foundational for M3. The next design must localize missing support rather than raise the contour cap.
- 8 November, 13:42 CET steer. Review shipped shadow filtering and stable cascade handoffs before designing a new coverage model. Select a fixed-phase-per-pixel directional PCSS adaptation for independent review. The existing transitions remain witnesses. The independent review confirmed the in-map sampling defect and required a matched fixed-phase arm and complete motion capture. No renderer change or new screen ran.

- The 8 November daily review found an orthogonal reflection in the game basis. Reopen M2 D1 and D3. Choose a right-handed local frame at −45° to keep the authored Sun path while correcting star orientation. M3 design proceeds alongside the recheck.

- M3 remains foundational. The uniform, adaptive-time and conforming fits miss the raised crest. The conforming screen keeps its seams but exhausts the next refinement within its diagnostic ceiling. Park pure fit trials and review an original-geometry complement. Raw rows and the method decision are in `tmp/g3s-sunset-lip/M3-RESEARCH.md`.

- 2026-11-07 — started the moonless-night milestones after the cubic LUT probe produced no appreciable SDR improvement.
- 2026-11-07 — the owner accepted M1 and added the midnight look check to M2 after inspecting the foreground sweep.
- 2026-11-08 — parked the sunset-shadow trials after repeated failures. They do not block the star milestone. M3 owns the separate shadow outcome and retains the witness and review evidence. Apply the revised engine-module-design stop-and-decide rule before restarting.
- 2026-11-08 — M3 treats coverage composition as foundational. `tmp/g3s-sunset-lip/M3-RESEARCH.md` records the published alternatives, the archive population check and the shared-aperture choice. A clear/full-only fast path cannot cover the difficult partial frame.
- 2026-11-08 — one-aperture Ground E sizing rejects the flat layout. Use pairwise slope bounds on the receiver face, with sparse angular refinement. The research note records why this is a different certificate and what still needs proof.
- 2026-11-08 — the independent review cleared one bounded disk-cell screen. It exhausted the read cap on every archived receiver. Park fragment-local integration and keep the crossing foundational. `tmp/g3s-sunset-lip/M3-RESEARCH.md` records the design, review and raw rows.
- 2026-11-08 — after the independent sunset brief, select a shared Sun-aligned strip sweep for review. The research note gives the band bounds, cost ceiling and fixed witness. SMRT, SDF cone minima and temporal history do not supply the required current-frame coverage.
- 2026-11-08 — park the raw-extrema sweep after the fixed entrance proof. The crossing stays foundational. `tmp/g3s-sunset-lip/M3-RESEARCH.md` records the required within-band support and the independent verdict.

### Offscreen emergence review brief

> Independently inspect the game capture's frame submission, clock and
> readback in `bin/window/{capture,motion}.rs`. Check every video frame
> against `offscreen-emergence/capture/frames.jsonl` for a tick and
> valid GPU time. Judge the fixed valley view during star emergence
> against D1 and D2. Distinguish decoded frames, continuous playback
> and camera motion, and report visible defects without pacing claims.

### Ground E pan review brief

> Compare the twelve held frames from `check-20261107-230802` with
> ground E's route baseline at `8f1dcf76`. Review both 600-frame
> lossless clips in `ground-e-night-pan{,-off}` for clear-night read,
> bright and faint star stability, and terrain masking. Verify paired
> ticks, pose and timings. Distinguish sampled and decoded review
> from playback. Check the pan uses the game's camera control.

### Sunset horizon review brief

> Inspect four matched 1080p60 clips in `tmp/g3s-star-catalog/current-main/`
> for current and prototype shadows on Ground E and the connected spur.
> Separate the terrain and shadow effects. Identify visible discontinuities
> and give frame indices and timing provenance.

### Independent all-frame decode brief

> Inspect the raw Metal emergence and matched midnight pans from the
> offscreen and Ground E briefs. Judge D1/D2 at native size, including
> temporal coherence, skyline masking and the clear-night read.
> Check video timestamps and submitted GPU samples. Separate actual
> playback, decoded motion evidence and held frames. Read only.

### Authored spur review brief

> Compare both six-stop `ba45ff60` route runs and the 300-frame Metal
> pair under `tmp/g3s-sunset-lip/`. Judge the west depth edge, the
> moving solar shadow, the ridge and other cameras at native scale.
> Inspect the two sources for geometric discontinuities and check
> the stock route and control results. Distinguish decoded samples
> and measured frame changes from physical presentation. Read only.


### Far-horizon review brief

> Review the baked far-horizon game comparison over `ba45ff60`,
> patch, CPU rows and interleaved timings under `tmp/g3s-sunset-lip/`.
> Judge the visible edge and the scope of the bypass. Check near and
> far composition, mesh identity, azimuth wrap and measured pass identity.
> Read only. Distinguish decoded samples from playback.

### M2 closeout review brief

> Independently judge D1–D4 on the shipped Ground E valley. Inspect the
> Metal pan, stars-off control and emergence records, the current Chrome
> motion and six-stop route, catalogue provenance, optical fixtures, and
> raw on/off timing rows. Separate visual samples from playback and
> distinguish logical attachment area from measured traffic. Read only.

### M2 orientation review brief

> Requalify D1–D3 independently on `9c1021bb`. Inspect the
> exact-build Metal and Chrome clips, independent named-star
> pixels, matched midnight and slider-wrap controls, and the
> production Metal optics log. Judge the actual route and
> distinguish captured motion from physical presentation.

### M2 timing review brief

> Audit the M2 D4 evidence independently, read-only. Check eight paired
> Chrome arms on Ground E, exact browser build and preset, both paired
> Metal sources, render CPU and full-frame timing scope, and the visible
> performance tripwire. Check attachment formats and graph operations
> against the logical byte totals. Reproduce the packed catalogue from
> the audited rows and verify its checksums. Inspect the saved trace
> inputs and distinguish deferred physical counters from graph-declared
> bytes. Judge what the numbers support without an attributed speedup
> or physical display claim. No new build or GPU run.

### M3 disk reference brief

> Check the finite Sun disk projection and Jacobian bound against
> the 289 raised-crest receivers and paired logs in
> `tmp/g3s-sunset-lip/M3-RESEARCH.md`. Challenge geometric coverage,
> apex and clipping semantics, event capacity and continuity.
> Rule separately on the CPU reference and runtime admission. Read only.

### M3 coverage design review brief

> Read-only independent review over `007e8f88` and the first M3 draft.
> Derive the required outcome from M3 and challenge the spherical
> union, terrain closure, thin caster, work bounds, Chrome limits and
> edit or exhaustion lifecycle. Read the archive population and prior
> failure witnesses in `tmp/g3s-sunset-lip/M3-RESEARCH.md` and
> `tmp/g3s-sunset-lip/NOTES.md`. Evaluate the next design decision and
> any established cheaper method. No new GPU trial.

### M3 edge support review brief

> Check the prepared original-edge atlas against the upper-envelope
> proof and the raw 1080p archive rows. Challenge whole-triangle
> partitioning, grazing and collinear hits, the terrain prefix and
> portable floating-point accuracy. State what the CPU witnesses prove
> and which work remains before a runtime change. No GPU or build.

### M3 receiver face review brief

> Review the design diff over `290d53da` against the full-source CPU census in
> `tmp/g3s-sunset-lip/M3-RESEARCH.md`. Challenge the receiver-face Bernstein
> certificate, origin and terrain closure, shared-list capacity and independent
> caster admission. Identify the smallest offline proof before runtime work.
> No GPU run or build.

### M3 directional reuse brief

> Review `c7937efb` and the raw face-sector census in
> `tmp/g3s-sunset-lip/M3-RESEARCH.md`. Find an established mechanism to
> share terrain work across continuous receivers. Account for Sun aperture
> bounds and compact-caster union. Return one recommendation and a
> falsifier before runtime work. Read only.

### M3 terrain coverage split brief

> Review the block-join and caster-cone census over `4461bff6`. Challenge
> prepared terrain-only coverage with common-aperture correction on
> detached overlap, continuity in space and time, edit ownership and
> caster completeness. Keep D1–D3 unchanged. Read only.

### M3 decoded fit brief

> Review the uniform and adaptive-time fit logs and detached residual
> over `d063f2e8` in `tmp/g3s-sunset-lip/M3-RESEARCH.md`. Challenge what
> the holdouts and detached-ray check establish and whether the spatial
> mechanism warrants a continuous proof. Read only.

### M3 terrain roof brief

> Review the conforming fit and independent union over `9e811aa2`.
> Compare an exact collar to nested terrain solids and original
> residuals. Recommend one work/ownership contract and a falsifier.
> Preserve D1–D3. No fit trials, builds or GPU work.

### M3 ray reference brief

> Check the 3D ray reference and its use in the unchanged Metal
> plateau fixture over `9e811aa2`. Challenge weighting, overlap
> controls and recorded per-column roof counts. Read only.

### M3 projected envelope brief

> Review the projected edge-envelope proposal and the prior shared
> beam in `tmp/g3s-sunset-lip/M3-RESEARCH.md`. Challenge support
> clipping, terrain fill, the cap and the paired work falsifier.
> Keep runtime integration held. Read only.

- 8 November, 14:12 CET. The matched 32-tap rotation cuts the coherent step but introduces visible crosshatching in the west-spur penumbra on Metal and Chrome. This is the first failure on the reviewed production PCSS comparison. Keep the shipped filter, matched drivers and all witnesses. The visible cue remains foundational. No seed or radius tuning. Review a resolve before another screen.

- 8 November, 14:18 CET. Select a current-frame shadow-mask resolve as a diagnostic. The independent review confirms the extra raster and packed guide design can be evaluated without changing the opaque winner. It flags compact-caster smear and coincident opaque ties. Apple tile review keeps three stored transients confined to the optional variant. No shared graph change or new game screen is cleared.

- 8 November, 14:28 CET. The frozen receiver check reduces grain without moving the strongest strip-average edge, but several raw zeros become positive. That is unresolved recovery versus false light. Both authored caster arms match the no-caster D32 receiver and guide bytes, so they cannot clear compact-shadow preservation. Hold integration and keep the next check on the existing positive witness.

- 8 November, 14:38 CET. The wide caster covers two last-map centres but both already hold closer depths. All three fitted layers are byte-identical with and without the caster. This explains the silent witness without a new candidate screen. The no-attachment quad estimate leaves substantially more regular grain than the 3×3 resolve and stays parked. D1 and D2 stay open.

- 8 November, 14:38 CET steer. Measure the exact compact fixture against the cascade that can receive it. It spans 0.20 × 0.47 far texels and is clipped by the near map. Keep it in M4. The reviewed stock-only 160 m replacement control shows a positive wide-filter penumbra. It adds a preservation check and leaves every D1 game view and the earlier overlap witness in place.
