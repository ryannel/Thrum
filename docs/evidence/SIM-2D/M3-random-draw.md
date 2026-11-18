# SIM-2D M3 portable random draws

Mechanism 3 is available through `thrum_core::profile`. `random_u64` returns the
full Philox2x32-10 result with its first output word in the high 32 bits.
`random_fraction` maps the high 24 bits to an exact `Number` in `[0, 1)`.
The existing `random` composes the two functions and keeps its existing signature.
[Portable rules guide](../../SIM2D-RULES.md)

On the Mac, the public API doctest passed and the unit tests matched both words
of all three published known answers. The tests also checked the fraction
boundaries, the existing fraction bits and reverse order on another thread with
a varying sequence. Both earlier resource refusal regression tests passed.
[Native test log](M3-random-draw/profile-tests.log) ·
[Random123 known answers](https://raw.githubusercontent.com/DEShawResearch/random123/main/tests/kat_vectors)

The proofs used clean source commit `45406db70b05e8274c70f885c13842c5bc6f2af5`
on the M3 Max. The target receipts record the simulation source identity,
artifact hashes, toolchain and host load. A separate byte comparison confirmed
that the two retained trace files and the retained profile vectors still match
M2's `9de561807`. The field-by-field comparison checked all 26 rows, including
complete checkpoint sequences and replay and wire witnesses.
[Retained comparison](M3-random-draw/retained-comparison.json)

| Item | Builder evidence |
| --- | --- |
| M3.1 | Each fixture changes one operation, field type, dependency, lint attribute or configuration from the passing package. The harness requires an unsuccessful exit and the named diagnostic for each refusal, plus acceptance when the target-specific violations are compiled out. [Mac profile controls](M3-random-draw/profile-controls.json) |
| M3.2 | The registered Crowd rule package passes three build targets in dev and release. The guard reads the package registry, checks the canonical Clippy configuration byte for byte and rejects suppressions. The repository check invokes both profile commands. [Mac registered-package check](M3-random-draw/profile-check.json) |
| M3.3 | Searched both engine crates for Crowd names and found none. The conformance command selects the runner, sizes, retained paths and witnesses from the registered package. Fifteen adapter tests pass, including a mocked alternate package. The platform-sine control ran through the same adapter and diverged on Mac versus Chrome at tick 1. [Adapter tests](M3-random-draw/adapters.log) · [Sine control](M3-random-draw/sine-control.json) |
| M3.4 | The linked guide covers the profile operations, refusals and reasons, random draws, query order, fixed cell size and recording. The documented registration checks and target commands ran. Recording a fresh Crowd candidate reproduced all 603 retained data rows. [Guide](../../SIM2D-RULES.md) · [Mac recording](M3-random-draw/guide-crowd.txt) |
| M3.5 | Ten Mac rows match both retained traces, including local and wire input, debug rows and retained profile vectors. [Mac receipt](M3-random-draw/mac-metal.json) |
| M3.6 | Eight Chrome Wasm rows match both retained traces, including local and wire input and retained profile vectors. [Chrome receipt](M3-random-draw/chrome-webgpu.json) |
| M3.7 | Eight iPhone 18 Pro simulator rows match both retained traces, including local and wire input and retained profile vectors. This proves simulator function. [Simulator receipt](M3-random-draw/ios-sim.json) |
| M3.8 | W1 and P3 distinguish implementation, correctness, visuals, devices and performance. They list physical iPhone, x86, Android and non-Chrome browsers as untested and limit the simulator result to function. [Maturity ledger](../../design/maturity.json) |
| M3.9 | The patch leaves the retained images untouched. ORCHESTRATE.md assigns verification to the shared main-branch check `exact-images`, which covers the integrated commit after landing. |

Full Philox words were compared against the known answers on the Mac. Chrome
and the simulator ran the existing profile vectors and retained timelines through
the composed fraction API.
