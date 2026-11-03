# Admitted rule source versus linked game code

2 November 2026. **Preliminary F4 code-source match, not executable attestation.**

`verify_linked_rule_source` receives an accepted `PinnedSource`, a portable code
path and fixed-size length/SHA-256 build metadata. It compares these with the
authenticated manifest, then streams the stored blob through
the production source verifier to ensure the actual admitted bytes are intact.
Mismatch refuses before the SQLite read. The returned path/digest is a typed
source match only; it cannot certify which binary or toolchain produced the game.

The F4 test admits rule bytes A, changes the checkout, proves a historical A pin
still matches the linked bytes, admits same-length changed rule bytes B and observes a
`Mismatch`. The `i1-mobile-game` test embeds its actual `game.rs` with
`include_bytes!` to derive test metadata, admits that exact file alongside a scene, cooks the pinned scene,
verifies the linked rule match, then admits an unmatched rule edit. The already
running host still accepts a tap and publishes the changed state to its replica.
It uses the existing `GameRules` and authority path; no second multiplayer rules
implementation is introduced. Source and cook crates are test dependencies of
the mobile example, so normal mobile artifact dependencies remain unchanged.

The F4 check has one bounded verified store read; it accepts a fixed-size digest
and does not require whole source files in production binary memory. The test
computes its embedded-file digest once. No whole-file heap copy or per-tick work
is added. No elapsed-time or RSS
measurement is claimed for this small one-time code check. A complete ready build
must bind the exact executable bytes, toolchain/dependency closure, target and
product manifest to `BuildRevision`, retain/load the closure, then start a fresh
world epoch from that build. The test's `create_host` call is a scoped integration
fixture, not that ready-build API. Save/restore and clean recipient rebuild remain
open in the [build plan](../../BUILD-PLAN.md).
