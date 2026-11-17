# SIM-2D M3 target-specific rule guard

The guard runs the pinned Rust 1.99.1 Clippy on every registered rule package in
development and release profiles for native, browser Wasm and ARM64 iPhone
simulator builds. It reports the package, target, profile and forbidden operation.
This covers the review's Wasm-only raw float method and a simulator method
compiled only for release. Both fixtures exercise the registered-package check
and name the refused class. The positive fixture passes all six builds.
[Registered refusal probes](M3-target-guard/registered-refusals.json)

The proof runs used clean source commit `e2c1ac2989091a7baf3e66131befc638cd0d6901`
on the M3 Max. The target receipts include the runner input and artifact hashes,
host load and toolchain. Both retained references match the M2 pass at
`9de561807` byte for byte. The complete folds and checkpoints in all 26
target rows were also checked against those references.
[Retained comparison](M3-target-guard/retained-comparison.json)

| Item | Evidence |
| --- | --- |
| M3.1 | The 11 original refusals still show their required diagnostic. The Wasm-only fixture is refused in both Wasm profiles. The simulator-only fixture is refused in simulator release. Each fixture adds its named violation to the passing package, and the controls require every unaffected build to pass. [Control receipt](M3-target-guard/profile-controls.json) |
| M3.2 | Crowd passes all six builds. `./dev check` calls the registered-package guard and controls. The guard still reads `.config/architecture.toml`, compares the canonical Clippy configuration byte for byte and refuses suppression. [Registered package receipt](M3-target-guard/profile-check.json) |
| M3.3 | The crowd-name search in both engine crates had no matches. The adapter accepts a registered package and selects its runner, vectors, sizes, traces and witnesses. The same adapter sends the platform-sine control through Mac and Chrome, where it diverges at tick 1. [Adapter tests](M3-target-guard/adapters.log) · [Sine control](M3-target-guard/sine-control.json) |
| M3.4 | The creator guide links the portable rules guide. It documents the profile operations, refused classes, state and random inputs, query order, fixed cell size, registration, recording and target commands. The documented recording command produced a candidate with 603 data rows matching retained Crowd exactly. [Portable rules guide](../../SIM2D-RULES.md) · [Candidate trace](M3-target-guard/guide-crowd.txt) |
| M3.5 | Mac passed 10 Crowd and Mixed rows, including both 10,000-body runs for 36,000 ticks, local and wire routes, debug rows and profile vectors. [Mac receipt](M3-target-guard/mac-metal.json) |
| M3.6 | Chrome passed eight rows for the same two retained traces, including local and wire routes and profile vectors. [Chrome Wasm receipt](M3-target-guard/chrome-webgpu.json) |
| M3.7 | iPhone 18 Pro simulator passed eight rows for the same two retained traces, including local and wire routes and profile vectors. This establishes simulator function. [Simulator receipt](M3-target-guard/ios-sim.json) |
| M3.8 | W1 and P3 record implementation, correctness, visuals, device coverage and earlier costs separately. They list physical iPhone, x86, Android and browsers other than Chrome as untested. They limit the simulator claim to function. [Maturity ledger](../../design/maturity.json) |
| M3.9 | The shared main-branch check `exact-images` covers the integrated milestone. This patch changes no registered image reference. |

The earlier authority-resource refusal remains covered by the native public API
tests. [Resource repair](M3-resource-repair.md)
