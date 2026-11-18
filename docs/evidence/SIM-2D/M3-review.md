# SIM-2D M3 review

All nine M3 items pass. One interface promised to game authors is missing at module close, so M3 returns to `in_progress`. I reviewed `7a024294bbb7`, with builder work ending at `146ecee5361a`.

## Must Fix · verified · outside the items

**`profile-missing-64-bit-random-draw`.** Mechanism 3 promises a keyed 64-bit draw and a second function that converts its top 24 bits into a profile number. SWARM and SESSION are named consumers of that mechanism. The only public draw returns `Number`. It runs ten Philox rounds, discards the second 32-bit word and the low eight bits of the first, and returns the fraction. A rule author needing the integer draw cannot recover the omitted 40 bits. The known-answer test also discards those bits before comparing. [Design](../../plans/SIM-2D/DESIGN.md#3-random-numbers-keyed-by-seed-tick-stream-and-entity) · [Public API](../../../crates/thrum-core/src/profile.rs) · [Known-answer test](../../../crates/thrum-core/src/profile/tests.rs)

I built a native consumer of this checkout with Rust 1.99.1. Calling the public function for a fraction succeeded. Assigning its result to a `u64` failed with E0308. I checked the public exports for a companion function. The missing interface is shared by all three targets. I ran this compile probe only on the Mac. [Consumer and compiler output](M3-review/random-api.txt)

A proposed done-when item is that a rule package can request the full 64-bit draw, match both words of a published Philox vector, and reproduce the existing fraction from the draw's top 24 bits. That would cover the missing API while protecting the retained traces.

## Item verdicts

| Item | Verdict | Judgment |
| --- | --- | --- |
| M3.1 | Pass | Each fixture changes one operation, field type, dependency, lint attribute or configuration file relative to the passing package. The two invalid field types fail the same field-wise encoding as the positive control. The harness checks exit status and diagnostic text, rejects an unexpected success, and requires the target-specific cases to pass wherever their violation is compiled out. |
| M3.2 | Pass | The xtask behind `./dev check` runs both profile commands. The registry marks `crowd-2d` as a rule package. The guard discovers it there, compares its Clippy configuration byte for byte with the canonical file, scans for suppressions and invokes Clippy with all three profile lints forbidden. Crowd passed all six target/profile combinations. |
| M3.3 | Pass | Neither engine crate names a crowd type, rule or constant. The command takes `--package` and reads its runner, sizes, variants, witnesses, vectors and retained paths from the registry. The native executable and Wasm module both follow the selected runner. The sine control uses the same adapter and diverged at tick 1 on Mac versus Chrome. The alternate-package test uses mocks. |
| M3.4 | Pass | The creator guide links the rules guide. It covers each operation in mechanism 1, each refused class and reason, the implemented random function, overlap and nearest order, and cell size as a rule-set constant. The commands match the CLI, the documented lock script exists on this host, and the recorded candidate matches Crowd. The missing integer API is the separate interface finding above. |
| M3.5 | Pass | Both references are byte-identical to the M2 pass at `9de561807`. I compared every Mac row in the script's clean run with its retained tick count, fold and full checkpoint sequence. All ten matched, covering both variants at all three sizes and the additional wire and debug cases. Profile vectors passed. |
| M3.6 | Pass | All eight Chrome rows matched the same retained values and profile vectors passed. The browser adapter loads the compiled Wasm in Chrome and calls its exports. Both variants include the 10,000-body, 36,000-tick run and the smallest wire route. |
| M3.7 | Pass | All eight simulator rows matched the same retained values and profile vectors passed. The adapter builds the ARM64 iOS simulator executable and runs it through simctl. This establishes simulator function. |
| M3.8 | Pass | W1 and P3 each distinguish implementation, correctness, visuals, device coverage and performance. Both explicitly list physical iPhone, x86, Android and other browsers as untested and limit the simulator result to function. |
| M3.9 | Pass | The shared main-branch exact-images check passed as reported by the script. M3 changed no retained image. |

## Previous findings

**Resolved · `rule-guard-skips-target-specific-code`.** Both the registered-package guard and the controls now build native, Wasm and iOS simulator code under dev and release. The Wasm-only sine call fails in both Wasm builds. The simulator release-only call fails in that build. The positive fixture passes all six. [Registered refusals](M3-target-guard/registered-refusals.json) · [Control results](M3-target-guard/profile-controls.json)

**Resolved · `profile-resource-nonfinite-commit`.** The runtime checks staged authority resources after merges and before commit. The crowd exposes the aggregate its apply callback commits. I reran the two native public-API resource tests from the review checkout. Atomic refusal, finite recovery and the non-profile control passed. [Resource tests](../../../crates/thrum-core/tests/profile_resources.rs)

## Suggestion · verified

The random order-independence test compares two sequences that each contain one repeated value. Its inputs set entity equal to tick, so those inputs cancel in the counter's XOR. The public-API probe returned the first value for all 256 draws. Add a varying sequence to the test so that accidental constant output on that sequence would fail it. I did not assess the statistical quality of the reference crowd. [Test](../../../crates/thrum-core/src/profile/tests.rs) · [Probe output](M3-review/random-api.txt)

## Evidence scope

I judged the items before reading the earlier reviews and the builder's narrative. I used the script's accepted check results on `eaf682210eab` and independently compared all 26 saved target rows. The three long trace runs and shared image check were not rerun. M3 has no reviewer-inspection or human-feedback items. The rules run headlessly, so weather, camera motion, display refresh, rotation and app lifecycle have no bearing on the proofs.

FAIL M3: 0 fail, 0 could_not_judge, 1 outside the items
