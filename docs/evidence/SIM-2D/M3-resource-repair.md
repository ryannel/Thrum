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

The [first simulator attempt](M3-resource/simulator-first-attempt.stderr) was
stopped at the 30-minute resource hold limit on the 10,000-body Crowd run.
It is not a passing simulator proof. An unchanged retry with idle system sleep
prevented is running; the repository gate is also still running. See the
[handoff](M3-handoff.md) for job identities and remaining work.
