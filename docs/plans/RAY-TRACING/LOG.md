# RAY-TRACING log

- 2026-11-17 Owner: "Just do ray tracing and mesh shaders on mac, let's leave PC for now."
- 2026-11-17 Planned from the lead's task, quoted in [the design](DESIGN.md#the-request). Six milestones: the
  structure for entities, the whole valley with edits, hard shadows from the Sun and lamps, soft Sun shadows,
  ambient occlusion, and the paired record with promotion, which waits for `SHOWCASE` M8. Reflections by ray
  and global illumination are left for a later plan. [The planning probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/README.md)
  showed that wgpu 30.0.1 traces correctly on this Mac from compute and fragment stages. The design review is
  owed.

- 2026-11-17 Human: "Ok let's lower the bar." The most plausible result as fast as possible, not physical accuracy; references become
  diagnostics. Plan: dropped M4.1 and M5.1, which held the soft-shadow and occlusion estimators to reference fractions; their look
  is judged by inspection. The ray oracle of M1 to M3 stays, because whether a ray is blocked is a fact about geometry. Five cost
  records pass on their check alone.
- 2026-11-18 [The design review](../../../.orchestrate/evidence/RAY-TRACING/design-review.md) asked for changes: seven Must Fix,
  resolved in the design and items. (1) Promotion needs cost no higher and a gain, beside portable runs, a
  difference image and a ten-minute run (M6.1 to M6.5). (2) The window asks for `ray-query` only in a qualified
  configuration, and the portable recipe refuses ray effects (M3.6, M6.4). (3) A centre ray traced each frame
  overrides a settled history, and a soft edge's latency is a stated settle length (M4.4, M4.7, M5.5).
- 2026-11-18 The same review, continued. (4) A ray's origin is lifted 8 float steps, not 256, after
  [a second probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/OFFSET.md) (M3.1). (5) Membership is the scene's,
  before any culling (M2.2, M2.4, M3.4). (6) Ray demand is the scene's and reaches the arena and cooked
  residency before allocation (M1.5, M2.6). (7) Pipeline creation is counted for each generation (M3.7). The
  suggestion was taken: M1.6 records a still scene and every entity moving beside the 1,500 movers. Items were
  renumbered. The design review is owed.
- 2026-11-18 Human, 17 November: "Ok let's lower the bar." Applied in full. The CPU ray reference no longer
  decides any item (old M1.2, M1.3, M2.1, M2.3, M3.1, M3.4). Tests aim rays whose answer is plain from how the
  scene was built and compare what the structure holds with the scene's records; the valley's structure is
  inspected in a traced view (M2.1). A builder may still write the reference as a diagnostic.
- 2026-11-18 The design review at `eaf682210eab` asked for changes: six Must Fix, no Suggestion. Lead: "the
  traced path is an explicit, recorded opt-in for this Mac, judged by eye against the inspection list, with a
  paired whole-frame timing kept as a tripwire, and a plain fallback to the portable path when its feature is
  missing. Drop adapter-keyed automatic promotion and recipe-difference crediting."
- 2026-11-18 Findings 1, 2, 5 and 6, resolved by that rule. No `Auto`, no qualified list, no host-added
  feature, no gain list and no comparison with the portable recipe. A ray request that cannot be met, or
  whose scene fails to prepare, falls back to the map frame and reports why (M3.6, M5.6). A launch gives a
  name as required or preferred, and a preferred one is retried once without it (M1.1, M6.2). M6 is now
  this Mac's opt-in record: the final-build inspection and a Maps-against-Traced pair on one recipe, with the
  whole frame measured as a GPU span through the output's consumer (M6.1 to M6.3). Old M6.2, M6.3 and M6.5
  are gone and M6 was renumbered.
- 2026-11-18 Finding 3, on its merits: the one-frame remnant of a soft edge a shadow has left is stated as a
  limit, not hidden. The history's settled state is its value (0 or 1), the pass settles a texel whose
  neighbourhood agrees, and M4.4 inspects a caster that outruns its own edge (M4.4, M4.7). More inline rays
  are the named candidate if the remnant reads as a second edge. Finding 4, on its merits: an instance's
  shadow bit is a fact about the thing and never follows Sun maps, so a lamp-only room is shadowed with Sun
  shadows off, for entities and cooked placements (M2.4, M3.4, M3.8). M3.12 was split from M3.6. The design
  review is owed.
