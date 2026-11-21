# TEST-SUPPORT log

- 2026-11-21 Built M4: the kit's `records` module holds the renderer's, sea's, asset crate's, water's and clouds' WGSL
  records to their Rust structs in member name, offset, size and scalar kind; the renderer's walk and the sea's own
  layout type are gone. Every record passed the stricter check, so none was left on its old comparison. The
  proving ground's cooked tests build store, cook, device and host through `tests/support/cooked.rs`, adding no lock
  or shared device (9.40 to 9.65 s and 1.34 to 1.43 s at load 8 to 9). Verified on Metal: a required-GPU
  `cargo test --workspace` at `b986e6893` passes with every earlier `ok` and no skip line.
  [Evidence](../../../.orchestrate/evidence/TEST-SUPPORT/M4/README.md).

- 2026-11-21 Fixed M3's review findings: four frame waits again give each poll what is left of one deadline, the
  last local frame waits in the runtime tests and the two-cascade PPM writer use the kit, and the cook bench and water
  query wrappers are gone. Verified on Metal: a required-GPU `cargo test --workspace` at `2a83415dc` passes with no
  skip line, and each of the 12 tests out of it since `4592e88cb` was taken out by `GATE-TIME` M9 (split into named
  cases) or `DEAD-CODE` M5; each of the 21 long tests out since `0870118d6` by `GATE-TIME` M2 or `DEAD-CODE` M5.
  [Evidence](../../../.orchestrate/evidence/TEST-SUPPORT/M3/README.md).

- 2026-11-21 Built M3, the conversion, in four workers with disjoint files. `test-readback` is 0 under `crates/` and
  the kit scenes (70 sites), with one named exemption, the mirror pool's wasm timer-yield; `test-deadline` fell from
  128 to 105. 132 tests open through the one call; the gate calls left are wrappers for a generation, a fresh
  adapter, an owned instance or a capability cap. Frame waits, scratch folders, PPM writers and all six allocators
  use the kit; the kit's count gained `largest` for the transport parsers' cap. Verified: `cargo test --workspace`
  before and after (`THRUM_REQUIRE_GPU=1`) report the same 1,339 `ok` lines and no skip line.
  [Evidence](../../../.orchestrate/evidence/TEST-SUPPORT/M3/README.md). Long tests and exact images are proved on main.

- 2026-11-21 Built M6: the kit's `target` module (`target::new(&device, Depth)`, a 64-texel `Rgba8Unorm` colour
  target with an optional `Depth32Float` one, wgpu types only, `target::SIZE` for hosts) and its two tests, which draw
  a left-half quad and read colour and depth back through the kit. Verified on Metal with `THRUM_REQUIRE_GPU=1`; a
  32-texel target and a removed draw each fail both tests. The skill's "Unit tests" and "GPU tests" rows became the
  five-line block (1,695 words); patterns.md gained the five anchored entries, linked to the settings' test tiers.
  The tests sit inline in `target.rs` because `.gitignore` ignores every `target/` folder. `agent_loop check`
  passed all three items and `./dev check` on `59047a2e4`; this session could not write to the evidence folder.

- 2026-11-20 Built M5, the kit: readback of a buffer, a texture mip or layer, or a byte range, on its own
  submission or appended to the caller's encoder, with one 120 s wait; `open` (gate, then recipe) in the kit and
  in `thrum-render`'s `test_device`; `wait::until`, `Scratch`, `ppm::write` and `CountingAllocator`. Each has a
  test, all passing on Mac Metal with the GPU required. No existing test was converted. `test-readback` entered
  at 80 sites in 73 files (it also counts `map_buffer_on_submit`), `test-deadline` at 145 in 61; one more fails
  `./dev check-docs`, verified by a probe. Exact images rely on the shared main-branch check.
  [Proof](../../../.orchestrate/evidence/TEST-SUPPORT/M5/README.md).

- 2026-11-20 Fixed the second review's M2.2 findings. The parent-condition finding repeated because the
  last round fixed named sites and widened the rule only to test modules; `whole-file-gpu-gate` now follows any
  OS-conditioned module's declared subtree, with a stated `platform-only tests:` reason as the one exception.
  Verified: 16 Metal-only tests admit Metal and pass on Mac Metal with the GPU required; `thrum-app`'s file
  watcher and capture writer tests compile for iOS Simulator; the camera route test is unconditional.
  Not run: Linux or Windows. [Proof](../../../.orchestrate/evidence/TEST-SUPPORT/M2/review-fix-2.md).

