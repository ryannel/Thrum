# I1 charged page and root owners

1 November 2026. This checkpoint charges the immutable page and root allocations made by `World::try_publish_charged`. A page or root owns one `PublicationCharge` inside its shared `Arc`; unchanged pages are shared without another charge, and a replaced page releases its charge after the last old snapshot drops. Two worlds using one pool compete for the same limit. The charged path rejects a too-small or foreign reservation and refuses to mix with a world that has already published through the uncharged prototype path. A charged world cannot later call the uncharged `publish` path.

The retained layout uses an exact-length boxed slice inside each page/root owner, avoiding retained spare Vec capacity. Each new page/root has a boxed data allocation and an Arc owner allocation. `required_charged_publication_bytes` returns a conservative logical **reservation peak**, including a possible Vec-to-Box staging overlap and the constructor's initial empty root. Only retained owner bytes transfer to lifetime charges; unused peak allowance returns when the reservation drops. The logical budget excludes allocator metadata, possible allocator capacity above a requested staging length, heap storage inside arbitrary game-owned `Clone` values, mutable world indexes, snapshot lease tokens, groups, outcomes, baselines and replica storage.

`RUSTC_WRAPPER= ./dev check` passes old-snapshot isolation, final-reader release, two-world pool competition, refusal without publication, and wrong-pool/mixed-mode tests alongside the existing authority/replica suite. Source identities: `world.rs` SHA-256 `b040d8908e805b317da02e50ae0f7e87e8bb7b5ab64fb16bbe1a8e3a6dc2df6c`; `publication_pool.rs` SHA-256 `7cdcedcff8d51d813c343a86013a17b56af4744543136df47fe92d7c5c33f896`.

## Local cost

`RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_charged_pages_cost` measures 1,000 ticks after 100 warmups per case on local macOS arm64, Rust 1.99.1 release. Fixture SHA-256: `1cc925570d76f3ec3dee07281aff76cc1e1d7d291709c7fdb2d441e7bc3fe9eb`. It mutates 0%, 1% or 25% of 64, 4,096 or 16,384 objects, keeps one previous snapshot, times exact post-mutation admission planning separately from charged publication, and asserts pool release at the end. Two sequential runs are [A](charged-pages-a.txt) and [B](charged-pages-b.txt). The fixture does not exercise game-owned rules, host callbacks, replica admission, or a precommit reservation.

| Workload | Admission p50 | Charged publish p50 | Retained logical bytes | Conservative peak |
| --- | ---: | ---: | ---: | ---: |
| 4,096 objects, 1% changed | 83–84 ns | 4.71–6.17 µs | 269,360 | 441,360 |
| 4,096 objects, 25% changed | 125 ns | 25.75–26.58 µs | 269,360 | 801,888 |
| 16,384 objects, 1% changed | 291–292 ns | 19.33–20.00 µs | 1,077,296 | 1,765,152 |
| 16,384 objects, 25% changed | 375 ns | 103.13–103.50 µs | 1,077,296 | 3,207,264 |

Zero-churn charged publication was about 41–42 ns p50 and did not create a new page/root owner. These values are local code-path measurements, not controlled before/after speedups or physical resident-memory measurements. The `i1_cost` prototype fixture before this owner-layout change and after it showed similar publication p50 values across the 4,096 and 16,384-object cases, but the runs were not an isolated paired comparison. The charged path's extra reservation scan and charge transfers need an integrated update-path measurement before a performance claim.

This method currently runs after game edits in the example. A capacity refusal there leaves the current immutable snapshot unchanged but cannot undo already-mutated authoritative state. The [whole gate](../../design/I1-HEADLESS.md#publication-memory-admission-gate) therefore remains open: the host must reserve a conservative full-tick peak before its first rule commit and account for every required published owner and bounded game payload.
