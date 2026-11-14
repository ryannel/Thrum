# ATMOSPHERE plan

Closes gate G3s at M6; M7 and M8 are track 2. [Design](DESIGN.md) · [done-when file](DONE.json), which holds M5 to M8 · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

## Open for the owner

- Assumption: readiness work for the sea and cloud port (M5) proceeds now, ahead of the owner's decision on when the port itself starts; the port decision says to keep the foundation converging on readiness.
- Assumption: G3s closes with the west-lookout crossing (M3) still parked and named as a known defect, and with M4 deferred; neither is judged in M6.
- Assumption: the sky's closing review waits for TAAU, so it is judged once on the image the exit will show.
- Assumption: solar sky radiance and solar aerial transport stay unqualified against an all-orders reference when G3s closes. M5 and M6 prove that the lent lookups return the stored products in the right units and that the existing bounded referees still pass; the ledger names the gap. A full accuracy campaign would be a separate plan.
- Assumption: the lent sky and aerial lookups stay one hardware-filtered read, and their handoff bound allows one stored half-float step (at most 0.1% of the value), because Metal returns filtered half-float reads at that precision. Blending exact texels in the shader for float32 results was rejected: it costs every sky pixel and lit fragment more reads and changes every scene's image for no visible gain.
- Assumption: "top quality atmosphere" (the owner, 14 November) gives this module two milestones after G3s closes, M7 and M8: haze the weather sets, shafts of light, and fog near the camera, integrated jointly with the air in one view volume. They gate nothing in the V1 exit, and the exit scene does not declare the view volume.
- Assumption: M7 and M8 wait for M6, so fog's first frame waits for TAAU and for G3s to close.
- Assumption: lamps and other local lights do not light haze or fog; the Sun, the sky and the ground's light do.
- Assumption: the weather sets the haze, so `ATMOSPHERE`'s fixed air (a visual range of about 244 km) becomes an input with that value as its default. Every scene that sets nothing draws as before.
- Assumption: the terrain lip on the shipped Ground E pan is judged in M6.1 as a brightness step, and the held 17.4 h stop is judged; only the moving spur edge of the parked M3 crossing is left out.

