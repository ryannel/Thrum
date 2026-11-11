# BASELINE-GUARD log

- 2026-11-11 Created from the owner's task, quoted in the design: "Say in ARCHITECTURE.md or the affected designs
  how a module's change that alters another module's exact baselines gets caught before landing." The rules are in
  the module list. This module exists because `land` runs only `./dev check` before it pushes, and the exact
  comparison was not in it.
- 2026-11-11 Planner's decision: a separate module with one milestone and no `needs`. Adding the guard to
  `APPLE-TILE` M3 would have put it on the critical path and behind attended items, and changed definitions a
  builder is about to start on. `T2-CAPTURE`'s design gives recipe comparison to `APPLE-TILE`, so it was not put
  there.
- 2026-11-11 The planner did not run the guard's checks: they need a release build and the GPU. Each check was
  parsed as shell and the done-when file validated with the loop's own validator.
