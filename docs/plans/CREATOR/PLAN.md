# CREATOR plan

Closes gate G6. [Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

## Open for the owner

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: the starter gets a Chrome form now. The G6 decision left it as an owner question and recommended deferring it to G7's page; website demos, and the sea and cloud demo scenes, need a creator's scene in Chrome.
- Assumption: "profiler" in the V1 exit is met by readouts in the demo UI on both targets, fed by the frame host's completed samples and the shells' callback records, not by a separate tool.
- Assumption: the profiler is built after `APPLE-TILE:M3` and not beside it, so it waits on that module's attended items. The other order has the readouts built now on the positional GPU slots, with the switch added later.
- Assumption: the profiler shows encode and submit as one row, and the frame-callback interval in place of a display interval, because those are what the engine measures. Physical cadence stays with the Mac qualification.
- Assumption: the profiler toggle is off by default, so a kit scene and the V1 window run untimed unless the profiler is on or a frames report was asked for.
- Assumption: a fresh agent's record stays valid after the public documents are edited to remove friction it reported, so the 17 November ziggurat run on `e5de7027e70b` stands for M3.1; a changed command, file or key on the creator path needs a new run.
- Assumption: G6 closes without the clipping overlay, the shadow-cascade overlay and parameter files as a cooked product, which the G6 decision holds or parks.

## M1 The starter scene runs in Chrome from one command, with its controls and live asset edits: done

[review](../../../.orchestrate/evidence/CREATOR/M1-review.md)

## M2 The demo UI shows where a frame's time goes on Mac and in Chrome: done

[review](../../../.orchestrate/evidence/CREATOR/M2-review.md)

## M3 A fresh agent builds a new scene for Mac and Chrome from the public docs alone, and G6 closes

The product, the documents and evidence for all five items exist, and no review found a defect in them
([review 4](../../../.orchestrate/evidence/CREATOR/M3-review.md)). The 17 November re-plan changed only how M3.1 and M3.4 are
proved. It also cleared every item's proof, so claim each item again on the current tip, and change the product
or the documents only where an item is untrue.

M3.1's fresh agent is the lead's, never you or a subagent of yours. Cite the lead-commissioned ziggurat record in
`.orchestrate/evidence/CREATOR/m3/fresh-lead/` and leave its files as they are. `fresh-session/` and `fresh-refresh/`
beside it hold the folded-sail exercise an earlier builder arranged, which does not count for this item. If you
change a command, file or key on the creator path, end `BLOCKED needs_context` and ask the lead for a new run
from [the brief](../../../.orchestrate/evidence/CREATOR/M3-fresh-agent-brief.md).

M3.4 runs through the window driver in `.orchestrate/evidence/CREATOR/m3/`, on the unlocked, awake built-in display.
Batch its setup with the Mac qualification session.

**For the lead.** M3's reviewer no longer has to be fresh, so start its review like any other. Commission a new
fresh run only when the builder or a review asks for one.

## M4 Small hot-reload checks prove which material the device draws

Record each original test’s disposition and map its assertions to the retained or replacement tests by
nextest name, including any CPU cases. Use the [design’s prefixes](DESIGN.md#fast-material-hook-regressions)
to include those cases in M4.1. Record the supplied starting times and each revised case’s solo and batch
times on the Mac, excluding compilation. For the batch, run M4.1 alongside normal builder work and record the
overlapping build or test activity and the nextest results. Every mapped test passes on the first attempt
under the workspace’s default scheduling and ten-second limit.

These are the cases from the passing `747909d1c` workspace check, with seconds from that run.

- 9.973 s — `v1-game::it cooked_material_gpu::requests_before_the_first_frame_wait_for_the_scene_and_the_newest_draws`
- 6.403 s — `v1-game::it cooked_material_gpu::a_replaced_device_draws_the_last_accepted_hook_and_retires_the_outstanding_reload`
- 6.228 s — `v1-game::it cooked_material_gpu::a_hook_that_writes_non_finite_factors_draws_the_placement_material`
