# SHOWCASE plan

Closes gate G7 and the V1 exit: the exit scene on the Mac's two recipes and in Chrome, reviewed, with its closing
record. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

The launch, the six stops and the route check were built before the loop as milestones M1 to M4
([record](../../evidence/SHOWCASE/ROUTE-SLICE-RECORD.md)), so this plan starts at M5. Read
[Mechanisms](DESIGN.md#mechanisms), [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any milestone.

## Open for the human

- Assumption: the exit closes on the agents' reviews and the records taken on the built-in display, as brief decisions 2 and 4 say, and the owner's own acceptance of the showcase's look is not an item. The other choice adds one item to M8 that waits for the owner to walk the route and say yes.
- Assumption: the 8,500 still entities are desert props (boulders, scrub, cacti and standing rocks) and the 1,500 movers are hot-air balloons adrift over the valley. No entity is a character or an animal, because an entity cannot turn or animate in V1. The layout is generated data, so another concept that only translates swaps in by regenerating.
- Assumption: the exit scene holds exactly 10,000 entities and 1,500 movers, the counts the game already steps, inside the brief's "1,000–2,000 movers".
- Assumption: "MetalFX, EDR and a quality governor" stay in the exit, so M7 and M8 wait for `APPLE-PRESENTATION` M4 to M11, which are drafted but have no done-when entries yet. Closing V1 without them would change the exit, which is the owner's call.
- Assumption: M5 and M6 start as soon as cooked mesh rows (`CONTENT-LOOP:M2`) and culling (`CULLING:M3`) are Done, so the scene is built while the other modules finish. Only M7 and M8 wait for every module's last milestone. The other choice holds all four until then.
- Assumption: "the same scene in Chrome" means the same cooked publication, layout, route and controls. Texture variant, reconstructor and display range differ by row, and the exit record says how.
- Assumption: prop detail is sized to fit the Mac's 120 Hz refresh interval at 1920×1080. Chrome's frame time is recorded and sizes nothing.
- Assumption: the exit scene is not run on the iPhone simulator, which gate G7 does not name. The exit record reports the simulator's status from the modules that proved it.

## M5 The valley is populated: 10,000 props drawn from cooked meshes, 1,500 of them adrift, on Mac and in Chrome

Criteria: M5.1–M5.10 in the done-when file.

- [ ] Add the exit scene as a third game content, chosen by name on the Mac and in Chrome, with the grid and field contents and every existing configuration left exactly as they are.
- [ ] Sweep first: give the 10,000 entities stand-in meshes at three or more triangle counts and record whole-frame GPU time by the [sizing rule](DESIGN.md#props-are-cooked-mesh-rows-sized-by-measurement). Choose the prop set from the result.
- [ ] Extend the generator to author the props and the layout: still props on the terrain by a density that falls with distance, movers on closed loops clear of the ground, at most 21 meshes. Keep the valley's source files and cooked root unchanged.
- [ ] Add the one registered rule that moves each mover along its loop, and put the exit scene through the corrected-run replay test and a simulated hour.
- [ ] Draw the entities as cooked mesh rows on the bucket route, with the scene's default mode, on the Mac and in Chrome.
- [ ] Author the exit flythrough route. Add `layout` and `check --exit` to the launcher, with the no-entities control, thresholds set by the fixed rule and the exit baseline file.
- [ ] Capture the stops and the flythrough on both targets and get a fresh opinion on the placement and the movers before claiming.

## M6 The populated valley is culled on the GPU with no pixel changed, on Mac and in Chrome

Criteria: M6.1–M6.6 in the done-when file.

- [ ] Run `CULLING`'s equality command on the exit scene and flythrough: Frustum and Occlusion against bucket-ordered Direct, on both native recipes and in Chrome, with the movers moving.
- [ ] Record paired whole-GPU rows on the flythrough in two runs and set the exit scene's default by `CULLING`'s rule. Report the mode on both targets.
- [ ] Capture the flythrough and a fast yaw in real time on both targets for the reviewer.

## M7 One command runs the same showcase on the Mac's Apple recipe, on its portable recipe and in Chrome, and each run says exactly what it ran

Criteria: M7.1–M7.11 in the done-when file.

- [ ] Write the exit configuration: one file with every selection each row makes and the value the game must report for it, taken from the owning modules' closing records. Make `mac`, `chrome` and every check read it.
- [ ] Add the game's report of what it resolved, natively and on the page, and the `exit --row` command that compares it with the file. Show that a removed selection and a fallback each fail.
- [ ] Compare the three rows at the stops, set thresholds by the fixed rule and keep one baseline per row.
- [ ] Prove the exit's clauses in the exit scene on each target: time controls, live edits, the profiler and no pipeline creation in play. Prove the controls are the same set on both targets.
- [ ] Record whole frames with capture off over the flythrough and a day cycle on each row, and the startup proxies. List every late frame and find its cause.

## M8 The showcase passes review on the Mac and in Chrome, its motion on the built-in display is on record, and the V1 exit record is written

Criteria: M8.1–M8.11 in the done-when file.

- [ ] Capture stops and motion on all three rows from the exit configuration. Get a fresh opinion against M8.1–M8.5, fix what it finds, and repeat while it finds things that matter.
- [ ] Do everything else, then the two attended items in one sitting: presented intervals and windowed startup on both recipes, then Chrome's visible window.
- [ ] Write [the exit record](DESIGN.md#the-exit-record) with the visual digest, platform status, known defects and device runs owed.
- [ ] Bring the maturity ledger's G7 rows and the README to match the record.

Stop rules, for every milestone:
- With the exit scene unselected, every existing configuration must prepare, step and draw exactly as before. If one does not, stop and fix that first.
- If a change fails an exact test, a baseline or a witness another module owns, do not edit it. Write the proposal in the log and end `BLOCKED`.
- A defect in another module's code is fixed in place only when the fix keeps that module's design, tests and exact comparisons true, and is noted in both logs. Otherwise write the proposal and end `BLOCKED needs_dependency`.
- The game gets no height function. Anything that needs the ground's height is decided in the generator.
- A mover that needs to turn, a prop that needs a texture or a level of detail is a proposal in the log, not work here.
- In M6, change no culling code or comparison. If equality fails, stop and diagnose; never widen a comparison.
- In M7, if M7.1 fails because `APPLE-PRESENTATION` has no M11 or it is not Done, end `BLOCKED needs_dependency` at once.
- Never tune a parity threshold. If a control does not fail as it must, the metric goes back to the design.
- A tripwire that trips is investigated. Record only runs of the commit being claimed, with their load.
- Run the verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
- Chrome and Instruments start outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute another target's result.
