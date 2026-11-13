# CONTENT-LOOP plan

Closes gate G5 and serves sea, clouds and 2D: edited meshes, textures and volumes appear in the running game
without a stall. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

The loop itself was built before this plan; [the design](DESIGN.md#purpose-and-scope) lists what exists with its
evidence. Read [Shared files and exact baselines](DESIGN.md#shared-files-and-exact-baselines),
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any milestone.

## Open for the owner

- Assumption: leases (M1) are built now, ahead of the owner's decision on when the sea and cloud port starts; the port decision says to keep the foundation converging on readiness, and no sea or cloud pass is built here.
- Assumption: "edited meshes" includes meshes the instance field's entities draw, so an edit replaces them live (M2) instead of asking for a restart. The narrower reading drops M2's live replacement and keeps only cooked meshes as rows at launch.
- Assumption: entities drawn from cooked mesh rows keep the scene's material table. Cooked textures reach placements and leases, not entity materials, so the exit scene's 10,000 entities are coloured by material as today.
- Assumption: "without a stall" is judged by bounds asserted in the game, the existing tripwires against their load-class history and the game's own frame records, not by a frame-time budget (brief decision 3) and not by panel cadence, which the Mac qualification owns.
- Assumption: in Chrome, "without a stall" is claimed for products of the size the valley and a twelve-volume cloud library use. The browser client verifies and stages on the page's one thread; M3 records the size at which a frame is first lost, and a client that works off that thread is built only if the record shows a need.
- Assumption: gate G5 closes after TAAU (M3 waits for `APPLE-PRESENTATION:M3`), so an edit is judged once on the image the exit shows. The other choice closes G5 earlier and leaves the reconstructed look of an edit to `SHOWCASE`.
- Assumption: G5 closes with CAS garbage collection, layered EAC volumes and parameter files as a product kind still parked, and with any iPhone volume encoding owed; the ledger names each.
- Assumption: G5 closes with watcher publications as presentation-only previews. Build provenance in the manifest waits for source admission (T1), as the G5 decision recommends.

## M1 A second engine module reads cooked volumes and textures in its own passes and follows edits to them, on Mac and in Chrome

Criteria: M1.1–M1.9 in the done-when file.

- [ ] Add leases to residency: names fixed in its configuration, made resident through the same loader, staging ring and budget as placements, and offered as a whole set only after the frame that recorded the last copy is submitted. Carry identity per product and report each change once.
- [ ] Build a diagnostic consumer in `examples/v1-game`, off by default, that holds only a lease and reads the twelve generated volumes and one texture in its own pass. Prove texels, the commit rule, an edit, both profiles, the shared budget and device replacement on the Mac.
- [ ] Add the fault probe to staging and the test of the fill-failure path; test a generation with a missing, invalid or refused leased product.
- [ ] Add the leased scenario to the `cooked_edit` bench and record it with its load class.
- [ ] Add the Chrome case; document the lease for `SEA`, `CLOUDS` and `RENDER-2D`.

## M2 An edited mesh changes on every entity that uses it while the game runs, with no pixel different from a fresh start, on Mac and in Chrome

Criteria: M2.1–M2.8 in the done-when file.

- [ ] Convert cooked mesh products to row geometry off the render thread, with the arena's admission, and let a scene's mesh selection be built from a store's current generation.
- [ ] Under [the seam ruling](../ARCHITECTURE.md#interfaces), add row replacement to the route: a bounded write into arena space no drawn row uses, one switch of every reader of the row at a frame boundary, a history-reset frame, reuse of retired space, and the retained scene following.
- [ ] Give the valley's entities cooked mesh rows behind the existing opt-in selection, on the Mac and in Chrome.
- [ ] Prove equality with a fresh scene in Direct, Frustum and Occlusion through an upload, a discarded switch frame, frames in flight, a refusal, a resize and device replacement.
- [ ] Measure render-thread time for the row write and record the bound; capture the two edits for the reviewer.

## M3 Edits to meshes, textures and volumes appear in the running valley without a hitch, on Mac and in Chrome, and the content gate closes

Criteria: M3.1–M3.10 in the done-when file.

- [ ] Run one edit of each kind in the running valley with TAAU, on the Mac and in Chrome: record frame records with capture off, then capture each swap for the reviewer.
- [ ] Add the Chrome case with its interval record, and find the product size at which Chrome first loses a frame.
- [ ] Rerun the G5 tripwires and the shipping-link scan on the commit to be claimed.
- [ ] List the test behind each bad save, and add any that is missing.
- [ ] Bring the maturity ledger's G5 row, the G5 decision's capability table and open items, and the public documentation up to date.

Stop rules, for every milestone:
- A scene with no lease and no cooked rows must compile to the same passes and draw the same bytes. If it does not, stop and fix that first.
- If your change fails an exact test, a baseline or a witness another module owns (`APPLE-TILE`'s baselines, `CULLING`'s equalities, `APPLE-PRESENTATION`'s temporal witness), do not edit it. Write the proposal in the log and end `BLOCKED`.
- In M2, if a live-edited scene differs from a fresh one in any pixel, stop and diagnose. Never widen the comparison or change the oracle.
- A consumer never gets its own loader, store access or upload path. If a lease cannot serve a consumer, write the proposal in the log.
- Run the verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
- A tripwire that fails is investigated. Never re-record history to make it pass, and record only runs of accepted code.
- Lift nothing from Broadsides. Fixtures are generated here.
- Chrome checks need a browser launch outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
