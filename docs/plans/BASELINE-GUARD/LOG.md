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
- 2026-11-11 [Design review](../../evidence/BASELINE-GUARD/design-review.md): changes requested, two Must Fix.
  MF1: `land` rebases, checks and pushes without handing control back, so the full matrix could only run after a
  failing push. MF2: the Chrome comparator needs 481 reference frames that exist only in another builder's scratch.
- 2026-11-11 Design revised for both. MF1: the verifier writes a proof keyed by a digest of the covered paths, and
  `./dev check` refuses a covered-path change without a proof for exactly that content, so `land` cannot push it.
  M1.3 and M1.4 are the acceptance items the review asked for. MF2: the retained Chrome manifests already hold a
  SHA-256 for every frame, so four sets (day and night, SDR and scene-linear) are compared by digest from a fresh
  Wasm build, with no bulk reference and no regeneration; ruled in the module list with `APPLE-TILE`.
- 2026-11-11 S1 not applied, because its subject is gone: the small image test inside `./dev check` is no longer
  built, so there is no debug-build reference set to approve or maintain. S2 applied in M1.4: patches, base
  revision, commands and logs are kept, and a failure tells a changed image from an unreadable reference.
- 2026-11-11 The planner did not run the checks, which need the GPU, Chrome and work that does not exist yet. Each
  was parsed as shell and the done-when file validated with the loop's own validator.

- 2026-11-11 M1 builder: untouched main `1d35b9a6` passed both unchanged native matrices (38 + 4
  exact comparisons) and all four Chrome sets (481 records each). Fresh release/Wasm builds and task-owned
  cook only; [preflight records](../../evidence/BASELINE-GUARD/M1/preflight/provenance.json) retain hashes.
  Implemented registry, clean-tree verifier, digest mode and content-key gate; proof and integration experiments
  are in progress. Coverage also includes `.cargo/` (design review S3), `tools/` and the registry itself; the
  architecture table records the same wider inputs. No retained reference or owner matrix was changed.

- 2026-11-11 M1.1–M1.3 pass with [target evidence](../../evidence/BASELINE-GUARD/M1.md). The final verifier
  passes 42 native comparisons and 1,924 Chrome records at covered key `f6b5c8fa`; all seven focused tests pass.
  Successful owner-published captures are archived incrementally after disk space dropped during shared work.
  Final-version mutant refusals, the third timing and the milestone check remain in progress.

- 2026-11-11 M1.4 passes on the final verifier with incremental capture archiving. The actual loop refuses all
  four scratch cases, with a fixed local remote after each refusal. Native difference metrics, failed manifests,
  patches, base, commands and logs are [retained](../../evidence/BASELINE-GUARD/M1/refusals-final/report.json).
  M1.5 and the complete milestone check remain.
