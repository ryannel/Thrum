# T2-CAPTURE design review 2 (with the Mac M2 re-plan)

9 November 2026. Independent `/review-milestone` design re-review of revision `6317102f`: the
[brief](../../design/modules/T2-CAPTURE-BRIEF.md), [design](../../design/modules/T2-CAPTURE.md),
[plan](../../design/modules/T2-CAPTURE-PLAN.md) and [done-when file](../../design/modules/T2-CAPTURE-DONE.json),
plus the same commit's re-plan of [Mac M2](../../design/modules/PLATFORM-RUNTIME-MAC-PLAN.md)
([criteria](../../design/modules/PLATFORM-RUNTIME-MAC-DONE.json)) and its consumer edits (ORCHESTRATE, BUILD-PLAN,
AI-DEVELOPMENT, C12, PLATFORM-RUNTIME, maturity ledger, Mac M2 evidence page). The first review is
[design-review.md](design-review.md). Reviewer: Claude Opus 5.5. No milestone is judged, so no done-when file is
changed. Nothing was built or run on a GPU. This is a read of the documents and the source, plus
`./dev check-docs` (passes on `6317102f`) and parsing both JSON files (both parse).

**Verdict: DESIGN CHANGES, 2 findings.** The T2 design itself is now sound and T2 M0 is ready to build once the
small wording fix in F2 is made. All ten first-review findings are addressed. The remaining blocking problem is in
the Mac M2 re-plan: it still has no route to done. Its milestone-level `needs` on `APPLE-TILE:M3` puts it, and its
attended tooling items M2.3 and M2.4, behind a module whose design has now failed review twice and is parked. Its
status lines don't say so.

## Capabilities derived from the brief (written before reading the revised design)

1. Exact-build stills, fixed-time sequences and separately labelled elapsed-time sequences of real engine output on
   the named GPU and recipe, with no Screen Recording grant and with the console locked.
2. Input, committed state, interpolation and history, frame identity, clock domains and missing samples are kept. A
   slow run must not become a smooth movie.
3. The real render graph and temporal path, bounded asynchronous readback, bounded disk and encoding work. Disabled
   capture adds no copy, readback, allocation or synchronization.
4. First Mac proof: cooked noon pan, paused world, locked, no permission; plus a running world with interpolation.
   Full frames, native-size crops, 1x motion, timing and capture-gap negative controls.
5. Chrome and the iOS simulator qualified separately, never as native panel evidence.
6. Each later visual milestone names its capture mode, surfaces, target and inspection list.
7. Panel cadence, compositor and native UI, OS input and focus, and EDR or photon proofs stay separate, with attended
   rows labelled by reason. No security bypass, no automatic unlock, no silent offscreen substitution.
8. Overhead and scaling measured without a numeric gate, and historical Mac evidence preserved with cadence open.

All eight are covered by the revised design. Nothing outside the brief was added.

## Disposition of the first review's findings