- 2026-11-20 Built M2's re-planned items. `scripts/check_cross_target.sh` type-checks every workspace target for
  Linux and Windows with temporary stand-in compilers, and `ORCHESTRATE.md` declares it as the main-branch check
  `cross-target`. Fixed its two failures (the asset draw tests' `serde_json`, the water scene's tests gated one by
  one) and one CPU route test in `v1-game` hidden by a macOS condition. Verified: both targets pass, a Linux-only
  error fails it, the shape checks pass, clippy is clean on Mac. Judged by reading, with the third review's
  probes: every Metal assertion sits behind a Metal admission or a macOS condition; the probe is keyword-based.
  [Proof](../../../.orchestrate/evidence/TEST-SUPPORT/M2/cross-target.md).

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
- 2026-11-20 Planner, diagnosis of M2 after three failed reviews, at the lead's request: a re-plan. The plan was at fault. M2's bar was a Linux or Windows run that neither fails for want of an adapter nor hides a CPU test, and nothing can make that run; M2.2's check was a shape rule, and its question asked for 13 named tests and "a compile for a target that is not macOS", which a compile of two graph targets for the iOS Simulator met. So each fix took OS conditions off code nothing had compiled for Linux or Windows, and each reviewer tried a wider probe than the last and found the next break: rule spellings and missing admissions in [the first review](../../../.orchestrate/evidence/TEST-SUPPORT/M2-review-1.md), tests the first fix exposed that admit any backend and assert Metal in [the second](../../../.orchestrate/evidence/TEST-SUPPORT/M2-review-2.md), and a build that fails in two packages in [the third](../../../.orchestrate/evidence/TEST-SUPPORT/M2-review.md), whose reviewer was the first to type-check the whole workspace for both platforms.
  Changed: M2.2 is now that type-check, as a tracked command the builder runs and the script runs again, and what it cannot see became M2.6 (no OS condition hides a CPU test) and M2.7 (a GPU test asks the gate first, and for Metal where it needs Metal), each with a question that has an end. M2.8 declares the type-check as the main-branch check `cross-target`. The bar and the design's new mechanism 7 say what is not claimed: linking, a lint-clean build, doc tests and GPU tests on another backend. M2.4 keeps its claim and names the recorded comparison as its proof, because 305 commits have landed since that record's starting commit and a new whole-workspace comparison would mostly show other modules' changes. M2.1, M2.3 and M2.5 are as they were.
  Measured while planning: the third review's script, on main at `837c651e2`, took 29 and 28 seconds from cold for the two targets at a load average of 15 to 20, about 1 GB of build output each, and fails only in `thrum-asset-gpu`'s lib tests and the water scene's 11 test targets; with every target, binaries and examples included, nothing else fails.
  Decided: the type-check runs in the main tier. It is short, but a break in it stops no builder, and `GATE-TIME` M1 is rewriting what `./dev check` runs. The water scene's tests stay in M2: the check cannot pass with them broken, an exemption would make it wrong from its first run on main, the change is confined to one package's test files, and M3, the next milestone in those files, waits behind `GATE-TIME` M2, which waits for this one.
  Proposal for `GATE-TIME`, for its next re-plan and not for M1, which is building: name `cross-target` in the Main row of the settings' test tiers and in the `CONTRIBUTING.md` text M5.5 asks for; `.config/long-tests.txt` and `./dev long-tests` need no change. It may fold the check into `./dev check-all` when M7 builds that.
- 2026-11-21 Planner, for the test guard rules the owner approved ("Yes, plan it"): the design gained mechanism 8, writing a test, and the plan gained M6, built before M3. M6 adds the kit's small target (`thrum_test_support::target`), which the lead's text for the skill names and which did not exist, replaces the "Unit tests" and "GPU tests" rows of the engineering skill with one block of five lines, and writes the five pattern entries that `GATE-TIME` M12's rule messages link to. The lead's words: "the block stays about that size and AGENTS.md gets nothing", so M6.2 holds the skill to about forty words over its 1,657.
  Decided: the target and the text are one milestone here and not part of M3 or M4, which wait for other modules, because `GATE-TIME` M12 waits for the entries; the entries state no limit and link to the settings' test tiers; existing tests are not moved to the target. No item of M3 or M4 changed. The plan's notes for M2 and M5, both Done, were cut to their headings.
- 2026-11-21 Planner, diagnosis of M3 after [its first review](../../../.orchestrate/evidence/TEST-SUPPORT/M3-review.md), whose M3.6 finding only a planner could settle: a re-plan of M3.4 and M3.6. The plan was at fault. M3.6 held M3 to every test `long-tests` ran at `0870118d6`, and `GATE-TIME` M2 (`dbe1cac8d`, which M3 waits for) and `DEAD-CODE` M5 (`ac62118e4`) then deleted 20 of them and moved one, as their own approved items asked, so no build could pass it without undoing them.
  Decided: M3 is held to its own commits. A test missing from the later run counts against M3 unless a commit of another module took it out; the record names that commit for each, and the reviewer confirms it from the diff and does not judge the removal again. A test missing with no such commit is still M3's loss, so the check catches what M3 drops. The design's Proof section states the rule.
  Changed: M3.6 and M3.4 are written again from that claim, with the earlier runs pinned to `0870118d6` and `4592e88cb`. M3.4 passed its review but had the same fault waiting: its next after-run lacks the four projection tests `DEAD-CODE` M5 removed. The plan's note for M3 says where the saved runs are. No check command changed and nothing was added to a merge's check.
  Not changed: M3.1, M3.2, M3.3 and M3.5. The review's four findings on M3.2 and M3.3 are within those items as written and are the builder's to fix.
