# TEST-SUPPORT log

- 2026-11-20 Fixed M2's review findings. Diagnostic device adjustments respect the limits and features
  selected by the recipe and registry. CPU tests stay in their harnesses, Metal tests report unavailable
  prerequisites through the GPU gate, and the sprite fixtures use declared modules. Verified the repaired
  cases on Mac Metal, the three shape checks, and graph compilation for iOS Simulator and Wasm.
  [Proof](../../../.orchestrate/evidence/TEST-SUPPORT/M2/review-fix.md).

- 2026-11-20 Routed workspace GPU tests and benches through the gate and HOST's device recipe.
  The three shape rules reached zero in their milestone scopes, and the 13 CPU graph cases compiled
  for iOS Simulator and Wasm. The Mac comparison matched every baseline pass with the GPU required.
  Five duplicate executions map to the same unchanged occlusion-oracle tests. The candidate recorded
  1,286 passes, no failures and no GPU skips. The shared main-branch check covers exact images.
  [Proof](../../../.orchestrate/evidence/TEST-SUPPORT/M2/README.md).

- 2026-11-16 Built M1's wgpu-only GPU gate, restricted test devices and test-crate-edges rule.
  Migrated sea and asset tests and the cooker's viewer-backed test and bench; the viewer accepts an
  injected device behind its test-support feature. Preserved assertions, limits and bench timing scope.
  Verified all 72 prior passing outcomes again with a required Metal GPU, with no GPU skip line;
  rule and child-process gate checks pass. [Proof](../../../.orchestrate/evidence/TEST-SUPPORT/M1/README.md).
  Exact images use the shared main-branch check; no retained image changed. HOST replaces the stand-in in M2.

- 2026-11-14 Planned from the owner's request to plan and drive the code review's fixes; theme T6. Decided: the
  gate deals in wgpu types only, because a crate that depends on `thrum-render` cannot hand its unit tests a value
  typed by it; the device comes from `HOST`'s recipe. Test files are not all moved to one layout.
- 2026-11-15 Revised after [the first design review](../../../.orchestrate/evidence/TEST-SUPPORT/design-review.md) (four Must
  Fix). A skip is written to the standard error handle, which the harness does not capture
  ([probe](../../../.orchestrate/evidence/TEST-SUPPORT/planning/skip-line-probe.md)), and adoption is proved by run results under
  `THRUM_REQUIRE_GPU=1`, not by test lists. A test's device is the recipe's by profile and restriction, settled
  with `HOST`, so the sea's reduced-limit tests keep their inputs. Fixtures move to a second package,
  `thrum-fixtures`, which the proving ground may take as a normal dependency; the rule is `test-crate-edges`,
  settled with `GAME-SPLIT`. M2 now waits for `GAME-SPLIT:M2`.
- 2026-11-20 Planner, from the owner's request "We want to do a code cleanup and consolidation for Thrum" and section 5 of the cleanup review: the design gained mechanism 6, the kit, and the plan gained M3 (readback, the prologue, waits and scratch through the kit) and M4 (one record reflection check, one cooked-game fixture). M3 waits for `GATE-TIME` M2 and `DEAD-CODE` M2, which delete tests and stubs in the same files, and M4 for `REVIEW-RENDER` M2. No item of M1 or M2 changed. The design is owed a review because its text changed.
- 2026-11-20 Planner, revision after the kit's design review and the hardening pass (`HARDENING.md` in the cleanup review's folder), in one pass with `GATE-TIME`, `DEAD-TOOLS` and `DEAD-CODE`. Must Fix 1: `thrum-render`'s unit tests, 160 of the 286 prologues, get the one call from the crate's own `test_device`, and M3.2 says so. Must Fix 2: the design has a files table for M5, M3 and M4, with what each can run beside.
  M5 is new and built before M3: the kit's functions, and `test-readback` and `test-deadline` frozen at the count of the commit they land on, because readbacks and deadline asserts in tests tripled and doubled in a week while the rule waited for the conversion (proposal 6). M3 keeps the conversion, waits for `GATE-TIME` M6 as well, and carries M3.6, the long tests' main-branch item (A2, A4).
  Suggestions taken: a whole mip of a 3D or array texture is a readback form (1); the allocator counts allocations and bytes (2); the wait serves deadline loops for new tests, and M3 sweeps frame waits only (3); M5 adds the rules (4); a record that fails the stricter check is reported, not fixed (6); the cooked fixture adds no waiting (7); the renderer's own record test calls the shared check (8). Not taken: a counted search in M4.1 (5), because nothing names a bound-struct walk before the kit exists; the ask counts them.
