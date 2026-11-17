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