- 2026-11-18 The design review at `66c37436f35a` asked for changes: two Must Fix, one Suggestion. (1) The
  opt-in could not reach tracing by itself, because nothing chose its recipe. The valley's selection now
  prefers Apple tile where the device enabled `ray-query`, under ruling 12, which M6 builds if `MESH-SHADING`
  M3 has not. A named recipe resolves as today, and portable draws maps and says why (M6.2). (2) M6 never
  judged the frame the launch line produces. New M6.3 inspects the opt-in as launched; the combined
  inspection is M6.4 and is labelled by its settings; the attended run covers both (M6.5). M6 renumbered.
- 2026-11-18 The same review's Suggestion was taken: each run of the pair keeps its resolved recipe, Sun
  method and cooked draw path, and the `Maps` arm names Apple tile (M6.1). Also brought into step with
  ruling 7 as `MESH-SHADING`'s third review left it: the GPU figure is the timed-pass span, with the queued
  transfers outside it disclosed (M3.10, M4.9, M5.7, M6.1). The design review is owed.
- 2026-11-18 Built the named device request and scene-owned entity structure. Mesh structures share arena
  geometry and the scene build follows the accepted draw rows at the frame's alpha. Verified M1.1–M1.6
  on Apple tile Metal at `e0fb4a93a`, with an exact no-demand comparison to the pre-change renderer.
  The contended field-count CPU rebuild was 9.804 ms median, with finish and submit the larger share.
  [Proofs and tripwire investigation](../../../.orchestrate/evidence/RAY-TRACING/M1/README.md).
