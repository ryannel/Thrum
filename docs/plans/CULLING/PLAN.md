# CULLING plan

Closes gate G3c: only what the camera can see is drawn. [Design](DESIGN.md) · [done-when file](DONE.json), which
owns the criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Frustum, Occlusion on cubes, the bucket route and Sun-caster culling were built before the loop; the
[design](DESIGN.md#purpose-and-scope) lists them with their evidence. Read
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any milestone.

## Open for the owner

- Assumption: G3c closes on the scenes that exist, the mixed-mesh field and the valley as it stands, before `SHOWCASE` assembles the exit scene. `SHOWCASE` reruns the same equivalence and cost commands there, and the default mode may change then by the same rule.
- Assumption: "GPU culling" in the V1 exit is met by Frustum as the default with Occlusion selectable, if the measured rows do not favour Occlusion on any shipped scene. The gate's Hi-Z clause is met by Occlusion running, equal to Direct, on Mac and Chrome.
- Assumption: the native-only culling recipes (subgroup compaction, first-instance draws) are not built for V1. They stay candidates for the performance phase the brief puts after the exit.
- Assumption: the culling mode is chosen when a scene is prepared, at launch or page load, and no control switches it while the game runs. The brief and gate G3c do not ask for a switch, and one that works needs a change across the frame runtime, the browser host and the contribution seam; the design says why and names the mechanism to cost if it is wanted.
- Assumption: M1 starts as soon as this design passes and does not wait for `BASELINE-GUARD` M1. Until that gate exists, M1's own check covers the Mac baseline only and Chrome's is rechecked when the gate's builder first runs the verifier on main. The other choice is to add `BASELINE-GUARD:M1` to M1's `needs`, which holds M1 until the gate is built. M2 and M3 do wait for it.
- Assumption: the forward pass's coverage output is on in every scene, with no switch, so that the game draws one image under GPU load. It changes pixels along shared triangle edges, so M3 refreshes `APPLE-TILE`'s reference sets once, in its own landing, and its cost in frame time is measured there, not known today. The other choice keeps today's output as the default behind a selection, and leaves the variation in every scene and every exact test that does not ask.
- Assumption: after M3, no loaded campaign is repeated on main. M3.11's campaign proves the merged build once. Later, the main-branch check guards only the pipelines its references draw, which are Direct's, and a culled or bucket pipeline that lost the correction shows only as some module's exact comparison varying under load, which stops that module. The other choice is a campaign on main on a schedule, at 45 to 90 minutes of loaded GPU each.
- Assumption: `CULLING` takes no quiet-machine sitting. Gate G3c asks for costs "as tripwires", so M3.3's and M3.13's rows are paired, taken once while other builders work, and recorded with their load. The other choice is one sitting with the other builders paused at the end of M3, as `ORCHESTRATE.md` offers a module's last milestone; it gives uncontended figures and costs every builder that pause.

## M1 On the mixed-mesh field, Occlusion draws only what Direct shows, pixel for pixel: done
[review](../../evidence/CULLING/M1-review.md)

## M2 The game picks its culling mode from the device and the measurements, and says which mode ran: done
[review](../../evidence/CULLING/M2-review.md)

## M3 The valley's 10,000 entities are culled by default and match the unculled image pixel for pixel, on Mac and in Chrome

Criteria: M3.1–M3.13 in the done-when file. Read [One image under load](DESIGN.md#m3-one-image-under-load),
through [how it lands](DESIGN.md#how-it-lands), and
[the module list's ruling on M3](../ARCHITECTURE.md#ruling-on-culling-m3) first.

The work is on branch `build/culling`, unmerged, and `docs/evidence/CULLING/M3-handoff.md` on that branch says
what was finished. Where the handoff says a proof must follow a "qualified first landing", or be made again
for the rebased build, this plan replaces it. The branch holds the coverage output, thirty regenerated native
files and four Chrome manifests, their pins and refreshes (`8894b9e77`), the campaign and the `verify --full`
that passed on `cfecd0c7` (`M3/coverage-qualification/passed-cfecd0c7/`), the starting verifier record on main
`c2b16594`, and M3.12's inspected difference images with a builder's pass. All of that stands, except that
the branch's campaign is no longer M3.11's proof. When this note was written the branch's registered
references were byte-equal to `cfecd0c7`'s, and main had moved none since the branch's base, so the next step
is `merge` and nothing is proved first.

M3.11's campaign is taken on main, after the merge, on a build of the merged commit:
[the design](DESIGN.md#what-a-campaign-stands-for) says why and what it stands for. It differs from the
branch's in two ways: its turns also alternate the entity route (`THRUM_VALLEY_MESHES` off and on), and it
stops after sixteen pairs. Take it when you expect to merge no more code, because a later merge of yours
under `crates/` or `examples/v1-game/` costs it again. Start it yourself, as one job.

Before merging, drop the landing pipeline; [the design](DESIGN.md#no-landing-pipeline) says why. Delete from
the branch, under `docs/evidence/CULLING/M3/`, the whole of `landing-queue/` and, in
`coverage-qualification/`, `resume_pipeline.py`, `after_jobs.py`, `after_landing.py`, `post_landing.py`,
`qualified_landing.py` with its copies under `passed-cfecd0c7/`, `check_qualified_landing.py`,
`qualify_after_refresh.py` and `refresh_after_captures.py`. Delete their scratch twins in `tmp/culling-m3/`
too, with `session2_post.py`, `session3_post.py`, `session4_post.py`, `finish_landing.py`,
`qualified_landing_chrome.py` and `queue_safe_loop.py`. In `campaign.py`, take out the fetch, rebase and
landing steps. Stop any
background job that waits on one of these, and remove any pre-push hook you installed. The records and logs
those jobs left are history and may stay.

After the merge the script runs every check on main's tip. Run each remaining check once yourself, take
M3.3's and M3.13's rows once, and make one capture pass for M3.7 and M3.8 after your last merge.
[The failed review](../../evidence/CULLING/M3-review.md) found M3.3 open: star visibility selected Direct for
the cooked valley, and the legacy cost benchmarks measured Direct twice. `scripts/apple_tile_m0.py` made the
Mac reference set, and the verifier archives each Chrome run's manifest. The registry is `BASELINE-GUARD`'s:
change only the `pin` and the `refresh` of the baselines whose references moved.

`APPLE-TILE` M10 and `ATMOSPHERE` M9 also land covered changes. If `merge`'s rebase changes or conflicts with
a registered reference, [the design](DESIGN.md#how-it-lands) names what is repeated, once; otherwise nothing
is. If the main-branch check then fails at your commit, fix it as your next task. If a reference moves where
M3.12 says none may, merge nothing and end `BLOCKED` with the payloads. If the build with the output differs
in the campaign on main, end `BLOCKED` with the payloads and revert nothing yourself.

M3.13's rows never stop the milestone or the merge. If they cross a tripwire, investigate the cause and record
a disposition, as the design's Cost paragraph says, and keep building.

Stop rules, for every milestone:
- If a mode differs from its oracle in any pixel, stop and diagnose. Never widen a comparison, sample fewer frames or change the oracle to pass.
- If your change fails an exact test or baseline another module owns, do not edit it. Write the proposal in the log and end `BLOCKED`. The one exception is the refresh the module list rules for M3's coverage output, with its pins.
- Prove a thing once on your branch and merge. The script's run on the merged tip is the proof of record; run again only the check it says failed, and repeat no proof for a rebase unless the design says so. M3.11's campaign is the one proof you take on main. `verify` passes from the run on main: set it `pass` with the evidence `main-branch check exact-images`, and never run plain `verify` yourself or under a lock.
- Write no job that waits for, asserts or follows another job, a merge or a verifier run, and no hook or helper around `merge`. Start one command, collect it, and decide the next step yourself.
- A default mode changes only on the recorded rows the design's rule names. No speed claim rests on one run or on an uncontended figure that does not exist.
- Chrome and simulator checks need a browser or simulator launch outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
