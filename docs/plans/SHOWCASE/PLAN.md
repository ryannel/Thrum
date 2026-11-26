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
- Assumption: the exit's "smooth at the display's full refresh" holds at full screen on the built-in display, so WP8 also waits for `APPLE-PRESENTATION` WP17, which holds the full 120 Hz there, and WP8.12 lists it. WP17 proves it on the bare valley, and WP8.6 on the populated exit flythrough. WP7 needs only WP11, where full screen presents at a steady 60 Hz as an interim rate. The owner can take WP17 out of the exit, which closes V1 at 60 Hz at full screen.
- Assumption: holding 120 Hz at full screen is claimed for the Mac's Apple recipe, the row WP17 brings to that rate. The portable recipe is the fallback: WP8.14 records its full-screen flythrough, and the exit does not wait for it to hold 120 Hz. The other choice makes that an exit condition, with work no module plans.
- Assumption: WP5 and WP6 start as soon as cooked mesh rows (`CONTENT-LOOP:WP2`) and culling (`CULLING:WP3`) are Done, so the scene is built while the other modules finish; WP5 closes after `ATMOSPHERE:WP10`, the shadow fix its review needs. WP7 waits for the work packages it builds on (`APPLE-TILE` WP12, `APPLE-PRESENTATION` WP11, `ATMOSPHERE` WP6 and `CONTENT-LOOP` WP3), and only WP8 for every work package the exit rests on, which WP8.12 lists. The other choice holds all four until then.
- Assumption: "the same scene in Chrome" means the same cooked publication, layout, route and controls. Texture variant, reconstructor and display range differ by row, and the exit record says how.
- Assumption: prop detail is sized to fit the Mac's 120 Hz refresh interval at 1920×1080. Chrome's frame time is recorded and sizes nothing.
- Assumption: before WP8 the movers' motion is judged tick by tick, from captures of every tick, and real-time takes are a tripwire record labelled with the machine's load (the lead's reading of the owner's bar of 17 November). Stutter at 1x is judged in WP8, with the other builders paused. A late frame in WP7's records that the load makes impossible to explain is listed as unresolved and answered in that WP8 sitting. The other choice judges real-time takes in WP5 and wants a cause for every late frame in WP7, and each waits for a quiet machine.
- Assumption: the exit scene is not run on the iPhone simulator, which gate G7 does not name. The exit record reports the simulator's status from the modules that proved it.
- Assumption: the valley's terrain changes once, in WP9, to take a bright band off the lookout's rim at sunset, which `ATMOSPHERE` WP6 could not pass with. Every registered exact image set is refreshed in that landing, and the two route baselines are calibrated again. The other choice closes gate G3s with the band named as a known defect and leaves it for WP8's review to find.
- Assumption: the sunrise stop moves later than 6.1 h, to the first hour the Sun reaches the lookout, because until then the props stand in the east ridge's shade and cast no shadow. The page's Sunrise button keeps 6.1 h. The other choice lowers the east ridge and keeps the hour.

## WP5 The valley is populated: 10,000 props drawn from cooked meshes, 1,500 of them adrift, on Mac and in Chrome

The scene, layout, mover rule, route and checks are built and on main (`1556e66d0`), and WP5.1 to WP5.7 and WP5.10
passed [the first review](../../../.orchestrate/evidence/SHOWCASE/WP5-review.md). Claim those again from their evidence, renewing
only what has changed since. What is left is motion and, if a stop moved, the exit baseline: read
[Motion on a shared machine](DESIGN.md#motion-on-a-shared-machine) first.
[The handoff](../../../.orchestrate/evidence/SHOWCASE/WP5-handoff.md) is history; its retakes are WP5.11.

The launcher gains three things. `ticks` drives the native route recorder for WP5.8, on the flythrough or the
held route, with the windows and the phase its caller names, so that a reviewer takes its own. `motion` takes
WP5.11's six arms and writes `tests/fixtures/showcase/motion-record.json`; `motion --check` reads that file,
computes the input digest and starts nothing. Take the arms last, on a branch that holds main, and land the
record with nothing else. If `check --exit` reports drift from the exit baseline at a stop, look at the frames,
then add and use [the refresh](DESIGN.md#the-route-check-sees-a-missing-prop).

## WP6 The populated valley is culled on the GPU with no pixel changed, on Mac and in Chrome

`CULLING` owns the equality command, its comparisons and the rule that sets the default. This work package runs
them on the exit scene and changes none of them. If equality fails, stop and diagnose.

## WP9 From the lookout at sunset the ground runs to its rim with no bright band along it, and at the sunrise stop the sphere and the rock cast long shadows

WP9 is built and on main (`cd0c011e5`). [Its review](../../../.orchestrate/evidence/SHOWCASE/WP9-review.md) passed
every item except WP9.6, and that item is written again. The landing moved the retained references and this
module's check inputs out of `docs/evidence/` into test fixtures, on the lead's ruling, and WP9.6 froze the
paths that had to follow them. [The relocation ruling](../ARCHITECTURE.md#ruling-on-relocated-references) now
says which paths may. Nothing is known to be left to build. Claim each item again from its evidence in
`.orchestrate/evidence/SHOWCASE/WP9/`, renewing only what has changed on main since, and read the landed diff
against WP9.6 as it now stands before claiming it. Change the terrain, a preset, a reference or the registry only
if a check fails, and then read
[The lookout's rim and the sunrise stop](DESIGN.md#the-lookouts-rim-and-the-sunrise-stop) and
[the valley ruling](../ARCHITECTURE.md#ruling-on-the-valley-fix) first, because everything that moves with the
terrain lands together. Keep this work package's receipts under a name of their own, and WP6's as they are.

## WP7 One command runs the same showcase on the Mac's Apple recipe, on its portable recipe and in Chrome, and each run says exactly what it ran

The exit configuration's entries are taken from the owning modules' closing records. `exit` and `frames` are new
launcher commands. WP7.9 and WP7.10 take their records while other builders work:
[Frame records, startup and hitches](DESIGN.md#frame-records-startup-and-hitches) says what a late frame's
investigation may end in, and each entry carried to WP8.13 keeps its row and its place. If WP7.1 fails, end
`BLOCKED needs_dependency` at once.

## WP8 The showcase passes review on the Mac and in Chrome, its motion on the built-in display is on record, and the V1 exit record is written

The order is part of this work package. First do everything that needs neither a quiet machine nor the display.
Then end `BLOCKED` asking the lead for the sitting with the other builders paused, as the project settings say.
In that one sitting take the real-time captures of WP8.2, WP8.3 and WP8.5, the three attended records of WP8.6,
WP8.7 and WP8.14 on the unlocked built-in display, and what WP8.13's carried entries need. Read
[the full refresh at full screen](DESIGN.md#the-full-refresh-at-full-screen-on-the-populated-exit) before WP8.6's
two takes. Before the first day-cycle take,
write the storage record that [the design's open questions](DESIGN.md#open-questions) name. Get a fresh opinion
against WP8.1 to WP8.5, fix what it finds, and repeat while it finds things that matter. Last, write
[the exit record](DESIGN.md#the-exit-record) and bring the maturity ledger's G7 rows and the README to match it.