| First-review finding | Disposition at `6317102f` |
| --- | --- |
| F1 windowed tap unowned | **Addressed.** Mac M2.3 (attended) integrates and qualifies the tap on a real CAMetalLayer window: known-pattern correspondence, no lease extension, capture-off direct path, pending-capture surface loss, `./dev check` under `gpu`. Owner stated consistently in T2, C12, PLATFORM-RUNTIME, BUILD-PLAN |
| F2 cadence measured with capture on | **Addressed.** M2.2 has three arms; the capture-OFF traced arm is the shipping cadence claim, and ON swaps are labelled "under capture" |
| F3 trace finalization unowned | **Addressed in substance.** M2.4 commissions a bounded investigation (changed hypothesis, at most two new attempts, finite timeouts, owned-process stop rule, blocked outcome recorded). But it is gated behind the milestone's prerequisites, including the parked APPLE-TILE (F1 below) |
| F4 writer throughput | **Addressed.** Raw sequential spool, deferred packaging with decoded-hash checks, a first M0.3 probe that rejects cached-write rates, a 16 GiB per-session allowance. I checked the arithmetic: 8,294,400 B x 120 x 4 s = 3.98 GB; 3 staging + 8 CPU buffers = 91.2 MB. The data volume has 66 GiB free at review time, so one session fits |
| F5 paused-world no-render gate | **Addressed.** The headless target goes through the ordinary presenter and `SurfaceOutput` path, and the design forbids a capture-only admission branch or ticking the world. I checked this against `crates/thrum-render/src/runtime.rs:946`: supplying a `SurfaceOutput` does clear the `completed_ticks == 0 && surface.is_none()` early return, and paused extraction then reuses the last extracted page (`:992`). M0.4's `ask` requires fresh submissions with an unchanged committed tick |
| F6 no `check` | **Addressed.** M0.1, M0.2, M1.2, M2.2 (T2) and Mac M2.3 carry `./dev check` with `lock: gpu`, plus named target-specific runs |
| F7 bundled items | **Addressed.** M0 is now nine items: tap, queues, writer probe, workflow, paused pan, running world, stalls, overhead, runbook |
| F8 locked-console evidence | **Addressed.** Cited, with its limit stated. The locked condition is taken opportunistically, never forced, never unlocked by automation. (Small inaccuracy: the design says the probe scratch "is not present in this checkout"; it is present in `tmp/platform-runtime-mac-m2/` of the `platform-runtime-mac` worktree. Harmless, because the design doesn't rely on it) |
| F9 stall hooks | **Addressed.** Diagnostic scenario actions, off by default, pinned by the scenario hash, with an ordinary-run test that neither fires (M0.7) |
| F10 leftovers and interim capture | **Addressed.** Mac M2.2 `evidence` cleared. The scope of the interim Chrome endpoint and `simctl` captures is stated identically in ORCHESTRATE, AI-DEVELOPMENT, C12 and the T2 rollout section |

## Findings (worst first)

### F1. Mac M2 still has no route to done: it needs `APPLE-TILE:M3`, which is parked, and M2.3/M2.4 are gated behind it (verified from documents)

**What I found.**
- `PLATFORM-RUNTIME-MAC-DONE.json` M2 `needs` is `["T2-CAPTURE:M0", "APPLE-TILE:M3"]`. That is milestone-level, so
  it gates all four items, including the attended tooling items that have nothing to do with the G3t surface path.
- M2.4's `ask` says the investigation starts "after this re-plan passes independent design review **and the
  milestone prerequisites are done**". The plan's `Next` line orders it "T2-CAPTURE M0 and APPLE-TILE M3; Mac builder
  then diagnoses trace tooling (M2.4), qualifies windowed tap (M2.3)".
- APPLE-TILE's second design review (`ff99cd8d`, [design-review-2.md](../APPLE-TILE/design-review-2.md)) returned
  DESIGN CHANGES with 7 findings, and APPLE-TILE is parked after these two failed reviews (per this review's
  commission). Nothing at `6317102f` records that. The APPLE-TILE plan still reads "independent re-review of revised
  design/JSON is owed", and the Mac plan, BUILD-PLAN and the maturity entry read as if `APPLE-TILE:M3` were an
  ordinary queued prerequisite.
- That review's finding 5 already pointed out the transitive chain. G3t is sequential, so M3 waits on M0-M2, and G3t
  M1 needs `T2-CAPTURE:M1` (Chrome) and `T2-CAPTURE:M2` (simulator). Mac M2 therefore waits on Chrome capture,
  simulator capture and four G3t milestones. It suggested stating this or letting M2.1 qualify now. `6317102f` does
  neither. Its Mac `Blocked` line names only `T2-CAPTURE:M0` / `APPLE-TILE:M3`, and the T2 plan says "no dependency on
  G3t/G4".

**Why it matters.** This is the first review's F3 again in a different form. The active G3b gate's last milestone
reads as `open` with a planned sequence, but it is blocked behind parked work with no owner or trigger to unpark it.
Gating M2.3 and M2.4 behind G3t also delays the two attended unknowns (does the windowed tap work on a real
CAMetalLayer, and can trace finalization be fixed at all) until after four G3t milestones. Those could be learned now,
in an attended slot, on the current surface path.

**Should be (either):**
- Decouple. Mac M2 needs only `T2-CAPTURE:M0` and qualifies on the current surface path. The post-G3t rerun of M2.1
  and M2.2 becomes a G3t M3 item (or a new Mac milestone that `needs: APPLE-TILE:M3`), so G3t's surface change still
  can't skip requalification. Or:
- Keep the ordering but say so. Move M2.3 and M2.4 into their own milestone that needs only `T2-CAPTURE:M0`. Record in
  the Mac plan's `Blocked` line, in BUILD-PLAN's G3b line and in the maturity entry that final M2 qualification is
  blocked on the parked APPLE-TILE design, transitively including T2 M1 and M2. Name what unparks it.

Either way, record APPLE-TILE's parked state where its consumers read it.

### F2. The Mac pan is inspected "against the proposed Ground E composition", a reference image (verified from documents; may be minor)

T2 M0.5's `inspect` reads: "inspect the full noon pan and both holds against the proposed Ground E composition".
Mac M2.1's reads: "against the proposed Ground E noon composition". That composition is the image
`docs/evidence/v1-workload/g7-showcase/route-ground-e.png`, marked "(proposed, pending)" owner review. The profile's
quality bar is "judged in words against inspection items, never against target images". A reviewer can't tell
whether "matches the PNG" is the test, and the PNG is itself still pending owner review.

This may not matter much. The image is a same-scene engine capture used to confirm the scene and framing, not a
quality target. But the item should say that.

**Should be:** replace the image comparison with worded entries, for example: rock, ridge line and sphere all in
frame at both holds; the sphere stays left of the ridge through the sweep; no cut, snap or reversal. Or state that
the PNG is only a scene and framing identity control, and that quality is judged against the worded list and the
`bar`.

## Done-when judgeability

| Item | Could a reviewer judge it? | Would it catch a convincing fake? |
| --- | --- | --- |
| T2 M0.1 | Yes: source trace plus `./dev check` under `gpu` | Yes. Pixel-equality with known patterns and odd widths, plus capture-on/off history comparison |
| T2 M0.2 | Yes | Yes. Stale-callback, saturation, disk-error and finalization-deadline controls |
| T2 M0.3 | Yes | Yes. Rejects cached write acceptance; decoded-hash check on every frame |
| T2 M0.4 | Yes, but only when the console happens to be locked, for the reviewer too | Yes. Includes the paused-world fresh-submission/unchanged-tick test |
| T2 M0.5 | Yes, once F2's wording is fixed | Yes. Fixed-step route and endpoint redraw explicitly excluded; omissions make it inconclusive |
| T2 M0.6 | Yes | Yes. Intermediate-alpha samples joined to endpoints |
| T2 M0.7 | Yes | Yes. Compares the recorded delay with image repeats and PTS; ordinary-run non-firing test |
| T2 M0.8, M0.9 | Yes | Adequate. Observations only, by design |
| T2 M1.1, M1.2, M2.1, M2.2 | Yes | Adequate. Each names refusal controls, forbids faked visibility, requires target-specific runs beyond `./dev check` |
| Mac M2.1 | Yes, once F2's wording is fixed | Yes |
| Mac M2.2 | Yes | Yes. OFF arm qualifies cadence, ON swaps labelled under capture, numeric Swap Time and a stated join error. Unreachable while F1 stands |
| Mac M2.3 | Yes | Yes. Known pattern, pending-capture surface loss, capture-off allocation check; occluded window can't pass. Unreachable while F1 stands |
| Mac M2.4 | Yes | Yes. Excludes "trace directory exists", historical swaps and substitute timestamps. Unreachable while F1 stands |

## Owner's aim

The aim is met for engine images and runtime motion. T2 M0, Mac M2.1 and later Mac visual milestones need neither an
unlock nor Screen Recording, and the remaining attended rows (M2.2, M2.3, M2.4) are genuinely attended and labelled
with their reasons. What stops Mac M2 from closing is no longer the attended boundary. It is the dependency on parked
G3t work (F1).

DESIGN CHANGES: 2 findings
