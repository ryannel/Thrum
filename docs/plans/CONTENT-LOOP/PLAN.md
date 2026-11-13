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
- Assumption: G5 closes with watcher publications as presentation-only previews, which the G5 decision put in force as the reading of C05. A save the cooker can tell is bad keeps the last good content, but an incomplete save that still validates (a changed glTF saved before its buffer) is shown until the save finishes, then replaced by the intended content. Refusing it needs source admission (T1), which also brings build provenance and is not built here.
- Assumption: when the running game cannot take one product of a save (a mesh too large for the entity arena, a volume the device cannot hold), nothing from that save or a later one appears until it is fixed, and the reason is named. The other choice shows the rest of the save and holds back only that product.

## M1 A second engine module reads cooked volumes and textures in its own passes and follows edits to them, on Mac and in Chrome

Criteria: M1.1–M1.10 in the done-when file.

- [ ] Add leases to residency: names fixed in its configuration, made resident through the same loader, staging ring and budget as placements, and offered as a whole set only after the frame that recorded the last copy is submitted. Carry identity and a change serial per product, so a consumer keeps a change pending until a submitted frame that advances history has used it.
- [ ] Make the publication residency's, under [one publication, one switch](DESIGN.md#one-publication-one-switch-m1-m2), so that placements and leases switch in the same frame. A scene with only placements behaves as it does today.
- [ ] Build a diagnostic consumer in `examples/v1-game`, off by default, that holds only a lease and reads the twelve generated volumes and one texture in its own pass. Prove texels, the commit rule, an edit, both profiles, the shared budget and device replacement on the Mac.
- [ ] Add the fault probe to staging and the test of the fill-failure path; test a generation with a missing, invalid or refused leased product.
- [ ] Add the leased scenario to the `cooked_edit` bench and record it with its load class.
- [ ] Add the Chrome case; document the lease for `SEA`, `CLOUDS` and `RENDER-2D`.

## M2 An edited mesh changes on every entity that uses it while the game runs, with no pixel different from a fresh start, on Mac and in Chrome

Criteria: M2.1–M2.9 in the done-when file.

- [ ] Convert cooked mesh products to row geometry off the render thread, with the arena's admission, and let a scene's mesh selection be built from a store's current generation.
- [ ] Under [the seam ruling](../ARCHITECTURE.md#interfaces), add row replacement to the route: a bounded write into arena space no drawn row uses, one switch of every reader of the row that commits when its frame is submitted, a history-reset frame, reuse of retired space only after that, and the retained scene following. Everything goes through the frame host.
- [ ] Make rows the publication's third reader, under the same rules: a row switches in the publication's frame, with the placements and leases.
- [ ] Give the valley's entities cooked mesh rows behind the existing opt-in selection, on the Mac and in Chrome.
- [ ] Prove equality with a fresh scene in Direct, Frustum and Occlusion through an upload, a discarded switch frame, a newer edit behind it, a newer edit that supersedes a replacement still being written, a refusal, a resize and device replacement, and for a mesh that is also placed.
- [ ] Measure render-thread time for the row write and record the bound; capture the two edits for the reviewer.

## M3 Edits to meshes, textures and volumes appear in the running valley without a hitch, on Mac and in Chrome, and the content gate closes

Criteria: M3.1–M3.10 in the done-when file.

- [ ] Run the design's three edits in the running valley with TAAU, on the Mac and in Chrome, two of them to a product read through two paths. Add the generation each reader showed to the game's frame record. Record frame records with capture off, then capture each switch for the reviewer.
- [ ] Add the Chrome case with its interval record. A stall this module causes at the valley's or the twelve-volume fixture's sizes is fixed here, by the worker fallback the design names if need be. Then find the larger product size at which Chrome first loses a frame, as a record.
- [ ] Rerun the G5 tripwires and the shipping-link scan on the commit to be claimed.
- [ ] List the test behind each bad save, and add any that is missing. Add the test that an incomplete save which still validates ends at what a fresh cook of the finished sources publishes.
- [ ] Bring the maturity ledger's G5 row, the G5 decision's capability table and open items, and the public documentation up to date, with the preview limit in [what a publication promises](DESIGN.md#what-a-publication-promises-m3) stated as a limit.

Stop rules, for every milestone:
- A scene with no lease and no cooked rows must compile to the same passes and draw the same bytes. If it does not, stop and fix that first.
- If your change fails an exact test, a baseline or a witness another module owns (`APPLE-TILE`'s baselines, `CULLING`'s equalities, `APPLE-PRESENTATION`'s temporal witness), do not edit it. Write the proposal in the log and end `BLOCKED`.
- In M2, if a live-edited scene differs from a fresh one in any pixel, stop and diagnose. Never widen the comparison or change the oracle.
- In M2, never test a row switch by changing the arena or the route around the host's encoded-frame token. If a proof seems to need a frame encoded before a switch and submitted after it, write the proposal in the log.
- A consumer never gets its own loader, store access or upload path. If a lease cannot serve a consumer, write the proposal in the log.
- No reader switches alone. If one switch for every reader cannot be kept, write the proposal in the log and end `BLOCKED`; C05 changes only through the module list.
- Run the verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
- A tripwire that fails is investigated. Never re-record history to make it pass, and record only runs of accepted code.
- Lift nothing from Broadsides. Fixtures are generated here.
- Chrome checks need a browser launch outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
