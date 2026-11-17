# SIM-2D M3 resource refusal repair

The review's `profile-resource-nonfinite-commit` finding exposed missing resource
preflight. `TickGame::profile_resources` now exposes the complete staged authority
resource aggregate through `StateEncode`. The runtime checks it after all callbacks,
parallel merges and component validation, inside the same abort boundary used for
tentative component writes. Failure returns `RegisteredWorldError::NonFiniteResource`
before edits, game apply, tick/revision advancement, history or publication commit.
The default opts out for existing non-profile games. Profile authors must expose every
authority resource and encode every field, just as they must for profile components.
The portable rules guide documents this obligation. Crowd returns its staged `Resources`.

`crates/thrum-core/tests/profile_resources.rs` uses the public API and a nested resource
encoding with the default recursive finite check. It covers NaN, positive infinity and
negative infinity produced by ordinary profile arithmetic, serial and forced Rayon
callbacks, validation after two private output merges, queued edits and tentative direct
writes. It asserts unchanged endpoint, complete digest, resources, game apply count,
publication identity/content and continued availability of the prior delta, then commits
a finite retry. Invalid derived scratch is deliberately excluded. An opt-out control
commits an infinite resource, preserving the behavior of non-profile games.

The existing component refusal test remains unchanged. Native focused tests passed during
development. Final source identity, check receipts and retained-target results follow below.
No performance, visual or simulator-speed claim is made by these tests.

Clean source checkpoint: `f64eade89b3547715f4bb5d7176c89c709f27d06`.
The [Mac receipt](M3-resource/mac-metal.json) passes all 10 rows and profile vectors;
the [Chrome receipt](M3-resource/chrome-webgpu.json) passes all eight rows and vectors.
Both have source SHA-256 `25cac590226ebf80706e46fc21adc3483e17d06a57284affbbc2ed3a8a38e62e`.
[Profile controls](M3-resource/profile-controls.stdout) and the
[registered package check](M3-resource/profile-check.stdout) pass.
The [platform-sine control](M3-resource/control.json) diverges at tick 1.
The [guide candidate](M3-resource/guide-crowd.txt) matches all 603 non-comment
rows in retained Crowd exactly. Both retained files remain unchanged.

| Item | Current builder proof |
| --- | --- |
| M3.1 | Read all eleven fixtures against the common baseline: each changes only its named source operation, dependency or configuration. The control command requires refusal and the named diagnostic, and also requires its positive control to pass. The retained control output passes. |
| M3.2 | The retained registered-package check passes without suppressions. The guard discovers `rule_package` entries, compares configuration bytes against the canonical file and checks all dependency kinds. Both profile commands are invoked by `tools/xtask/src/main.rs` in `check`. |
| M3.3 | A fresh case-insensitive crowd-name search in both engine crates has no matches. `--package` selects the registry's runner, sizes, vectors and references; the real sine control uses the same adapter and diverges in Chrome. Fifteen adapter tests passed. |
| M3.4 | The creator guide links the portable rules guide; operations, refused classes/reasons, random, query ordering/domain and fixed rule-set cell size are present. The record exercise passes. Current simulator command completion is still pending. |
| M3.5 | Mac: ten rows, both complete retained traces at all three sizes, local/wire routes, debug rows and profile vectors pass at the source checkpoint. |
| M3.6 | Chrome Wasm: eight rows, both complete retained traces, local/wire routes and profile vectors pass with the same source hash as Mac. |
| M3.7 | Open pending a complete current simulator receipt; the prior failed attempt is not counted. |
| M3.8 | Read W1/P3 `sim2d` ledger entries: implementation, correctness, visuals, devices and historical costs are separated. Physical iPhone, x86, Android and other browsers are explicitly not run; simulator speed is explicitly not claimed. |
| M3.9 | Uses `main-branch check exact-images`, as required by ORCHESTRATE.md; no retained image/reference changed. |

The [first simulator attempt](M3-resource/simulator-first-attempt.stderr) was
stopped at the 30-minute resource hold limit on the 10,000-body Crowd run.
It is not a passing simulator proof. An unchanged retry with idle system sleep
prevented is running; the repository gate is also still running. See the
[handoff](M3-handoff.md) for job identities and remaining work.
