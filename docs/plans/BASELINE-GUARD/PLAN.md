# BASELINE-GUARD plan

A change that alters another module's exact reference images is found on main within about two hours and traced
to the commit that made it. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria ·
[log](LOG.md) · [the rules](../ARCHITECTURE.md#exact-baselines)

## Open for the human

- Assumption: the key-narrowing work of the earlier WP2, nine commits on the branch `build/baseline-guard`, is set aside and not merged. No proof is asked before a merge now, so a proof that survives more rebases saves nothing. The branch is kept as the record.
- Assumption: the run on main may start every 30 minutes, not every hour as the lead's task said. The script waits 55 minutes for a claim's check, a run takes 16, and a claim that arrives just after a start waits for the next one. At one hour about a third of claims would fail as pending. The lead's hourly `main-check --due` is still what starts a run when no claim asks.
- Assumption: the run on main holds `chrome` as well as `gpu` (the task said `gpu`), because it drives Chrome and the settings give Chrome one run at a time. Chrome items can wait up to 16 minutes behind it.
- Assumption: `simulator` is declared an exclusive resource, and the 15 `ios-sim` items moved to it from `gpu`. The owner approved `gpu: 2, chrome: 1`; with `gpu` shared, two simulator checks could otherwise run on the one simulator at once.
- Assumption: when the run on main names a first failing commit and no fix has merged an hour later, the lead reverts that commit, as the approved report says, unless the failure log shows a cause outside the repository.
- Assumption: no builder is paused for a timing. Every timing is a tripwire taken under load, in a module's last work package too, and figures for a quiet machine wait for one window batched for every module, if they are still wanted. Only a sitting a design asks for by name, as `SHOWCASE` WP8's for stutter at 1x, pauses the others. From 15 to 18 November every last work package asked for a paused sitting, and `CULLING` WP3 and `SCENE-VIEWS` WP3 each stopped on it.
- Assumption: the guard stays its own module, so it does not sit on `APPLE-TILE`'s critical path.
- Assumption: macOS, the GPU driver, Chrome, Node and `wasm-bindgen` stay outside what any check pins. An update that moves an image shows as main failing, and the script may then name the first commit after the last pass, so the name is a lead for whoever reads the failure log.
- Assumption: the guard pins the files that choose the comparisons and decide equality, listed in the design, and not the PNG decoder they call from `scripts/run_v1_showcase.py`. `SHOWCASE`'s file changed in 19 commits in two weeks, and pinning it would ask a refresh record of each. The design lists it under what the guard does not stop.
- Assumption: the exact images keep being drawn from the build with `proving` on, on the Mac and in Chrome, because the capture page exists only there. No exact baseline draws the normal bundle, the one without test hooks. `GAME-SPLIT` WP3.3 checks what that bundle ships, not its pixels.
- Assumption: `GAME-SPLIT` WP3 is not held for the wrong Wasm hashes in the verifier's record. Its builder may not change the verifier, the fault moved no image, and the fix is WP3 here. The other choice keeps `GAME-SPLIT` WP3 failing review until WP3 here has merged.

## WP1 A change that alters a registered exact image, on the Mac or in Chrome, is refused before it reaches main: done
[review](../../../.orchestrate/evidence/BASELINE-GUARD/WP1-review.md)

## WP2 A commit that changes a registered reference image or its comparison is refused at the merge unless it records the refresh: done
[review](../../../.orchestrate/evidence/BASELINE-GUARD/WP2-review.md)

## WP3 A passing exact-image record names the Wasm file that Chrome ran, and a run whose capture ran another fails

Read [the Wasm a Chrome capture runs](DESIGN.md#the-wasm-a-chrome-capture-runs) and
[the ruling](../ARCHITECTURE.md#ruling-on-the-wasm-a-chrome-capture-runs), which says what you may change in
`T2-CAPTURE`'s capture script and test and that nothing else of another module's changes.

`GAME-SPLIT`'s builder left [a draft](../../../.orchestrate/evidence/GAME-SPLIT/WP3/round-3/exact-receipt-proposal.patch)
of this change. It is evidence, written before this plan: it ran only against stand-ins, never in Chrome, and it
puts its test in `T2-CAPTURE`'s file where WP3.1 names this module's. Use what serves the items.
`build.sh --proving`, the capture's `--web-root` and `scripts/wasm_receipt.mjs` exist on main.

The verifier is a comparison file of all five baselines, so its edit moves every pin. Commit the edit, copy the
pins the guard prints, set each refresh (the registry's field for the work package is still named `milestone`)
and commit, and only then make WP3.2's run, on that clean commit. That run is the one real proof that the capture
page yields a hash, and a fault in the verifier would otherwise first show as `exact-images` failing on main for
every module. Take the `gpu` lock for it with the script's `lock` command. The lock's hold limit is 30 minutes,
and on 24 and 25 November 14 passing runs on main took 15 to 28 minutes with their builds warm, so build the
release game, the cooker and the proving bundle before you take the lock. If the limit still stops the run, say so
in your report; do not shorten or split the run. Copy the run's `record.json` and its logs to
`.orchestrate/evidence/BASELINE-GUARD/WP3/`. After a rebase, make the run again only if the rebase changed the
verifier or the capture script. A failure in that run that is not yours is judged by the rulings on
[a frame that varies under load](../ARCHITECTURE.md#a-frame-that-varies-under-load) and
[a fixed capture that stalls](../ARCHITECTURE.md#a-fixed-capture-that-stalls).
