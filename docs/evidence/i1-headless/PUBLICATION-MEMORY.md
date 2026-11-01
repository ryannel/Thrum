# I1 publication-memory preparation

1 November 2026. This is a focused allocation and cost checkpoint, **not** whole-process memory qualification. The [governing design](../../design/I1-HEADLESS.md#publication-memory-admission-gate) selects precommit byte admission across world publications and replicas. The current runtime still has only an eight-bundle count cap; arbitrary `Clone` payloads and structural writes prevent a defensible hard byte limit.

## Direct new-page allocation

Previously, when a world grew from `P` to `P + D` immutable pages, `World::publish` built `D` temporary empty `Arc` pages, then replaced each with its final page. The selected path appends each final page directly. This removes one discarded `Arc` allocation per new page by code inspection: 2 at 64 objects, 128 at 4,096, and 512 at 16,384 on initial publication with 32 slots per page. It does not remove final page clones, the new root, game-owned payload allocations, group copies or retained readers.

The growth regression test publishes three pages, extends a partial page and adds another, then checks old snapshot isolation and complete records in both versions. `RUSTC_WRAPPER= ./dev check` exercises this with the authority and replica tests. The read-only review found no actionable defect in this change or its scope claims.

## Local initial-publication cost

Run `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_growth_cost`. Each case preloads an empty world outside the timed region, then times `publish(0)` for 100 measured iterations after 10 warmups. Every publication checks page/object counts and complete records after timing. Two selected runs: [A](growth-a.txt), [B](growth-b.txt). Their p50 initial-publication times were 1.83–2.38 µs for 64 objects, 80.3–86.1 µs for 4,096, and 311–328 µs for 16,384. The 16,384-object p99 varied from 439 to 562 µs. These runs measure the selected path only; they do not establish a before/after speedup or count physical allocations. World construction, game rules, replica admission, held readers and process residency are outside the timed region.

Measured source SHA-256: `world.rs` `a83661c24b2a2e16f52472fb2a3eb9693a7d56af5df3914b1de7795607e0b6bd`; fixture `i1_growth_cost.rs` `723aeac0619120a1bbeef24aeac8a242d8874c805e6cee99bc2fd70793b1a356`. Rust 1.99.1 release on local macOS arm64. Hardware model, thermal state and CPU allocation counters were unavailable. This measurement does not qualify desktop/browser/phone budgets or the proposed byte pool.
