# CULLING plan

Closes gate G3c: only what the camera can see is drawn. [Design](DESIGN.md) · [done-when file](DONE.json), which
owns the criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Frustum, Occlusion on cubes, the bucket route and Sun-caster culling were built before the loop; the
[design](DESIGN.md#purpose-and-scope) lists them with their evidence. Read
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any work package.

## Open for the owner

- Assumption: G3c closes on the scenes that exist, the mixed-mesh field and the valley as it stands, before `SHOWCASE` assembles the exit scene. `SHOWCASE` reruns the same equivalence and cost commands there, and the default mode may change then by the same rule.
- Assumption: "GPU culling" in the V1 exit is met by Frustum as the default with Occlusion selectable, if the measured rows do not favour Occlusion on any shipped scene. The gate's Hi-Z clause is met by Occlusion running, equal to Direct, on Mac and Chrome.
- Assumption: the native-only culling recipes (subgroup compaction, first-instance draws) are not built for V1. They stay candidates for the performance phase the brief puts after the exit.
- Assumption: the culling mode is chosen when a scene is prepared, at launch or page load, and no control switches it while the game runs. The brief and gate G3c do not ask for a switch, and one that works needs a change across the frame runtime, the browser host and the contribution seam; the design says why and names the mechanism to cost if it is wanted.
- Assumption: WP1 starts as soon as this design passes and does not wait for `BASELINE-GUARD` WP1. Until that gate exists, WP1's own check covers the Mac baseline only and Chrome's is rechecked when the gate's builder first runs the verifier on main. The other choice is to add `BASELINE-GUARD:WP1` to WP1's `needs`, which holds WP1 until the gate is built. WP2 and WP3 do wait for it.
- Assumption: the forward pass's coverage output is on in every scene, with no switch, so that the game draws one image under GPU load. It changes pixels along shared triangle edges, so WP3 refreshes `APPLE-TILE`'s reference sets once, in its own landing, and its cost in frame time is measured there, not known today. The other choice keeps today's output as the default behind a selection, and leaves the variation in every scene and every exact test that does not ask.
- Assumption: after WP3, no loaded campaign is repeated on main. WP3.11's campaign proves the merged build once. Later, the main-branch check guards only the pipelines its references draw, which are Direct's, and a culled or bucket pipeline that lost the correction shows only as some module's exact comparison varying under load, which stops that module. The other choice is a campaign on main on a schedule, at 45 to 90 minutes of loaded GPU each.
- Assumption: `CULLING` takes no quiet-machine sitting and no attended panel sitting. Gate G3c asks for costs "as tripwires", so WP3.3's and WP3.13's rows are paired, taken once while other builders work, and recorded with their load, and no item judges frame cadence: `SHOWCASE` judges "smooth at the display's full refresh" on the culled exit scene (the design's Frame cadence section). The other choice is one sitting with the other builders paused at the end of WP3, as `ORCHESTRATE.md` offers a module's last work package; it gives uncontended figures and a cadence record for today's valley, costs every builder that pause, and `SHOWCASE` takes the record again.

## WP1 On the mixed-mesh field, Occlusion draws only what Direct shows, pixel for pixel: done
[review](../../../.orchestrate/evidence/CULLING/WP1-review.md)

## WP2 The game picks its culling mode from the device and the measurements, and says which mode ran: done
[review](../../../.orchestrate/evidence/CULLING/WP2-review.md)

## WP3 The valley's 10,000 entities are culled by default and match the unculled image pixel for pixel, on Mac and in Chrome

Criteria: WP3.1–WP3.13 in the done-when file. Read [Frame cadence](DESIGN.md#frame-cadence) first, then
[One image under load](DESIGN.md#m3-one-image-under-load) for anything you change.

Everything is built and on main. The coverage output, its references and pins landed at `534ba5021`, the
campaign on main passed, and [the second review](../../../.orchestrate/evidence/CULLING/WP3-review.md) passed eleven items. It
saw no culling artifact in WP3.7's or WP3.8's captures and left both unjudged for "full-refresh smoothness". The
re-plan of 17 November 2026 settles that cadence is not this module's, and rewrote WP3.7, WP3.8 and the quality
bar to say so. Nothing else is asked.

Because the definitions changed, the script cleared every item's claim and verdict. No proof changed, so claim
each item again from the evidence it had:
[the list](../../../.orchestrate/evidence/CULLING/WP3-replan/claims-before-2026-11-17.md). WP3.7 and WP3.8 are claimed from your
merged-build capture pass, `.orchestrate/evidence/CULLING/WP3/replan/motion.md`. Take no new capture, campaign, cost
row or reference run, unless the script says a check failed or you have merged a change under `crates/` or
`examples/v1-game/` since that proof.

No item needs an awake panel, a capture-off run or paused builders, so ask for none. Commit `83165972e` on
`build/culling`, unmerged, holds a handoff, a log entry and `WP3/motion-followup/` that ask the lead for a
cadence sitting. That request is withdrawn: rewrite the handoff and keep the follow-up page only as history
that says so. Its finding that Chrome's recorder runs on a headless 60 Hz timer is right and is now in the
item.

If you do merge code again: [the design](DESIGN.md#how-it-lands) names what a rebase that moves a registered
reference repeats, and a merge of yours under `crates/` or `examples/v1-game/` costs WP3.11's campaign again.
WP3.13's rows never stop the work package or the merge. The registry is `BASELINE-GUARD`'s: change only the `pin`
and the `refresh` of a baseline whose reference moved.

Stop rules, for every work package:
- If a mode differs from its oracle in any pixel, stop and diagnose. Never widen a comparison, sample fewer frames or change the oracle to pass.
- If your change fails an exact test or baseline another module owns, do not edit it. Write the proposal in the log and end `BLOCKED`. The one exception is the refresh the module list rules for WP3's coverage output, with its pins.
- Prove a thing once on your branch and merge. The script's run on the merged tip is the proof of record; run again only the check it says failed, and repeat no proof for a rebase unless the design says so. WP3.11's campaign is the one proof you take on main. `verify` passes from the run on main: set it `pass` with the evidence `main-branch check exact-images`, and never run plain `verify` yourself or under a lock.
- Write no job that waits for, asserts or follows another job, a merge or a verifier run, and no hook or helper around `merge`. Start one command, collect it, and decide the next step yourself.
- A default mode changes only on the recorded rows the design's rule names. No speed claim rests on one run or on an uncontended figure that does not exist.
- Chrome and simulator checks need a browser or simulator launch outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
