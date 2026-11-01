# I1 prepared rule boundary and terminal fault qualification

1 November 2026. The game rule now decides and prepares complete edits through read-only `&World` access. The authority validates and applies each decision in canonical order, so a later action sees earlier same-tick state. A zero/one/two-edit enum avoids an owned batch vector for small actions; two-edit validation uses inline bookkeeping. Larger batches still use an owned vector. This migrates the prototype rule path; it does **not** qualify the publication byte gate.

`RUSTC_WRAPPER= ./dev check` passes, including the unchanged standalone/match, two-peer, delayed-replica and hosted callback fixtures. New cases prove that a second invalid edit in one prepared batch applies neither edit; an invalid second action after a valid earlier same-tick action returns `TerminalGameContract`, forbids baseline/input/acknowledgement and cannot replay after another `step`. The authority is poisoned before mutation so a caught game unwind also cannot resume. A hosted fault leaves the replica at its last complete tick. When a later tick fails during one catch-up callback, the callback returns an earlier committed publication plus an explicit terminal fault rather than dropping that publication.

The release fixture runs 10,000 measured ticks after 1,000 warmups on macOS arm64, 64 fixed-size `Counter(u64)` objects, one input per tick, with final authority/replica equality. The one-edit and two-edit paths both include input admission, authority publication, replica admission and result acknowledgement. Two local runs (`tmp/publication-memory/prepared-rule-cost-b.txt` and `-c.txt`) gave:

| Path | Tick p50 / p95 / p99 | Complete p50 / p95 / p99 | Work per tick |
| --- | --- | --- | --- |
| One edit | 416–500 / 458–542 / 459–583 ns | 708–834 / 750–917 / 792–959 ns | 1 page, 32 objects, 1 group object |
| Two edits | 458–542 / 542–625 / 625–750 ns | 708–834 / 792–958 / 875–1,042 ns | 1 page, 32 objects, 2 group objects |

The pair addresses neighboring IDs, usually in the same 32-object page. The second edit adds validation and one group object but does not necessarily add a copied page. These local distributions overlap; no speedup or device budget is inferred. `Instant` resolution and scheduler noise limit sub-microsecond comparisons. The fixture does not count allocator calls or qualify arbitrary game payloads.

The real 120 Hz callback fixture was run once before and twice after catching authority-step unwinds in the host. All three runs committed 241 ticks over 480 callbacks with no normal budget overrun; host callback p95 was 6 µs before and 7–9 µs after, while p99 was 14 µs in all three. This noisy wall-time fixture cannot isolate the cost of unwind handling or certify a callback deadline. Its correctness benefit is exercised by the caught-unwind catch-up test.

The remaining gate must enforce per-action edit, spawn, inline entity-size and owned payload bounds; reserve all due actions and replica/output peaks before the first edit; and charge mutable indexes, group buffers, retained outcomes, baselines and replica state. The current generic `Entity: Clone` and `PreparedEdits::Many` still admit unbounded allocations. A large inline entity can consume stack even though the small fixture is fast. The parent [memory work item](../../BUILD-PLAN.md) remains open.
