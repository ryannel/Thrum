# BASELINE-GUARD plan

A change that alters another module's exact reference images is refused before it reaches main.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[the rules](../ARCHITECTURE.md#exact-baselines)

## Open for the human

- Assumption: the guard is its own module, not an item added to `APPLE-TILE` M3, so it does not sit on the critical path or wait behind that milestone's attended items.
- Assumption: every landing that changes a file a verified build or comparison reads pays one full verifier run on its final content (two builds, a cook, the Mac matrix and four Chrome captures, about 17 minutes), and pays it again if another such change lands first. The time is recorded, not budgeted. Landings that touch only other files pay nothing.
- Assumption: "before landing" means before a push made through the loop's `land`, which reruns `./dev check` on the rebased commit. A thread that pushes with a plain `git push` is bound only by its own `./dev check` run; no Git hook or server-side check is built.
- Assumption: "exactly the inputs ... and nothing else" (15 November) is met file by file only for Rust sources, where the compiler's own record says what a build read. Every other file in a linked package stays covered, and so does the whole of a package that has a build script, because nothing records what the cooker, the page build and the scripts read. On the night of 14 to 15 November this key changes at 10 of main's commits, where the listed one changed at 33.
- Assumption: the workspace `Cargo.toml`, `Cargo.lock` and every workspace package's `Cargo.toml` stay covered as whole files, so a landing that adds a workspace member or changes any package's dependencies lapses every open proof. Cargo reads them all to load the workspace. A key over the resolved graph alone would have spared one of those 33 commits, and would trust a second derivation.
- Assumption: "the scripts and tools it runs" means files in this repository: the cooker, the scripts reached from the verifier by an import or a quoted file path, and any path the registry lists. The verifier works in a scratch tree that holds only covered files, so a script that reads anything else through a computed path fails and gets no proof, until the path is listed. macOS, the GPU driver, Chrome, Node and `wasm-bindgen` stay outside the key as before, so an update that moves an image shows as untouched main failing the verifier.
- Assumption: one read stays unguarded. A Cargo build script could read, undeclared, a test source of another linked package, and a change to that file would keep a proof. One linked package has a build script and it reads one C file of its own. Closing it means building each such package from nothing on every verifier run.
- Assumption: every verifier run builds in that scratch tree, which adds a second page build and whatever Cargo rebuilds because the sources moved. M2 records the time and disk beside M1's 17 minutes; none is budgeted.
- Assumption: `SIM-2D`'s cost records keep their own source key, which is drawn wider than its inputs (the survey is in `docs/evidence/BASELINE-GUARD/M2-plan/README.md`). Narrowing it is a small change to `SIM-2D`'s script and plan, and is not planned here.

## M1 A change that alters a registered exact image, on the Mac or in Chrome, is refused before it reaches main: done
[review](../../evidence/BASELINE-GUARD/M1-review.md)

## M2 A proof of the exact images survives a rebase that brings in only code the verified game is not built from

The gate, the verifier and the key are in `scripts/exact_baselines.py`, their tests in
`scripts/test_exact_baselines.py`, and the registry in `.config/exact-baselines.json`. `./dev check` runs the gate
and the tests from `tools/xtask/src/main.rs`. [The planner's measurements](../../evidence/BASELINE-GUARD/M2-plan/README.md)
give the linked packages, what one verifier build's dependency records list and where Cargo wrote them, and the
scripts reached today; check your derivation against them.

The item checks name test classes: `CoveredPathTests`, `UnreadSourceTests`, `FailClosedTests`,
`GuardInterfaceTests` and `VerificationRootTests` are new, and `GateTests` is M1's. M1's tests build a Git
repository with no Cargo workspace in it, so their fixture has to become one; keep what each asserts.

The verifier runs in the worktree today. [The planner's trial](../../evidence/BASELINE-GUARD/M2-plan/README.md#a-root-that-holds-only-the-covered-paths)
of the verification root says what Cargo and Git needed there and what was not tried: no matrix or capture has
drawn from a root yet, so make one full verifier pass in it early. The owners' scripts ask Git for the files it
lists, tracked and untracked, for the revision and for a diff, and `capture_v1_browser.mjs` asks with no path
filter. They belong to `APPLE-TILE` and `T2-CAPTURE` and do not change.

M2.5 is made before the landing, on the branch, with the proof from the verifier pass that the landing needs
anyway. The scratch main it rebases onto is a local bare repository.

Other modules' scripts call the guard; [the design lists what they rely on](DESIGN.md#what-other-modules-use).
`GAME-SPLIT` changes the verifier's environment and the game's package name later, in its own milestones: prepare
nothing for it. This landing changes the verifier, which is a covered path, so every open proof lapses once.
