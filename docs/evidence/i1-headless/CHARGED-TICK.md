# Charged live group owner

1 November 2026. This is an authority-side live group storage proof, not the complete publication-memory gate. `step_charged` is available only on the sealed charged authority path. The existing `step` and host callback still return prototype groups.

Before a rule callback or world edit, the authority reserves one checked peak for snapshot pages/roots and all four group columns. The change bound is the number of existing unpublished stable IDs plus the admitted maximum prepared edits for all due inputs. Results and events each have at most one entry per due input; watermarks have one entry per registered peer. Structural spawn/despawn pairs can reduce the final group but cannot exceed this bound. The world requests its change vector with the exact changed-ID count; the authority requests result, event and watermark vectors with explicit capacities. After publication, those vectors move into charged shared owners without copying records again. The reservation remains live until each owner takes its charge.

A no-input tick reuses the last charged watermark owner while peer progress is unchanged. Peer registration and reconnect invalidate it; a prototype step also invalidates it. The authority holds one cached owner, and an old group reader can keep an earlier owner alive. This avoids a new watermark allocation on steady idle ticks; the first tick after a peer change still needs capacity. The precharged idle floor does not yet include that first group owner.

The two-peer fixture first leaves only the snapshot-page peak free and verifies that group admission refuses the tick before mutation, without faulting the authority. After capacity returns, one action commits an event and one receives a game rejection; the group carries both results and both watermarks. A shared reader retains the group charge. With all remaining pool bytes occupied, a clean next tick still advances using the cached watermark owner. A later due input updates one peer watermark and the last owner releases its charge after authority and readers drop. An unqualified `step_charged` call returns a configuration error without poisoning the prototype authority. A world unit test covers the additive change bound with 65 cancelling spawn/despawn pairs and a later tight bound.

The [release fixture](../../../crates/thrum-core/examples/i1_baseline_cost.rs) also measures 1,000 sequential clean ticks per sample after 20 warmups, with 100 samples. Each operation drops its output. Two local macOS/aarch64 release runs produced these p50 **batch** times in nanoseconds:

| World records | Prototype batch, run 1 / 2 | Charged batch, run 1 / 2 |
| ---: | ---: | ---: |
| 64 | 128,125 / 80,208 | 98,333 / 72,875 |
| 4,096 | 74,292 / 74,584 | 68,833 / 68,917 |

The runs vary, especially at 64 records. They establish that this local steady idle path has no clear cost increase after watermark reuse; they do not isolate the cache's benefit or predict device performance. The fixture runs prototype ticks before charged ticks on the same authority, keeps baseline objects live, and has no due actions. Changed-tick construction, required replica admission and the complete host callback still need direct measurement. Mutable world indexes, completed history, future-input credits and host delivery remain outside this pool gate.
