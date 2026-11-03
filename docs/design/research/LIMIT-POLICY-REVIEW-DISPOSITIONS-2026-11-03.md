# Cross-owner limit review dispositions — 3 November 2026

Three bounded independent commissions challenged the selected policy against
current Thrum contracts, code, affected consumers and primary external sources:
[source/storage](LIMIT-REVIEW-SOURCE-STORAGE-2026-11-03.md),
[runtime/simulation/network/terrain](LIMIT-REVIEW-RUNTIME-SIM-2026-11-03.md), and
[render/platform](LIMIT-REVIEW-RENDER-PLATFORM-2026-11-03.md). They inspected
`e3ce277` after reading the policy, so this is an informed adversarial review,
not a blind derivation or a new platform qualification. No renderer, solver,
transport or device workload was run by these reviewers.

## Decision

Retain the [cross-owner policy](HOLISTIC-LIMIT-POLICY-2026-11-03.md): exact
publication and required-consumer semantics remain invariant; owner-specific
supported capacities and operational controls remain finite and visible;
device performance and appearance remain measured claims. The review found no
reason to restore an unproved 8 GiB APFS physical guarantee or to create a
universal limits service. It did find one consequential render-design gap and
several precise implementation/qualification gates. A corrected document
closes only its design wording; it does not mark the corresponding code or
runtime behavior complete.

| Finding | Disposition in this checkpoint | Remaining executable gate |
| --- | --- | --- |
| P1: WGSL runtime overflow/NaN may yield an indeterminate result, so a post-operation nonfinite test cannot prove the fault bit | [Render specification](../RENDER-SPEC.md#4-scene-colour-and-light-quantities) now requires input/intermediate range derivation and checks before dangerous arithmetic, including pre-exposure and f16 storage. | Independent high-precision boundary cases and pinned-backend shader tests must precede any portable arithmetic-fault claim. |
| P2: a partial large SQLite blob is currently deleted in one transaction after a fixed 1 MiB WAL allowance | [Foundation](../FOUNDATION-SPEC.md#3-native-admission-and-browser-runtime-persistence) selects bounded prefix verification/resume plus interruptible bounded cleanup on mismatch. | Replace `begin_blob`'s whole-partial `DELETE`; interrupt after many chunks and during retry, measure per-slice WAL/hold time and preserve exact A/B outcomes. |
| P2: source staging prices the whole tree and accepted preflight charges the entire unused WAL stop, causing false refusals | Keep conservative live checks and exact storage-fault recovery. The [build plan](../../BUILD-PLAN.md#cross-owner-limits-and-resource-policy) separates marginal work from configured ceilings. | Compare no-op/small/large edits in a retained near-target store; report missing bytes, expected work, existing debt and observation separately. |
| P2: simultaneous same-volume store/materialization/compiler jobs have no shared claim or live output guard | Keep T2's narrow per-volume coordinator and owner-scoped claims in the policy. | Make two individually affordable but jointly unaffordable jobs defer correctly; inject mid-copy space fault and prove old source/ready build survive. |
| P2: source acceptance cannot yet return quantitative estimate/observation and qualified durability details | Keep `commitGuarantee` and exact root/outcome distinct from a small operational diagnostic receipt. | Exercise each pressure/fault class through the public API; state observation scope and retry without implying physical reservation or power-loss proof from PRAGMA alone. |
| P2: ordinary headless peer registration has no aggregate membership bound outside optional credit profiles | Record an explicit ordinary owner profile or bounded outer owner before runtime promotion. | Reject P+1 peers before sequence/use growth; prove reconnect, pending/result pressure and later retirement semantics. |
| P3: networking family wording allowed W1 snapshots to remain leased until remote queue/age pressure | [Networking module](../modules/NETWORKING.md) now requires a bounded N1-owned projection and W1 release before remote send/ACK, matching C13. | Stall remote ACK for more than eight ticks and show W1 lease count and authority progress are independent of it. |
| P2 proof gate: independent N1/tick/physics/water/nav maxima cannot be treated as one desktop CPU guarantee | Keep `HostBudget` as a narrow foundation and derive complete due-batch cost for the selected recipe. | Measure two/four due ticks with selected N1 bursts, P3/P5 casts and E1 work; report overload without partial physical tick or hidden lost time. |
| P2 proof gate: final-triangle terrain evidence does not certify every activated consumer | Retain the current restricted certificate claim and its explicit missing closure. | Bind admitted source, geometry and P3/P5/E1/V2 consumer-specific support/error/topology before region activation. |
| P2: a prepared render fallback pipeline does not produce a complete current-frame image | The [R3 failure design](R3-OPTICS-AND-TRANSPARENCY-DESIGN-2026-11-02.md) selects an independent diagnostic/loading path for its first late-overflow proof; richer fallback needs its own encoded graph and failed-frame cost. | Force late GPU overflow with delayed readback; verify the same submission displays complete current output or diagnostic and writes no invalid history. |
| P2: concurrent GPU frames lack a selected allocation/reuse schedule | The [build plan](../../BUILD-PLAN.md#cross-owner-limits-and-resource-policy) makes an R0 lifetime schedule the first V1 proof. | Hold an old submission across new frame, resize and replacement; account for exact allocation identities, last uses and fences. |
| P2: portable internal-pixel targets do not bound output/DPR/capture allocations | Keep exact output extent/rounding policy as an explicit pre-render selection, not an inferred 720p cap. | Exercise fractional DPR, resize, zero-size, large viewport and capture with exact high-water and quality review. |
| P2: CPU submission metadata could advance while GPU rejects a temporal sample | [Render specification](../RENDER-SPEC.md#8-r0-first-rendered-game-contract-and-proof) separates submitted and accepted-history metadata; initial temporal uncertainty resets history. | Reject N after accepted N−1, delay readback, then render N+1 with changed pose/exposure and prove coherent history identity. |

The first safe path remains deliberately small: one accepted R0 source/build,
one exact output policy, a concrete V1 lifetime schedule and a diagnostic
failure surface. R3, temporal history, integrated water/navigation and named
device claims follow only when their selected interactions and costs are
proved. This does not lower Thrum's visual goal; it prevents a narrow success
from being promoted as a broader capability.

## External evidence and its limit

The current [SQLite WAL documentation](https://www.sqlite.org/wal.html)
explains checkpoint interference by readers and large-transaction WAL growth;
it supports bounded retry work and explicit checkpoint debt, not Thrum's
particular 52/2/2/8 GiB values. [WGSL floating-point evaluation](https://gpuweb.github.io/gpuweb/wgsl/#floating-point-evaluation)
requires precondition/range reasoning for portable shader diagnostics.
[WebGPU device limits](https://www.w3.org/TR/webgpu/#limits) are selected per
requested device and do not bound a game's complete GPU residency.
[WHATWG WebSockets](https://websockets.spec.whatwg.org/) and
[Rapier integration parameters](https://rapier.rs/docs/user_guides/rust/integration_parameters/)
help separate application queue control from browser buffering and solver
timestep from iteration count. These are primary mechanism references, not
Thrum implementation or performance results. The three reviews record the
other considered sources, alternatives and scoped rejection cases.
