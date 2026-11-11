# Repository setup — 1 November 2026

The standalone Thrum scaffold was verified from its final location on this Apple-silicon Mac.
`./dev check` passed local link/anchor, skill metadata, maturity digest and crate-policy checks;
four adversarial guard tests; rustfmt; Clippy with warnings denied; and workspace compilation/tests.
The scaffold has no engine behavior tests yet. `./dev check-web` compiled the portable core for
`wasm32-unknown-unknown`; `./dev help` listed the real commands. Rust/cargo were pinned to 1.99.1.

The three repository skills also passed the skill-creator metadata validator. All 116 imported source
files retained their original hashes in Broadsides, and its Git status fingerprint was unchanged.
No source files there were modified. Import provenance records original bytes and destination paths.

This is repository/tooling evidence only. CI is configured but has not run remotely. Windows,
browser runtime, iOS and Android have not been exercised; no gameplay, networking, rendering,
performance or visual quality is qualified. The PowerShell launcher has not been executed here.

Next implementation scope is [I1 in the build plan](../plans/BRIEF.md).
