# Foundation consolidation review

2 November 2026. Reviewed `codex/publication-memory` at `0a690d1` against the central work list, C4 contracts, Foundation specification, selected evidence and production crate boundaries. This is a scope and integration review before moving the branch to `main`; it is not a new benchmark or a capability promotion.

| User-visible path | Current evidence | Open integration gate |
| --- | --- | --- |
| Local headless game | A player action reaches the shared rule, authority, published world and replica. The selected clock and host run local and two-peer in-memory fixtures; sustained synthetic-clock callback cost and owner lifetimes were measured. | A representative full game, real transport, target devices and presentation remain separate qualifications. The 60 Hz default world clock is not a render ceiling. |
| Editable source | Declared files can be inventoried, staged as exact bytes and checked against a provisional source-carried registry, local references and manifest identity. A 256-descriptor candidate with one game record has a local release cost/RSS sample. | Broader F3/external dependency closure, editor application, all-type and maximum-edge workload, combined edit-to-ready memory and latency. A candidate has no accepted-source authority. |
| Durable source | Test-only SQLite fixtures connect verified blobs, an authenticated current-root history, candidate validation, root/outcome transaction, read pins, mark-only GC and several interruption/space/WAL cases. | A production accepted-source API, coordinated file application, certified physical peak/free-space guard, complete pin/collection lifecycle and recovery qualification. `sqlite_store` is still compiled only under `cfg(test)`. |
| Run and save | The headless authority can run fixture rules without rendering or network I/O. | Cook and launch only from a pinned admitted revision; save a separate private authority root and restore it against its exact source/build closure. |

The selected headless path is a sound starting point for source/run integration. Its checks and local timings do not establish whole-engine or named-device performance. The source-store proofs are extensive but currently cannot be called by a production game. No review finding required unchecking a narrowly worded proof; the parent **Editable game and save** task stays open. The maturity ledger keeps broad capability status separate from repository checks and has been corrected to reflect the retired universal process-memory cap. Low measured allocation, RSS and pressure behavior remain performance goals; the optional bounded host pool retains its scoped owner-byte guarantee.

## Next complete interaction

1. Close the selected F3 source and durability contract for the first bounded game: exact staged-byte/dependency closure, coordinated working-file application, production root/outcome admission, physical-space guard and retained-reader/collector behavior. Preserve old accepted roots and retry outcomes on refusal or interruption.
2. Consume a pinned admitted revision through cook and local run, then save/restart/restore a private authoritative checkpoint. Exercise failed edit/build, corrupt save, stale source/build and final lease release in the same integration fixture. Local and later multiplayer runs must use the same game rules and distinct source, run and save authorities.
3. Measure the complete edit-to-ready path and sustained small-game callbacks on a named machine, including allocation/RSS, latency tails and pressure. Add all-type/edge and real-transport workloads before claiming general source or multiplayer performance. Keep phone work at the already selected portability foundation until a platform game needs deeper qualification.

## Verification scope

At the reviewed checkpoint, `RUSTC_WRAPPER= ./dev check`, `RUSTC_WRAPPER= ./dev check-web` and `RUSTC_WRAPPER= ./dev check-mobile-core` passed. The latter two compile the portable core; neither runs a browser or phone app. The independent read-only review found no merge-blocking code or C4 authority contradiction in this bounded scope. It did not rerun every historical benchmark or qualify power-loss durability, device behavior, visuals or a shipping game.
