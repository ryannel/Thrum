# SEA log

## 2026-11-14 — M1 CPU reference built

Added the registered, dependency-free `thrum-sea-model` crate: a conservative spectrum partition,
phase-preserving power response, displaced-world-point long-wave oracle, scalar cascade transform and
provider conformance harness. Nothing was copied or translated from Broadsides.

Verified ten CPU tests, exact regeneration of the 70-digit fixture, all item checks, and `./dev check`
on source `b0e08412fa39471b272d260f43a177e302977d43`. The unchanged verifier passed 42 native comparisons
and all four 481-record Chrome sets. [Evidence](../../evidence/SEA/M1.md) keeps the numerical proof,
baseline identities and limits together. Sea rendering and GPU cost remain later milestones.

Integration brought in main's CULLING renderer changes and the revised SEA plan, whose M1 items are
unchanged. Rechecked all ten model tests and the complete native/Chrome baseline verifier on
`43caeacc430f8c0de2d96cc7a42a8ee729b5890c`; all passed. The selected baseline record now names this
combined source. The merge gate runs the full repository check again.

- 2026-11-13 Owner, through the lead, when asked whether SEA and CLOUDS planning could start now: "On 2 use the
  fable planner like normal but you can bring that across when it makes sense." The lead's reading, recorded as
  an assumption in the plan: planning starts now, and this plan decides when the Broadsides Rust work is brought
  across, inside the port decision.
- 2026-11-13 Planned from the lead's task and [the brief](BRIEF.md): a design, a plan and a done-when file with
  ten milestones. M1 needs nothing; M2 names `EFFECT-GRAPH:M2`; M3 waits on `EFFECT-GRAPH:M3`, with
  `ATMOSPHERE:M5` and `CREATOR:M1`; M7 names `CONTENT-LOOP:M1`; M9 waits on `APPLE-PRESENTATION:M3`; M10 carries
  a check that fails until `APPLE-PRESENTATION`'s M7 and M8 are Done, since they are not planned yet.
- 2026-11-13 Planner's decisions on the five questions the brief left open. Cost account: three 256² cascades
  below 20 m, at most 192 analytic components above, a field cache for the long waves, everything stepped on the
  tick and drawn as a blend of two ticks. Wake operator: eWave on two body-following levels with complementary
  bands, which is how the water decision's two levels avoid carrying a wave twice. Rigid bodies: integrated in
  the example scene; the engine owns the query and the wake source. Refraction source: opaque colour and depth
  gathered only inside the screen rectangles of bodies, on every recipe, with no whole-scene copy. Foam look:
  feathered caps, pale water over submerged air and a widening trail are inside S4; wind streaks are not.
- 2026-11-13 Planner's decisions the brief did not name. The sea draws after opaque geometry and writes scene
  depth, so reconstruction and a later cloud march see it. A body's wake source is the vertical force the water
  put on it, and its own static dent is added back in its query. The display blends two tick banks, written on
  alternating ticks, because a history pair does not offer its older side on a skipped frame. Chrome is proved in
  its own milestone (M4) and then in each later one. Packages: `thrum-sea-model`, `thrum-sea` and `sea-scene` in
  `examples/sea`.
- 2026-11-13 Planner's reproductions, in [the evidence folder](../../evidence/SEA/design-proof/). Wake: transverse
  wavelength within 2% of theory at 3 and 6 m/s, a crest that travels on at its phase speed after source-off, two
  banded levels within 2.2% of one fine level, and 69% error when both levels carry the whole source. Long waves:
  f32 with f64 phase wrap is within 0.05 mm of f64 at 4 km from an anchor 50 km out after 24 hours, against 18 mm
  unwrapped; the declared tolerance is 2 mm.
