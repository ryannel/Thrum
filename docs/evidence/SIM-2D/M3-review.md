# SIM-2D M3 review

M3 returns to `in_progress`. All nine written items pass. One verified defect outside the items breaks the milestone quality bar on a supported target. The script checked builder tip `b8c8e71bd6b3`. I reviewed checkout `5ca20ca8ee3a`.

## Must Fix · rule-guard-skips-target-specific-code · verified

A registered rule can use a prohibited raw float method on Chrome and pass the profile guard. The guard invokes host Clippy with `--all-targets` in [sim2d_profile.py](../../../scripts/sim2d_profile.py#L52). That includes Cargo targets such as libraries and tests, but Rust still compiles them for the host architecture. The Wasm Clippy commands in [xtask](../../../tools/xtask/src/main.rs#L52) select specific application packages. They do not apply the profile lints to each registered rule package for Wasm.

I used the passing control's manifest and canonical configuration in a temporary package and supplied this rule. The input is an ordinary raw axis of the kind the guide instructs authors to convert.

```rust
use thrum_core::profile::Number;
pub fn movement(axis: f32) -> Number {
    #[cfg(target_arch = "wasm32")]
    let axis = axis.sin();
    Number::from_axis(axis)
}
```

The production configuration check returned no errors. Its Clippy invocation exited **0**. Running the same three forbidden lints with Rust 1.99.1 and `--target wasm32-unknown-unknown` exited **101** at `axis.sin()` with “use of a disallowed method `f32::sin`”. The Mac converts the original axis while Chrome would convert its sine. A rule with that branch should fail the project's guard even when the current trace never calls it. This was a compiler probe on the Mac. I did not launch that fixture in Chrome or repeat it for iOS.

Proposed item: “The project guard rejects a prohibited operation compiled only for a supported rule target and names the class.” This covers a gap between the host refusal fixtures in M3.1 and the passing reference package in M3.2. Neither item currently exercises a violation present only in a target build.

## Item verdicts

| Item | Verdict | Basis |
| --- | --- | --- |
| M3.1 | Pass | Compared the eleven fixture cases to the passing package. Each adds its named violation or substitutes one field type. The dependency and missing-config cases keep the passing source. The two invalid field types lose the required encode method. The controls command requires both a nonzero exit and the expected diagnostic for each refusal, and requires the baseline to succeed. The script's controls passed. |
| M3.2 | Pass | The architecture registry marks crowd-2d as a rule package. The guard discovers it there, compares its Clippy file byte for byte to the canonical copy, checks dependencies and suppressions, and forbids all three profile lints. The reference package passed. The check entry in xtask invokes both the guard and its controls. |
| M3.3 | Pass | Neither engine crate names the crowd. Package selection supplies the registered runner, trace sizes, vector and retained paths, and required witnesses. Native and Wasm dispatch use that runner. The platform-sine control uses the same adapter with its registered counts and diverged at tick 1 in the saved run. The alternate-package test covers dispatch with mocks. |
| M3.4 | Pass | The creator guide links the portable rules guide. It enumerates mechanism 1's arithmetic, rounding, comparisons and conversions, explains each refused class, specifies the random inputs, gives overlap and nearest visit order, and identifies cell size as a rule-set constant. The registration, CLI and Wasm interfaces match the runner. The documented loop path exists on this host. The guide's candidate-record exercise matched all 603 retained data rows, and the target commands have passing receipts. |
| M3.5 | Pass | Checked the script's ten Mac rows against the retained folds and checkpoint sequences. Crowd and Mixed each ran the three registered sizes, including 10,000 bodies for 36,000 ticks, plus the smallest wire and debug cases. Profile vectors passed. Both retained files are unchanged from the M2 pass at `9de561807`. |
| M3.6 | Pass | Checked all eight Chrome rows against the same references. The browser adapter loads the built Wasm module in Chrome. Both traces cover three sizes and the smallest wire route, and the profile vectors passed. |
| M3.7 | Pass | Checked all eight simulator rows against the same references. The runner builds for the iOS simulator and invokes the executable through simctl. Both traces and the vectors passed. This establishes simulator function. |
| M3.8 | Pass | W1 and P3 separate implementation, correctness, visuals, device coverage and historical costs. Both name physical iPhone, x86, Android and other browsers as untested, and explicitly limit the simulator claim to function. |
| M3.9 | Pass | Main-branch check `exact-images` passed on `b8c8e71bd6b3` itself. The script's verifier receipt confirms coverage. M3 changed no retained image. |

## Previous finding and review scope

**Resolved — `profile-resource-nonfinite-commit`.** Crowd exposes the same staged resource aggregate that its apply callback commits. After merges and component validation, the runtime checks the aggregate inside the tentative-write rollback boundary. I ran the two public-API resource tests in this checkout. NaN and both infinities were refused across queued and direct writes with serial and forced-parallel execution. The tests check the previous digest, resources, endpoint, publication and delta availability, then require a finite retry to commit. The non-profile control also passed. [Resource tests](../../../crates/thrum-core/tests/profile_resources.rs)

I assessed the items before consulting the prior review and builder's narrative. M3 has no reviewer-inspection or human-feedback items. Its CPU traces exercise slot reuse, corrected replay and wire conversion. Display, weather, camera and simulator UI lifecycle edges do not apply to these headless commands. I used the script's successful closing runs for the three targets and the shared image verifier.

FAIL M3: 0 fail, 0 could_not_judge, 1 outside the items