- 2026-11-18 Routed the shared native test gate through registry admission, resolved ray demand before
  resource creation, and restored the sky kit's public error variants. Verified the portable fallback and
  the shared gate's ordinary and named requests on Metal, and compiled the sky kit's exhaustive match.
  [Review fix evidence](../../../.orchestrate/evidence/RAY-TRACING/M1/README.md#review-fixes).
- 2026-11-18 Cooked residency now builds mesh structures from its draw buffers and contributes every
  placement using the raster publication's transform and switch. Verified sampled rays and raster bytes
  across placement and entity edits, discard, refusal and device replacement on Apple tile Metal.
  Inspected four T2 fixed valley pairs and native-size contact crops. The two contended timing runs found
  a cold terrain-build outlier and an estimated 0.737–0.905 ms added GPU cost for the edit.
  [M2 evidence](../../../.orchestrate/evidence/RAY-TRACING/M2/README.md).
- 2026-11-18 Routed the opt-in M2 timing probe's two pipelines through the prepared scene's device
  generation. The probe installation is recorded as a creation transition.
- 2026-11-18 Completed arena builds during startup and device preparation, carried them across
  same-device visual replacement, and gated initial cooked publication on completed mesh builds.
  Verified both Metal generations, the edit builds, and the traced view's instance colors and distance
  cue. [Review fix evidence](../../../.orchestrate/evidence/RAY-TRACING/M2/README.md).
- 2026-11-19 Added opt-in hard Sun and lamp shadows with prepared fallback and map-only secondary views.
  The Metal fixtures verified receiving-surface hits, cooked casters, no-Sun parity, air-shadow bytes, and
  day, night and capture preparation on two device generations. Inspected the fixed route and three movers
  at elapsed speed, and kept the night lamp contact crops. The contended repeat showed higher traced frame
  CPU time and timed-pass span, with both cascade passes absent. [M3 evidence](../../../.orchestrate/evidence/RAY-TRACING/M3/README.md).
- 2026-11-19 Corrected broad self-shadow facets on coarse rounded receivers, the mirrored lamp receiver
  and native traced-pipeline fallback. Metal hit IDs identified the three angular balloon patches as shadows
  from other balloons beyond 600 m. Recaptured the fixed Sun route, four-lamp stop and realtime movers on
  Metal. The lamp fixture excludes its wall from the camera and all three Sun volumes at both poses.
  [Review fix evidence](../../../.orchestrate/evidence/RAY-TRACING/M3/review-fixes.md).
- 2026-11-19 The design review at `77453aab4883` asked for changes: two Must Fix, one Suggestion. (1) M3's
  facet correction skipped nearby hits on the receiving instance by facing, which erased a sheet joined into
  the same mesh. It is now one excused passage: a ray in the terminator band starts inside its triangle, may
  cross its own instance's far face once within the reach the vertex normals give, and is then an ordinary
  ray (new M3.13). (2) The visibility pass applies the same rule from what a ray reports, bounded by a dive
  limit, and a fragment whose passage is refused shows its centre ray (M4.7 rewritten, new M4.11).
- 2026-11-19 The same review. [A third probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/PATCH.md)
  checked the rule on three bodies. The Suggestion was taken: an inconclusive pair is reported as runs that
  could not tell the methods apart (M6.1). M1 and M2 shrank to their headings. The design review is owed.
- 2026-11-19 The design review at `cb6efe8a9c80` asked for changes: one Must Fix, no Suggestion. The
  visibility pass excused a dipped ray's first crossing of its own instance at any distance, so past an
  open rim it excused the back of a panel joined into the same mesh metres away, and the panel's soft edge
  went hard. The pass now records such a ray both ways, `passed` and `stopped`, and the fragment reads the
  one its own centre ray's passage bears out: a refused passage reads `stopped` and no longer discards the
  history. A fragment whose plane the disc straddles takes the passage too. A fourth forward target that
  would hand the pass the reach was weighed again and left out: `SceneSurface` gives every contribution
  three colour formats. M4.7 and M4.11 are rewritten, and M4.9 records the history's bytes.
- 2026-11-19 The same review. [A fourth probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/SOFT.md)
  ran the review's case: 540 and 480 of 1,024 disc rays blocked in all four arrangements, where the
  reviewed rule blocked none at the open rim under the joined panel. This was the second design review in
  a row to ask for changes, so the script approves this revision without another.
- 2026-11-19 Replaced the Sun's terminator filter with one bounded crossing. Metal forward shading kept
  the band lit beside a sheet and shadowed it behind a joined or separate sheet of either winding.
  A real cooked pipeline fault on device replacement retried with Maps and retained the cooked scene.
  Inspected the fixed valley route, four night lamps and three movers. Recorded the timing under load.
  [M3 Metal evidence](../../../.orchestrate/evidence/RAY-TRACING/M3/passage.md).
- 2026-11-20 Initial attachment, visual publication and device replacement now share the cooked
  trace preparation fallback. On Metal, a real invalid pipeline at each phase resolved the Sun
  to Maps and disabled lamp shadows with `PreparationFailed`. Each complete cooked scene matched
  its no-request control byte for byte. [Metal proof](../../../.orchestrate/evidence/RAY-TRACING/M3/passage.md#preparation-fallback).
- 2026-11-20 Updated the terminator fixture for the shared Metal admission helper. With Metal required,
  the clear receiver and all eight sheet cases passed. [Metal proof](../../../.orchestrate/evidence/RAY-TRACING/M3/passage.md#one-crossing).
- 2026-11-20 Cooked pipeline preparation now validates before replacing live bindings. A same-device
  retry with Maps reuses the resident draw buffers and retires the ray structures. On Metal, the first
  submitted fallback image matched the no-request control with zero upload bytes for a receiver that
  initially required more than one upload budget. [Metal proof](../../../.orchestrate/evidence/RAY-TRACING/M3/passage.md#resident-cooked-fallback).
- 2026-11-23 M3 diagnosis. The reviews followed failures from native pipeline error handling through device
  replacement and attachment to the first submitted frame after a visual change. Per-member validation
  still let an early member evict its Maps residency before a later member failed. M3 now requires one
  commit for the scene and the whole list, so Maps retries from the original accepted state. M3.6 checks
  selection and primary pipeline fallback. M3.14 checks the shared boundary on the CPU and M3.15 compares
  the first submitted bytes and residency on Metal in a small, bounded-upload fixture.
  [Final review and previous finding](../../../.orchestrate/evidence/RAY-TRACING/M3-review.md).
- 2026-11-23 M3 design review. Existing contributions keep their Maps hooks and admit tracing only through
  optional candidate preparation. The boundary includes host construction and stages coupled resize before
  publication. M3.14 and M3.15 remain small merge guards, and M3.16 waits for the registered `long-tests`
  result on main. [Design review](../../../.orchestrate/evidence/RAY-TRACING/design-review.md).
- 2026-11-23 Final design review of `3ce33cc0c`. RAY-TRACING M5 owns candidate preparation for the valley's
  cloud provider. M5.3 now verifies traced ambient occlusion at a prop's base with the production cloud sky
  active under CLOUDS M7's overcast preset. [Design review](../../../.orchestrate/evidence/RAY-TRACING/design-review.md).
- 2026-11-23 Scene preparation now stages the whole contribution list before publication. The CPU guard
  verified admission, ordered commit and rollback. On Apple M3 Max Metal the first same-device fallback
  submission matched the no-request control with its original cooked generation and zero copied bytes.
  Repeated the fixed and realtime route, hard-shadow fixtures and timing. The timing repeat confirmed
  increased forward cost with tracing. Main's workspace check failed independently of this branch.
  M3.11 and M3.16 remain open for the registered checks after integration.
  [Verification and captures](../../../.orchestrate/evidence/RAY-TRACING/M3/transaction/README.md).
