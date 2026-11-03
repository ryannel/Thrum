# I1 charged authority lease floor

> Status after the code-quality cleanup: the I1 timing examples cited here were removed from the active tree. Their historical measurements and source remain in Git; quoted example commands are no longer current commands.

1 November 2026. A charged authority now acquires one logical lease floor before tick-zero publication. It covers the shared floor owner and the requested storage for eight snapshot lease tokens. The floor charge stays with the authority and every live snapshot token, so dropping the authority cannot release it while an old reader survives. The fixed inline weak-token directory admits at most eight live publication tokens. A clean tick creates a new token using the already charged floor and needs no new pool reservation for that token. The shared eight-token bound has one crate-private constant.

The constructor reserves the floor and the tick-zero page/root peak together. If either cannot be reserved, activation fails without publishing. A private charged-world entry checks that the floor and page/root reservation belong to the same pool before publication. The direct `World::try_publish_charged` primitive still charges pages/roots only and remains outside this authority lease guarantee.

The integration fixture fills every remaining pool byte after charged authority activation and advances 16 clean ticks. Each tick publishes once while pool usage remains at its limit. After the competing reservation and authority drop, a held snapshot keeps the floor charge alive; its final drop returns the pool to zero. A world unit test rejects a foreign-pool floor before publication without spending reservation or clearing dirty state. `RUSTC_WRAPPER= ./dev check` passes 19 core, 6 clock and 41 game integration tests.

The `release fixture` (historical `git show 84d7673:crates/thrum-core/examples/i1_cost.rs`) now times 100 batches of 1,000 clean authority ticks after 10 warmup batches, dividing batch elapsed time by tick count to reduce timer quantization. The same benchmark source was run in a temporary worktree at pre-floor commit `cce1d1c` and on the floor path after `9f67aec`. Three local macOS/aarch64 runs each gave these median ns/tick values:

| Path | Pre-floor | With floor |
| --- | --- | --- |
| Charged authority | 46 / 51 / 47 | 71 / 52 / 51 |
| Uncharged authority | 39 / 42 / 39 | 55 / 43 / 43 |

The first with-floor run was slower on both charged and uncharged paths. The remaining runs are close enough that this fixture does not isolate a reliable floor overhead. Source inspection shows one extra shared-owner refcount on charged publication, and every tick still allocates an `Arc` token; precharging capacity does not remove that CPU or allocator work. The floor does not charge group results, host delivery, replica storage or future input execution credit. Complete host latency and allocator counts remain open.