- 2026-11-13 Asks of other modules, settled as [seam rulings](../ARCHITECTURE.md#seam-rulings-for-sea): an
  after-opaque pass may write scene depth, an effect's own pass may read scene colour and depth as textures, and
  the shell, page and capture tap serve a second ticking game. All three are built by this module's builder. No other module's design, plan or done-when
  item changed, and no contract's text changed.
- 2026-11-13 Not read for the design: the Atlas GDC 2019 slides (over the fetch limit), Crest's source and
  Horvath 2015. Rabaud and Moisy 2013 was read as an abstract only.
- 2026-11-13 Design review: [changes requested](../../evidence/SEA/design-review.md), five Must Fix findings and
  three Suggestions. The design, the plan and the done-when file were revised the same day; the entries below
  say how. New reproductions are parts A to E of `revision_proof.py` in
  [the evidence folder](../../evidence/SEA/design-proof/).
- 2026-11-13 MF1, numeric gates. No timing figure decides an item or a milestone. `sea_cost.py check` fails only
  on a missing or incomplete record or an uninvestigated tripwire. M3.6, M6.8, M7.9, M8.7, M9.10 and M10.5 were
  written again as paired records with investigations; M3 no longer ends `BLOCKED` on its frame time; M10 was
  renamed, since its name claimed the refresh.
- 2026-11-13 MF2, reflections. The gather's source rectangles (what is copied) and each body's receiving
  rectangle (which sea pixels may show it) are now separate. The receiving rectangle bounds every mirror image
  and refracted view of the body's box within a reach of 64 m and a slope allowance of 20°. Part C: no ray
  outside it in 36 cases. M8.5 holds that bound; M8.3 asks for whole reflections. Only registered bodies
  reflect, which is a new assumption in the plan.
- 2026-11-13 MF3, units. The gather turns copied scene colour back into radiance with the exposure scale and
  `ATMOSPHERE`'s lent camera aerial lookup, and the design declares four approximations with their range. Nothing
  new is asked of `ATMOSPHERE`. New item M8.9 holds a panel's reflection and submerged view to the equation at
  two exposures and two distances.
- 2026-11-13 MF4, hitches. The two banks are the committed state and the state committed before it, each with its
  tick, blended by time across the real span; a full frame writes both at one tick; a capture frame reads and
  changes nothing. Wake and foam take one step of up to 8 ticks (part A: 0.16% of RMS for one 8-tick step, 0.6%
  with one a second). M2.4 and M4.2 were written again to the endpoint table; M6.10 and M7.11 are new lifecycle
  items.
- 2026-11-13 MF5, foam. M7.1 was written again as an account of transfers and sinks: recipients rise with no
  birth, the inventory never rises, every stock ends at nothing (part D).
- 2026-11-13 Suggestions, all three taken. S1: the wake's source and border are stated as approximations, and
  the catch-up bound of 8 ticks is set from part A. S2: a query answer is valid at its use tick, and carries that
  tick and its state tick (design, M5.1, M5.2, the module list's body-query row). S3: inspections name the mode,
  the views and the range, and leave the route to the reviewer; the design's capture rule says so once.
- 2026-11-14 Owner, through the lead: "You can fully use everything in broadsides. But we should make sure that it
  aligns with the approach we want. But yes, you can copy that code out and reuse as is if it's useful." Then: "We
  can fully reuse anything from broadsides, but it's there as a resource where it's useful, it shouldn't dictate
  design."
- 2026-11-14 Owner, through the lead, on clouds: "That's the approach we were going with in broadsides that might
  not be the right approach here. The agent should look at that work from first principles and figure out how to
  design the best possible atmospheric system for the thrum game engine. Maybe that's a flexible set of cloud
  capabilities built from what we did in broadsides, maybe it isn't, but it should start from first principles."
- 2026-11-14 Lead's reading, recorded as assumptions in the plan: the owner's last words govern the sea too; the
  design of 13 November and the port decision are inputs to judge; Broadsides code, shaders and assets may be
  copied as is where the design finds them useful; Broadsides stays unmodified.
- 2026-11-14 Design judged again from first principles; the verdict on each inherited choice is in
  [the design](DESIGN.md#judged-from-first-principles). Kept, for reasons that do not rest on Broadsides: the
  three layers, the field cache, the GPU query, the two-level eWave wake, the reaction-force source, four foam
  stocks, body-only reflection, bodies integrated by the game, and all seven cues.
- 2026-11-14 Changed: the long waves, cascades and field cache are evaluated every frame at the display's time
  and keep nothing between frames, in place of tick copies blended in the surface pass. Only the wake and foam
  keep two endpoints. M2's name, M2.2 to M2.5, M2.7, M4.2 and M5.1 were written again to that; M6.10 and M7.11 now
  hold the endpoint table. Parts F and G of `first_principles_proof.py` are the evidence.
- 2026-11-14 Changed: cascade phase is a whole number of turns in a declared loop (1,024 s), because f32 seconds
  are 0.26 rad wrong after 24 hours (part F). New item M2.9 puts the loop frequencies in `thrum-sea-model`.
- 2026-11-14 Changed: every part beyond the open sea is a capability a scene can leave out at no cost (new item
  M9.12), as an assumption. The design names where Broadsides has each mechanism and how a copy is recorded.
- 2026-11-14 The licence and no-Broadsides-asset assumptions were deleted, answered by the owner's words above.
  Third-party material inside Broadsides keeps its own licence; that reading is an assumption. M10.9 was reworded.
  M1's items are unchanged: M1.8 is met by the log entry above, and its built model needs no change before M2.
- 2026-11-14 [The port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md) now says the sea's methods are this
  design's and Broadsides is a resource; its cloud half was left for `CLOUDS`' planner. The water decision's
  open-sea row and the module list's licence line were brought into step.
- 2026-11-14 Owner, through the lead, about this module: "Please make sure the sea work is actually thinking about
  water as a whole. So puddles, rivers, water falls, streams, lakes, and more. We want something that's built
  cohesively."
- 2026-11-14 Re-planned on those words. Water is designed as one system in [`WATERS`](../WATERS/DESIGN.md), and
  this module is narrowed to the part that never depended on where water lies: the wind's wave field. M1 and M2
  are unchanged, word for word, so M1 as it is being built stands. M3 to M10 were removed here and written again
  in `WATERS` as its M2 to M11, generalised to every body of water.
- 2026-11-14 The design was cut to the wave field. Its sections on the surface, the query, the wake, whitewater,
  optics, capabilities, cost and risks moved to `WATERS`' design, with what changed there said in that design's
  table of verdicts. Once M2 is Done, `thrum-sea-model` and `thrum-sea` pass to `WATERS`.
- 2026-11-14 The assumptions that were about the surface, wake, optics, cost and the close moved to
  [`WATERS`' plan](../WATERS/PLAN.md#open-for-the-human). The one on body-only reflection was replaced there:
  water now reflects what is on screen.
- 2026-11-14 Built M2's composable every-frame GPU field and loop-frequency CPU model.
  Metal source `4b1fbfa8` passes the long/cache, cascade displacement/rate, moments,
  lifecycle, wind-transition and default-WebGPU-limit tests on this M3 Max. The
  [M2 record](../../evidence/SEA/M2.md) retains the errors and fixed measured bound;
  the [cost record](../../evidence/SEA/M2-cost.md) explains the estimate overruns.
  No surface or integrated performance is claimed. Exact baseline qualification
  and final integration are being completed separately.
- 2026-11-14 Resumed M2 and completed all three Metal integration targets and the
  library tests at `4b1fbfa8`. The integration script rebased the work to
  `4372f847`, then refused integration because its exact-baseline proof was
  absent. The rebase changed none of the sea crates, their renderer dependency
  or workspace dependencies; the ambient target also passed again at `4372f847`.
  The first verifier attempt at that source stopped at native route-motion
  capture: Metal was correct, but the interval was incomplete, with 153 readback
  ring omissions and zero writer omissions. Its compact diagnostic archive is
  `tmp/sea-m2/baseline-first-failure.json.gz`; disposable pixels were removed
  after preserving the metadata. This identifies the rejected completion
  condition, not the cause of the omissions. The complete scripted M2 check is
  background job `20261114-054946-45357`; it remains queued between target checks
  and includes another baseline attempt. No baseline reference was changed.
