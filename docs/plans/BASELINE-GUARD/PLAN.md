# BASELINE-GUARD plan

A change that alters another module's exact reference images is found on main within about two hours and traced
to the commit that made it. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria ·
[log](LOG.md) · [the rules](../ARCHITECTURE.md#exact-baselines)

## Open for the human

- Assumption: the key-narrowing work of the earlier M2, nine commits on the branch `build/baseline-guard`, is set aside and not merged. No proof is asked before a merge now, so a proof that survives more rebases saves nothing. The branch is kept as the record.
- Assumption: the run on main may start every 30 minutes, not every hour as the lead's task said. The script waits 55 minutes for a claim's check, a run takes 16, and a claim that arrives just after a start waits for the next one. At one hour about a third of claims would fail as pending. The lead's hourly `main-check --due` is still what starts a run when no claim asks.
- Assumption: the run on main holds `chrome` as well as `gpu` (the task said `gpu`), because it drives Chrome and the settings give Chrome one run at a time. Chrome items can wait up to 16 minutes behind it.
- Assumption: `simulator` is declared an exclusive resource, and the 15 `ios-sim` items moved to it from `gpu`. The owner approved `gpu: 2, chrome: 1`; with `gpu` shared, two simulator checks could otherwise run on the one simulator at once.
- Assumption: when the run on main names a first failing commit and no fix has merged an hour later, the lead reverts that commit, as the approved report says.
- Assumption: a module's qualified timings are taken in its last milestone with the other builders paused, and the builder asks for that sitting by ending `BLOCKED`. No item's text changed for it: every open timing item already says that no figure decides it.
- Assumption: the guard stays its own module, so it does not sit on `APPLE-TILE`'s critical path.
- Assumption: macOS, the GPU driver, Chrome, Node and `wasm-bindgen` stay outside what any check pins, so an update that moves an image shows as main failing with no commit to blame.

## M1 A change that alters a registered exact image, on the Mac or in Chrome, is refused before it reaches main: done
[review](../../evidence/BASELINE-GUARD/M1-review.md)

## M2 A commit that changes a registered reference image or its comparison is refused at the merge unless it records the refresh

The verifier, the notice and the answer for a commit are in `scripts/exact_baselines.py`, their tests in
`scripts/test_exact_baselines.py`, and the registry in `.config/exact-baselines.json`. `./dev check` runs the
notice (`gate`) and the tests from `tools/xtask/src/main.rs`; the guard takes the notice's place there.
`verify` without `--full`, `covered_by_main` and `MainCheckTests` were built with the ruling of 15 November and
stay as they are.

The module list has two rulings that allow a refresh today, for `ATMOSPHERE` M9 and for `CULLING` M3. No
reference file has changed since 11 November, so the first pins record the references as M1 found them, and
their refresh names the section itself, `exact-baselines`.

The earlier M2 is on the branch `build/baseline-guard`. Take nothing from it: its tests assume a Cargo workspace
fixture and a key this plan does not use. The run on main uses the checkout
`.git/orchestrate/main-checks/exact-images/checkout`, which is where its scratch collects.
