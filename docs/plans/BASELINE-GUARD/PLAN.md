# BASELINE-GUARD plan

A change that alters another module's exact reference images is refused before it reaches main.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[the rules](../ARCHITECTURE.md#exact-baselines)

## Open for the owner

- Assumption: the guard is its own one-milestone module, not an item added to `APPLE-TILE` M3, so it does not sit on the critical path or wait behind that milestone's attended items. It needs nothing and can be built first.
- Assumption: every landing that changes the renderer, the game, the scripts or a baseline pays one full verifier run on its final content (two builds, a cook, the Mac matrix and four Chrome captures), and pays it again if another such change lands first. The time is recorded, not budgeted. Landings that touch only other paths pay nothing.
- Assumption: "before landing" means before a push made through the loop's `land`, which reruns `./dev check` on the rebased commit. A thread that pushes with a plain `git push` is bound only by its own `./dev check` run; no Git hook or server-side check is built.

## M1 A change that alters a registered exact image, on the Mac or in Chrome, is refused before it reaches main

Criteria: M1.1–M1.5 in the done-when file.

- [x] First, run the comparisons on untouched main: the native matrices, and the four Chrome `fixed` captures against the retained digests. If any fails there, end `BLOCKED` with the comparison.
- [x] Add the registry file and the verifier: both builds, the fresh cook, the owners' native matrices, the Chrome captures and their comparison, the record, and the proof on a full pass.
- [x] Add the reference-by-digest mode to `scripts/apple_tile_m1_compare_browser.py`, leaving its byte mode as it is.
- [x] Add the gate to `./dev check`, with a test that drives fetch, rebase, gate and push against a throwaway remote.
- [ ] Run the three mutants and the stale-proof case through the loop's `land` in a scratch clone with a local bare remote. Keep the patches, the base revision, the commands and the logs.
- [ ] Record the verifier's and the gate's run times from three runs each. Describe the gate and the verifier in CONTRIBUTING and `./dev help`.

Stop rules:
- Never refresh, regenerate or edit a file under a registered baseline. If a baseline does not match on untouched main, end `BLOCKED` with the comparison; the refresh is its owner's.
- No tolerance, nearest-frame search, retry or skip, for any reason.
- The verifier reads nothing from another task's scratch, such as `tmp/g3t-m0` or `tmp/g3t-m1`.
- Do not change `scripts/apple_tile_m0.py`, `apple_tile_m1.py` or `apple_tile_m2.py`, nor the byte mode of the Chrome comparator.
- Mutants and the `land` demonstration never push to the real remote.
- Chrome needs a browser launch outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
