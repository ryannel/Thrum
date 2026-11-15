# BASELINE-GUARD plan

A change that alters another module's exact reference images is refused before it reaches main.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[the rules](../ARCHITECTURE.md#exact-baselines)

## Open for the human

- Assumption: the guard is its own module, not an item added to `APPLE-TILE` M3, so it does not sit on the critical path or wait behind that milestone's attended items.
- Assumption: every landing that changes a covered path pays one full verifier run on its final content (two builds, a cook, the Mac matrix and four Chrome captures, about 17 minutes), and pays it again if another covered change lands first. The time is recorded, not budgeted.
- Assumption: "before landing" means before a push made through the loop's `land`, which reruns `./dev check` on the rebased commit. A thread that pushes with a plain `git push` is bound only by its own `./dev check` run; no Git hook or server-side check is built.
- Assumption: M2 is worth building for what it saves. Replayed over main since 11 November, M1's key changed at 204 commits and the smaller key changes at 186: it spares 18, all changes to `thrum-sea`, `thrum-cloud-model`, `thrum-water-model` or `thrum-sea-model` alone. On the night of 14 to 15 November it spares 5 of 65. Of the 186, 46 touched nothing covered but `scripts/`. Re-proving is likely to stay a bottleneck, and the fuller key is recorded as later work, not planned.
- Assumption: "fails closed" includes what the compilers read. After its builds the verifier checks Cargo's own records and writes no proof if a build read a repository file the key does not hold, such as a shader a linked crate includes from an unlinked crate's folder. The task did not ask for this check; without it that file could change an image and keep a proof.
- Assumption: one read stays unguarded, as the task's subtraction implies. A script, the cooker or the game at run time that reads a file in an unlinked crate's folder as data takes input the key does not hold, unless the registry lists the file. Nothing the verifier runs does so today, beyond hashing those files into records nothing compares. A Cargo build script that reads such a file undeclared is the same case.
- Assumption: an unlinked crate's `Cargo.toml` stays in the key, since Cargo reads every member's manifest to load the workspace. No commit since 11 November changed one and no other covered path.
- Assumption: "workspace crates and examples" removes no example today, because M1's key never held an example other than `v1-game`. A crate or example the game comes to link joins the key wherever it lies.
- Assumption: macOS, the GPU driver, Chrome, Node and `wasm-bindgen` stay outside the key as before, so an update that moves an image shows as untouched main failing the verifier.
- Assumption: `SIM-2D`'s cost records keep their own source key, which is drawn wider than its inputs (the survey is in `docs/evidence/BASELINE-GUARD/M2-plan/README.md`). Narrowing it is a small change to `SIM-2D`'s script and plan, and is not planned here.

## M1 A change that alters a registered exact image, on the Mac or in Chrome, is refused before it reaches main: done
[review](../../evidence/BASELINE-GUARD/M1-review.md)

## M2 A proof of the exact images survives a rebase that brings in only changes to crates the verified game does not link

The gate, the verifier and the key are in `scripts/exact_baselines.py`, their tests in
`scripts/test_exact_baselines.py`, and the registry in `.config/exact-baselines.json`, whose `covered_paths` are
the listed paths. `./dev check` runs the gate and the tests from `tools/xtask/src/main.rs`.

The item checks name test classes: `CoveredPathTests`, `ClosureTests`, `FailClosedTests`, `BuildReadTests` and
`UnlinkedRebaseTests` are new, and `GateTests` is M1's. M1's tests build a Git repository with no Cargo
workspace in it, so their fixture has to become one; keep what each asserts.

[The planner's trial](../../evidence/BASELINE-GUARD/M2-plan/README.md#the-smaller-step) holds what Cargo gave
on a workspace made for it and on this one. `cargo metadata --no-deps` reads no lockfile and no registry cache
and gives each member's declared dependencies; the resolved graph with every feature on gives the same nine
packages today. The verifier's builds leave their records at `target/release/v1-game.d`,
`target/release/thrum-cook.d` and `target/wasm32-unknown-unknown/release/v1_game.d`. A record names an included
file by the path the compiler formed, with `..` in it.

M2.6 is made before the landing, on the branch, with the proof from the verifier pass that the landing needs
anyway. The scratch main it rebases onto is a local bare repository.

Other modules' scripts call the guard; [the design lists what they rely on](DESIGN.md#what-other-modules-use).
`GAME-SPLIT` changes the verifier's environment and the game's package name later, in its own milestones: prepare
nothing for it. This landing changes the verifier, which is a covered path, so every open proof lapses once.
