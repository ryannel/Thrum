# SHOWCASE plan

Closes gate G7 and the V1 exit: the exit scene on the Mac's two recipes and in Chrome, reviewed, with its closing
record. Work packages run in the order WP5, WP6, WP9, WP7, WP8. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

The launch, the six stops and the route check were built before the loop as work packages M1 to M4
([record](../../../.orchestrate/evidence/SHOWCASE/ROUTE-SLICE-RECORD.md)), so this plan starts at WP5. Read
[Mechanisms](DESIGN.md#mechanisms), [Limits for every work package](DESIGN.md#limits-for-every-work-package),
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any work package.

## Open for the human

- Assumption: the exit closes on the agents' reviews and the records taken on the built-in display, as brief decisions 2 and 4 say, and the owner's own acceptance of the showcase's look is not an item. The other choice adds one item to WP8 that waits for the owner to walk the route and say yes.
- Assumption: the 8,500 still entities are desert props (boulders, scrub, cacti and standing rocks) and the 1,500 movers are hot-air balloons adrift over the valley. No entity is a character or an animal, because an entity cannot turn or animate in V1. The layout is generated data, so another concept that only translates swaps in by regenerating.
- Assumption: the exit scene holds exactly 10,000 entities and 1,500 movers, the counts the game already steps, inside the brief's "1,000–2,000 movers".
- Assumption: "MetalFX, EDR and a quality governor" stay in the exit, so WP7 and WP8 wait for `APPLE-PRESENTATION`'s paired proof, WP11, and through it for WP4 to WP12. Closing V1 without them would change the exit, which is the owner's call.
- Assumption: holding 120 Hz at full screen is claimed for the Mac's Apple recipe, the row `APPLE-PRESENTATION` WP17 brings to that rate. The portable recipe is the fallback: it resolves the same output scale, WP8.14 records its full-screen flythrough, and the exit does not wait for it to hold 120 Hz. The other choice makes that an exit condition, with work no module plans.
- Assumption: WP5 and WP6 start as soon as cooked mesh rows (`CONTENT-LOOP:WP2`) and culling (`CULLING:WP3`) are Done, so the scene is built while the other modules finish; WP5 closes after `ATMOSPHERE:WP10`, the shadow fix its review needs. WP7 waits for the work packages it builds on (`APPLE-TILE` WP12, `APPLE-PRESENTATION` WP11, `ATMOSPHERE` WP6 and `CONTENT-LOOP` WP3 and WP6), and only WP8 for every work package the exit rests on, which WP8.12 lists. The other choice holds all four until then.
- Assumption: "the same scene in Chrome" means the same cooked publication, layout, route and controls. Texture variant, reconstructor and display range differ by row, and the exit record says how.
- Assumption: the three rows are held to showing the same scene and looking alike, not to the same pixels (the lead's ruling of 26 November 2026, on the owner's bar of 17 November). Between rows the median grid cell stays within one 8-bit code, and edges and fine detail may differ by three times what the reconstructors were measured to differ by at that stop. Each row is also held to its own baseline at one code. The other choice corrects MetalFX and TAAU until every row fits one code.
- Assumption: prop detail is sized to fit the Mac's 120 Hz refresh interval at 1920×1080. Chrome's frame time is recorded and sizes nothing.
- Assumption: before WP8 the movers' motion is judged tick by tick, from captures of every tick, and real-time takes are a tripwire record labelled with the machine's load (the lead's reading of the owner's bar of 17 November). Stutter at 1x is judged in WP8, with the other builders paused. A late frame in WP7's records that the load makes impossible to explain is listed as unresolved and answered in that WP8 sitting. The other choice judges real-time takes in WP5 and wants a cause for every late frame in WP7, and each waits for a quiet machine.
- Assumption: the exit scene is not run on the iPhone simulator, which gate G7 does not name. The exit record reports the simulator's status from the modules that proved it.
- Assumption: the valley's terrain changes once, in WP9, to take a bright band off the lookout's rim at sunset, which `ATMOSPHERE` WP6 could not pass with. Every registered exact image set is refreshed in that landing, and the two route baselines are calibrated again. The other choice closes gate G3s with the band named as a known defect and leaves it for WP8's review to find.
- Assumption: the sunrise stop moves later than 6.1 h, to the first hour the Sun reaches the lookout, because until then the props stand in the east ridge's shade and cast no shadow. The page's Sunrise button keeps 6.1 h. The other choice lowers the east ridge and keeps the hour.

## WP5 The valley is populated: 10,000 props drawn from cooked meshes, 1,500 of them adrift, on Mac and in Chrome: done

[review](../../../.orchestrate/evidence/SHOWCASE/WP5-review.md)

## WP6 The populated valley is culled on the GPU with no pixel changed, on Mac and in Chrome: done

[review](../../../.orchestrate/evidence/SHOWCASE/WP6-review.md)

## WP9 From the lookout at sunset the ground runs to its rim with no bright band along it, and at the sunrise stop the sphere and the rock cast long shadows: done

[review](../../../.orchestrate/evidence/SHOWCASE/WP9-review.md)

## WP7 One command runs the same showcase on the Mac's Apple recipe, on its portable recipe and in Chrome, and each run says exactly what it ran

The exit configuration's entries are taken from the owning modules' closing records. `exit` and `frames` are new
launcher commands. WP7.9 and WP7.10 take their records while other builders work:
[Frame records, startup and hitches](DESIGN.md#frame-records-startup-and-hitches) says what a late frame's
investigation may end in, and each entry carried to WP8.13 keeps its row and its place. If WP7.1 fails, end
`BLOCKED needs_dependency` at once.

It was built to `3714c8495` on `build/showcase` and stopped on two things the plan now answers
([handoff](../../../.orchestrate/evidence/SHOWCASE/WP7-handoff.md)). Resume that branch and claim WP7.6 to
WP7.10 again from their evidence, renewing only what has changed. Two things are left:

- **The cooked root (WP7.2 to WP7.4).** Carry `CONTENT-LOOP` WP6's observation into the native reports and the
  page's cooked status, as
  [the exit configuration](DESIGN.md#the-exit-configuration-one-file-three-rows-reported-and-not-echoed) says,
  then run the three row checks and the fallback control again.
- **The comparison (WP7.5).** Read [what the three rows are held to](DESIGN.md#what-the-three-rows-are-held-to-wp75).
  Do not trace or correct the reconstructor differences. Before keeping the calibration, look at the three
  rows' stills at every stop against [what each stop must show](DESIGN.md#what-each-stop-must-show): a cue all
  three rows lack would be calibrated in. Log the calibration's measured values and the baseline file's hashes.

## WP8 The showcase passes review on the Mac and in Chrome, its motion on the built-in display is on record, and the V1 exit record is written

The order is part of this work package. First do everything that needs neither a quiet machine nor the display.
Then end `BLOCKED` asking the lead for the sitting with the other builders paused, as the project settings say.
In that one sitting take the real-time captures of WP8.2, WP8.3 and WP8.5, the three attended records of WP8.6,
WP8.7 and WP8.14 on the unlocked built-in display, and what WP8.13's carried entries need. Read
[the full refresh at full screen](DESIGN.md#the-full-refresh-at-full-screen-on-the-populated-exit) before WP8.6's
two takes: they run at the output scale `APPLE-PRESENTATION` WP17 promoted, with no launch setting forcing the
scale or the rate, and the launcher's report prints the scale and both extents. Before the first day-cycle take,
write the storage record that [the design's open questions](DESIGN.md#open-questions) name. Get a fresh opinion
against WP8.1 to WP8.5, fix what it finds, and repeat while it finds things that matter. Last, write
[the exit record](DESIGN.md#the-exit-record) and bring the maturity ledger's G7 rows and the README to match it.
