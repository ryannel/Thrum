# Import into Thrum — 1 November 2026

Plans, research, compact engine review handoffs and the two engine design skills were copied from the
Broadsides working tree. Broadsides was left unchanged. No runtime code, game assets, dependency trees,
build outputs, conversations or worktrees were imported.

[The manifest](import-manifest.json) records each original relative path, exact source SHA-256 and its
Thrum destination. Source files included uncommitted work: the recorded Git HEAD alone cannot reproduce
their bytes. Destination files are adapted working documents, not claimed byte-identical archives.
Relative links were relocated; omitted game-specific references resolve to [context records](CONTEXT.md).
Current engine requirements and decisions are retained in the brief and decision record. Historical
traveller/host studies do not supply runnable sample assets or select a host.

The maturity ledger preserves historical reviewed hashes and report metadata as claims about original
source bytes, including old `tmp/handoffs` paths. The manifest locates the relocated reports. Current
contract/module/spec/reconciliation hashes are explicitly refreshed for Thrum by `./dev refresh-design-hashes`.
This operation grants no new review, implementation, quality or platform status. The repository setup
record is separate from engine maturity. Current validation is performed by `./dev check-docs`; imported
validation counts describe their historical runs.

Adaptations are limited to standalone navigation, repository-specific instructions and superseded workflow
wording. C4 remains the selected contract. The original prototype/asset directories stay outside this repo.