## M1 The valley remains legible at moonless midnight: done
[record and reviews](../../evidence/ATMOSPHERE/M1-M4-records.md#m1-the-valley-remains-legible-at-moonless-midnight-done)

## M2 Stars emerge as the observer turns and time advances: done
[record and reviews](../../evidence/ATMOSPHERE/M1-M4-records.md#m2-stars-emerge-as-the-observer-turns-and-time-advances-done)

## M3 The west lookout’s shadow crosses the ground without a flash

<a id="m3-sunset-crossing"></a>

A parked spike, outside the done-when file: the stock graph ships and M6 does not judge the moving spur edge.
[The record](../../evidence/ATMOSPHERE/M1-M4-records.md#m3-the-west-lookouts-shadow-crosses-the-ground-without-a-flash-parked-spike)
holds the proposed reference clip, the diagnosis handoff and the finished work. It re-enters only as a new
commission against these criteria:

- [ ] **D1.** At the west lookout on the cooked valley, the near ground and middle distance read as continuous terrain during the 17.40–17.80 h crossing and a slow ±20° pan on Metal and Chrome. Independent side-by-side motion review matches the reference’s continuous moving edge. The timed 1× replay shows no newly introduced flecks, ribbing or detached bright bands. The fixed 1080p spur witness removes the known row-665 in-cascade flash at frames 226–227, and the six-stop route and both controls pass.
- [ ] **D2.** Compare against current main at the same receiver, source and Sun. The open Sun and full occlusion controls retain their expected result. The broad edge, close terrain, distant crest, represented compact-caster control and overlap exhibit no new visible false light or shadow across near/last/far range, camera and azimuth sweeps, or a cooked asset swap. Review full-resolution visibility and shaded images, with signed difference masks registered by receiver and time. Report new lit/umbra regions separately from the existing errors and compare asset replacement with a fresh run of the replacement. Record signed disk-reference error on the fixed witnesses as a measured open item. The original sub-texel caster stays in M4.
- [ ] **D3.** The regular graph preserves aerial and surface ownership, recovery and strict discarded-frame image equality. Record paired whole-frame and shadow costs, and pass workspace and independent code and visual reviews on the integrated result.

Work left when it was parked:

- [ ] D1–D3. Resolve cooked and tile ownership before a live candidate. Keep the submitted-slot history rules and three-screen stop limit.
- [ ] D1–D3. Integrate one shared surface evaluator, then capture the shipped ground and spur replays, range and cooked replacement controls, and paired frame costs. Review both targets in motion before promotion.

## M4 Exact finite-disk shadow reference match

A deferred bounded spike, outside the done-when file. M3 and the other V1 gates proceed without it.
[The record](../../evidence/ATMOSPHERE/M1-M4-records.md#m4-exact-finite-disk-shadow-reference-match-deferred-bounded-spike)
holds the finished work.

- [ ] Compare one reviewed representation with the original-geometry disk reference at the broad edge, distant narrow crest including moving receivers, the 64 m close terrain exclusion, the 24 cm compact caster at 160 m and the overlap. Keep the open Sun control and one coverage union. Use the frozen raised, distant, broad and close 4 m patches and the four row-665 receiver/phase domains at x=480, 720, 960 and 1200. Keep the crest-only ablation, close umbra, compact-only and overlap controls. Retain a 0.5 percentage-point continuous omission bound. Prove the retained blocked set is a subset of the reference or separately bound overcoverage to the same tolerance. Freeze finite sample and reference integration tolerances before the run.
- [ ] Publish the verdict and the complete source, receiver and caster cost for the measured domain. Only a passing candidate can propose a separate in-game landing.

Work left:

- [ ] Review at most one representation with a distinct source-side reuse mechanism and explicit producer, receiver and caster work caps fixed before running. Run it once against the frozen witnesses. Park it if it exceeds the cap or fails admission, retain the failure and stop. M3 and other V1 gates can proceed independently.

## M5 Other modules read the one sky and Sun, and one cloud optical depth dims direct sunlight once: done
[review](../../evidence/ATMOSPHERE/M5-review.md)

## M6 The valley's sky and sunlight pass the route in motion on Mac and Chrome, and G3s closes

Read [Closing G3s](DESIGN.md#closing-g3s) first.

- [ ] Recapture the route and the dawn-to-night scrub with TAAU on, on Mac and Chrome, and list which recorded
      defects still show.
- [ ] Fix what shows, one defect at a time, with the referees green after each and no reference, case or bound
      changed.
- [ ] Record the commit M6 starts from, and rerun it beside the final build for the cost comparison; quote the
      M1 and M2 records as history.
- [ ] Maturity rows and the brief's G3s line, naming solar sky and aerial accuracy as not qualified.

## M7 Haze thickens from crisp air to murk under a control, and shafts of sunlight stand in it below ridges and broken cloud, on the Mac and in Chrome

Read [The view volume](DESIGN.md#the-view-volume-haze-fog-and-light-near-the-camera) first. The aerial
producer is `crates/thrum-render/src/atmosphere/aerial.wgsl` with `AERIAL_EXTENT` in `gpu.rs`; the aerosol's
constants are written into shader source by `spectrum.rs`; the forward decoders are the `synthetic_aerial*.wgsl`
files, one of which splits solar in-scatter by the surface's shadow, which the volume must not repeat. The
volume's history uses `EFFECT-GRAPH`'s state kinds, as `examples/v1-game/src/effect` does. The control and the scene's declaration go behind a selection in `examples/v1-game` that is off by
default; that game is `SHOWCASE`'s, so leave its generator, routes and exit configuration alone. `CLOUDS`
builds rulings 1 to 4 in these same files; integrate on top of what has landed, and if its haze scale is
present make sure the volume does not take it. Record M7.1's base before changing anything, and measure M7.3's
bound before asserting it. `scripts/view_volume.py` is created here.

## M8 A test bank of fog stands in the valley: the land fades into it, sunlight dims inside it, and the camera walks in and out of it, on the Mac and in Chrome

The cloud-factor function and its field are in `crates/thrum-render/src/atmosphere/cloud.rs` and `cloud.wgsl`,
and the labelled test cloud field in `examples/v1-game/src/host/scene_source/cloud.rs`; the test medium sits
beside it and is replaced by `CLOUDS` M14. The joint reference is written here, in
`scripts/reference/atmosphere/`, apart from the shader. What a provider's coefficients mean is in
[CLOUDS' design](../CLOUDS/DESIGN.md#condensate-that-meets-the-ground-mist-fog-and-low-cloud). If a forward pipeline cannot take the ground band inside
WebGPU's sampled-texture limit, write the proposal in the log and end `BLOCKED`.
