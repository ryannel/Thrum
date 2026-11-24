# CONTENT-LOOP plan

Closes gate G5 and serves sea, clouds and 2D: edited meshes, textures and volumes appear in the running game
without a stall. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

Work packages run in the order below.

Read [Shared files and exact baselines](DESIGN.md#shared-files-and-exact-baselines),
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any work package. Follow the [builder rules](DESIGN.md#builder-rules).

## Open for the owner

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: leases (WP1) are built now, ahead of the owner's decision on when the sea and cloud port starts; the port decision says to keep the foundation converging on readiness, and no sea or cloud pass is built here.
- Assumption: "edited meshes" includes meshes the instance field's entities draw, so an edit replaces them live (WP2) instead of asking for a restart. The narrower reading drops WP2's live replacement and keeps only cooked meshes as rows at launch.
- Assumption: entities drawn from cooked mesh rows keep the scene's material table. Cooked textures reach placements and leases, not entity materials, so the exit scene's 10,000 entities are coloured by material as today.
- Assumption: "without a stall" is judged by bounds asserted in the game, the existing tripwires against their load-class history and the game's own frame records, not by a frame-time budget (brief decision 3) and not by panel cadence, which the Mac qualification owns.
- Assumption: in Chrome, "without a stall" is claimed for products of the size the valley and a twelve-volume cloud library use. The browser client verifies and stages on the page's one thread; WP3 records the size at which a frame is first lost, and a client that works off that thread is built only if the record shows a need.
- Assumption: gate G5 closes after TAAU (WP3 waits for `APPLE-PRESENTATION:WP3`), so an edit is judged once on the image the exit shows. The other choice closes G5 earlier and leaves the reconstructed look of an edit to `SHOWCASE`.
- Assumption: G5 closes with CAS garbage collection, layered EAC volumes and parameter files as a product kind still parked, and with any iPhone volume encoding owed; the ledger names each.
- Assumption: G5 closes with watcher publications as presentation-only previews, which the G5 decision put in force as the reading of C05. A save the cooker can tell is bad keeps the last good content, but an incomplete save that still validates (a changed glTF saved before its buffer) is shown until the save finishes, then replaced by the intended content. Refusing it needs source admission (T1), which also brings build provenance and is not built here.
- Assumption: when the running game cannot take one product of a save (a mesh too large for the entity arena, a volume the device cannot hold), nothing from that save or a later one appears until it is fixed, and the reason is named. The other choice shows the rest of the save and holds back only that product.

## WP1 A second engine module reads cooked volumes and textures in its own passes and follows edits to them, on Mac and in Chrome: done

[review](../../../.orchestrate/evidence/CONTENT-LOOP/WP1-review.md)

## WP4 A saved source is always cooked and published, even when macOS reports the save late or never, so the full check stops failing on a busy Mac

[Review](../../../.orchestrate/evidence/CONTENT-LOOP/WP4-review.md)

## WP2 An edited mesh changes on every entity that uses it while the game runs, with no pixel different from a fresh start, on Mac and in Chrome

[Review](../../../.orchestrate/evidence/CONTENT-LOOP/WP2-review.md)

## WP3 Edits to meshes, textures and volumes appear in the running valley without a hitch, on Mac and in Chrome, and the content gate closes

[Review](../../../.orchestrate/evidence/CONTENT-LOOP/WP3-review.md)

## WP5 Small cooked publications prove supersession and safe reuse

Use the [fixture design](DESIGN.md#cooked-publication-regressions-m5) for the cases in the
[workspace case list and baseline times](LOG.md#m5-workspace-timing-baseline).

Record each case’s disposition, any replacement coverage and its before-and-after time in the workspace
check’s default Nextest build on the Mac, excluding compilation. Measure the fast cases alone and in a
focused batch under normal builder load. The batch passes the existing ten-second limit on its first run.
