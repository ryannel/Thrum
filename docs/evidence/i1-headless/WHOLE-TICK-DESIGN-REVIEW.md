# Whole-tick publication admission design challenge

1 November 2026. A read-only review challenged the [whole-tick owner plan](../../design/I1-HEADLESS.md#whole-tick-byte-gate-bounded-profile-and-owner-plan) against C02/C13 and the authority, world, replica and pool implementations. This is a design review, not an implementation or byte qualification result.

| Finding | Resolution in the governing design |
| --- | --- |
| Arbitrary `Copy` still permits external references or pointers; even its custom `Clone` can allocate. A value may also carry a charged product from another pool. | The qualified path requires closed engine-owned structural storage, engine-controlled inline duplication and pool provenance checks for every charged owner. Arbitrary game `Copy`/`Clone` cannot qualify by a reported estimate. |
| `decide` can create a retained rejection before `prepare`; a prepare-only budget context misses it. | The budgeted rule context covers both callbacks and every retained result/event value. Temporary private game scratch has a separate work budget and is excluded from the publication-pool claim. |
| Clean idle ticks still create lease and delivery owners. | Authority construction or activation must precharge a finite reusable idle lease/delivery floor before accepting input; idle cost remains O(1) and must be measured. The proof includes a zero-pending world while other consumers fill all unprotected capacity. |
| Charged pending/history payloads can fill the pool so the tick required to retire them cannot run. Bounded queue lengths alone do not ensure progress. | Input admission protects future execution/output credits before consuming a sequence. Credits for later pending work remain reserved as earlier work commits; retained outputs take ownership of spent credits. Remote queues use separately bounded capacity and cannot spend this reserve. Saturated pending/history and competing-world cases must prove progress or explicit refusal before sequencing. |

The reviewer found no remaining contradiction in the revised owner and liveness plan. The target still needs concrete buffer/lease representations, complete allocation accounting, a working two-peer proof and measured cost before the parent memory task can close. Earlier snapshot-only charged authority evidence remains valid only for pages and roots.
