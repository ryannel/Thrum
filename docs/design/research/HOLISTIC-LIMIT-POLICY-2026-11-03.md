# Thrum limit policy — cross-owner review, 3 November 2026

This review reconciles numerical limits across source, build, runtime,
simulation, environment, render, network and delivery at inspected main
`0e70299`. The [disk provenance](SOURCE-LIMIT-PROVENANCE-2026-11-03.md)
traces the former hard 8 GiB floor. This is a design selection, not a claim of
whole-engine implementation, device performance or a measured safe threshold.

## Classify the promise before choosing a number

| Kind | Rule and change policy | Authority |
| --- | --- | --- |
| Semantic and publication invariant | Complete source/root/outcome, one world authority, retained required bytes, conservation receipts and no silent required-content loss. A resource setting cannot waive these. Changed meaning requires the affected semantic contract/schema/protocol revision. | Named C01–C13 state owner. |
| Encoding/ABI | Product, wire and shader layouts and versioned source definitions must be interpreted exactly. Change through a versioned reader/migration. | F3/F4, N1 or V1. |
| Model validity/error | Numerical or visual provider domain, units, error and outside-regime outcome. Enlarging a CFL, solver, shore or optical envelope needs owner analysis and affected-consumer proof. | P3/P5/E1/V3 providers; W2/F4 own terrain geometry/product certificates, with T1 exposing authoring controls and diagnostics. |
| Supported capacity | Finite parser, queue, snapshot, light-list, pipeline or work count accepted by one implementation profile. A larger allowance with unchanged meaning needs a qualified profile, not automatically a source-format migration. | Consuming owner, composed by F2. |
| Operational policy | Disk headroom, WAL watermarks, speculation, queue pressure and quality hysteresis. Configurable and observed; cannot promise OS or driver allocation. | T2/F3, F1, V1 or N1 in their own resource domains. |
| Qualification target | Frame, startup, memory, visual error and named device support. Select a workload/device and measure the integrated result; a target is neither an input cap nor a hardware guarantee. | T2 evaluator and affected owners. |

Some numbers have more than one role. The 160-byte light product is an encoding
rule while its cooker also needs bounded work. Water conservation is semantic;
its numerical tolerance is derived for one arithmetic path. State both roles.
Do not promote an internal constant or percentage into an engine-wide law.

## Resolve once, enforce with the owner

F2 resolves a compact versioned recipe from required game semantics, providers,
validity domains, implementation capacities, target/backend features and
selected quality. Each relevant limit has an owner, unit, timebase, scope and
failure. The recipe is a composed view, not a new allocator or universal
constraint language. Owners report actual capabilities and reject incompatible
combinations. F2 cannot silently change gameplay, material meaning or physical
support to fit a device.

Keep identity scopes separate. `SourceRevision` names exact admitted bytes;
increasing a parser allowance or disk target cannot change it for identical
bytes. Schema and N1
gameplay locks include actual meaning and authoritative timebase. F4 `CookKey`
includes typed values plus declared meaning/definition, dependency, cooker,
evaluator, toolchain and target-cache class revisions needed for compatibility,
even when output bytes happen to coincide. `BlobHash` alone identifies exact
product bytes. The current built-in environment definition embeds record and
opaque capacity ceilings; widening those ceilings changes its definition hash
and dependent cook key, even if a selected light's bytes do not change. Only
an allowance outside the hashed definition/dependency closure can change
without that product-key effect. The build binding separately names accepted source and
target/profile. Queue
size, free-space target and evidence revisions are diagnostics/policy, not
automatic source, product or network hashes. A valid F3 snapshot can remain
accepted when one phone renderer or F4 product is unsupported; a failed build
leaves the earlier ready build.

One coordinator reconciles claims within each shared resource domain:

| Domain | Coordinator and account |
| --- | --- |
| Native disk/build | T2 coordinates cooperating jobs per volume; F3 owns exact admission/recovery; F4 supplies product and materialization demand. Count known old/new retained closures, unique missing blobs, stage, WAL/main/checkpoint debt, compiler output and owned scratch once. Estimate and observe separately; `ENOSPC`/I/O resolves exact old-or-new state. No APFS physical-floor promise. |
| Runtime CPU/owner bytes | F1 schedules due ticks and optional work and reconciles the whole-scene critical path and logical residency. An optional bounded pool guarantees only its charged owner bytes; ordinary native game code has no universal hard RSS cap. Required local readers backpressure rather than reusing live pages. |
| GPU/frame | V1 owns allocations, resource lifetimes, fences and pass graph; V2/V3/V4 contribute demand; F1 accounts whole-frame timing. Charge simultaneous views, frames in flight, histories, uploads, transients, old/new overlap and pending fences. A per-pass subtotal is insufficient. |
| Network | N1 owns bounded projections, wire/peer queues and resync/disconnect; W1 owns committed world state and F1 coordinates time/publication. Release world snapshots after constructing owned projections, before remote ACK; a slow peer cannot pin authority snapshots or stall ticks. |

Known shared logical allocations count once while every consumer lease/fence
reason remains visible. Report logical owner bytes, allocator estimates and
observed physical/RSS/device bytes separately. No logical reservation proves an
OS allocation ceiling. Exact safety, finite queues, dependency readiness and
lease-safe retirement apply even where strong precharged progress is optional.

On pressure, first remove truly unnecessary/duplicate work, then evict only
unleased derived data, defer speculation, serialize work that can be chunked,
select an already prepared acceptable visual tier, show loading/stop-load, or
reject. Never truncate a required source, light, collision support, water
receipt, network dependency group or current GPU output. An outside-regime
physical result is not a cheaper visual tier. A substitute must be declared,
cooked and ready for the **current device generation**, preserve its cues and
required consumers, and fit old/new overlap. Commit one validated generation
at its safe barrier, invalidate affected histories, then retire disabled work
after its last lease/fence. Report owner, stage, profile,
quantity, unit/timebase, requested, allowed, estimate versus observation,
outcome and a supported next action. Use owner-specific checked counters and a
small common diagnostic envelope; do not create a new public limits crate or
per-frame dynamic registry.

## Concrete conflicts and decisions

1. **Optional preparation:** BASELINE allocates 10% of the CPU critical-path
   envelope (0.5/0.8/0.7 ms desktop/browser/mobile); PLATFORM separately says
   10% of frame period (1.67/3.33 ms). BASELINE owns the recipe allocation.
   Actual remaining deadline slack can further restrict it, never enlarge it.
   Required activation shows loading instead of using optional work time.
2. **Source capacity composition:** 100,000 tree entries has a separate 64 MiB
   encoded-manifest store bound; 4,096 records, 256 schemas and 65,536 required
   edges are independent. Publish all dimensions and their aggregate behavior.
   A larger processing allowance with unchanged source meaning needs profile
   proof, not a mandatory file-format change. Definition-encoded ceilings
   still revise that hashed definition and dependent cook key.
3. **Disk estimates:** 52/2/2/8 GiB are configurable planning targets. Staging
   may price the whole tree while admission deduplicates blobs; a tiny edit can
   be charged the entire remaining WAL stop allowance. Improve marginal
   estimates and diagnostics while retaining live checks and fault recovery.
   The older strict APFS floor is not a first-profile admission gate.
4. **Timebase:** C02 allows a game-selected tick, but initial P3/E1 integrates
   `1/60 s` and N1 windows assume 60 Hz. F2 rejects incompatible recipes or
   selects a separately derived provider/protocol. World seconds, callback
   milliseconds, transport deadlines and retention age stay distinct.
5. **Whole cost:** R3's 38.008 MiB 720p optical subtotal excludes full graph and
   frames-in-flight lifetimes. Light, pipeline, water-step and navigation caps
   also lack complete work multipliers. Derive concurrent demand and checked
   overflow before claiming target readiness. Keep source validity, numerical
   validity and visual quality as separate results.
6. **Prototype restrictions:** test-only provisional source/GC/F4 caps, the
   current eight pending inputs/snapshot bundles and the mobile shell's 16
   touch slots are specific implementation profiles, not SDK-wide ceilings.
   Expose actual support and a larger-profile route when needed.
7. **Output extent:** browser CSS/DPR and phone 720p-equivalent targets lack an
   exact output extent/rounding/capture policy. Internal pixels alone do not
   bound swap, UI and filter storage; F2/V1/V4 must resolve both dimensions.

## Owner adoption map

This is the first implementation register. The numbers below retain their
owning designs; the table states what an implementer must publish and prove
before treating them as a composed game/profile guarantee. An isolated test
or an analytical worksheet does not mark the integrated row complete.

| Owner and representative existing limits | Present status | Required implementation and proof |
| --- | --- | --- |
| F3 source: 100,000 entries, 16 MiB JSON record, 1,000,000 nodes, 64 depth, 16 GiB blob; accepted manifest 64 MiB; semantic 4,096 records/256 schemas/65,536 edges | Parser/accepted-store gates exist; dimensions can conflict and parser peak is not fully qualified. | Publish the whole supported profile and the specific exceeded dimension. Test dense combinations and retained old/new pinned closures; qualify wider capacities without changing exact source identity. |
| T2/F3 disk: default 52/2/2/8 GiB main/WAL/temp/observed free and checkpoint watermarks | Configurable plans and exact fault paths exist; projected versus actual peak and cooperating job coordination are incomplete. | Price marginal missing bytes, WAL/checkpoint debt, materialization and concurrent retained work per volume; inject every write/sync/space fault, including uncertain commit. Show the estimate class and observed overrun. |
| F4 cook/build: per-input/output guards, 160-byte selected light product, source pin, product/build keys | Headless cook from the accepted pin exists; light bridge is historical provisional proof; complete ready publication is open. | Use one authenticated selected source across products; bound product/work closure; verify meaning and target keys; retain inputs/products through atomic ready publication and stale/failure recovery. |
| F1/W1: 60 Hz default, four ticks/callback, eight queued, eight live snapshot bundles; optional charged publication pool | Clock and bounded headless host run; ordinary authority still has fixed small count limits; no universal process RSS bound. | Expose chosen capacities before run, prove tick/progress/reader backpressure with other owners active, and measure one complete CPU critical path and logical/observed peak. |
| P3/P5: initial 1/60 s physics, solver/substep and navigation query budgets, 17 steering casts per agent | Selected provider designs; P3/P5 implementation and combined gameplay proof unbuilt. | Bind dt/space/geometry certificates at F2, distinguish search iterations from P3 casts, then measure and refuse an unsupported full due batch without partial world publication. |
| E1 water/rain: terrace 8,192 cells/four steps, model and residual envelopes; 32,768 exposure rows/tick | Model design and restricted proofs, not integrated runtime qualification. | Preserve mass/source receipts and outside-regime refusal, compose water/exposure work only when selected, and verify staged candidate/receipt refusal before commit plus terminal-fault/checkpoint recovery after unexpected mutation. |
| W2/T1 terrain: final-triangle, bed/shore/topology certificates and consumer support | Offline evidence covers selected cases; runtime source-to-geometry activation is open. | Bind admitted source to actual final geometry and terrain product once; carry error/support certificates to P3/P5/E1/V2 and reject a missing or invalid certificate before activation. |
| N1: 64 KiB message, 8 KiB dependency group, 256-result window, peer queues/time windows | Wire/retention design; full adapter unbuilt. I1 result/input counts are narrower in-memory profile gates. | Publish semantic tick policy separately from wire capacity, reject whole oversized groups, build owned peer projections, and prove slow ACK cannot retain W1 snapshots or stop authority ticks. |
| F4/V1–V4 GPU: four bind groups, view/material/object ABI; light, alpha, pipeline, sample and optical work caps; two frames in flight | ABI and analytical first recipes exist; no full graph/device qualification. | Query device limits, enumerate required pipelines and complete pass DAG, charge simultaneous views/fence-held generations, detect overflow before current display/history, and prove prepared fallback and retirement. |
| I4/T2 delivery: initial download/install, startup, frame, memory and thermal targets | Worksheets and isolated shell/FXAA probes, not qualified scenes. | Run cold and sustained named-device workloads with exact resolution/DPR, content, high-water, frame tails and visual review; report supported and unsupported recipes separately. |

## Build and proof order

1. Correct duplicated optional-preparation wording and stale strict-floor
   status. Reconcile Foundation's source-capacity sentence: raising a parser or
   store allowance with unchanged bytes/meaning needs a supported-profile
   revision and cost proof; a definition-encoded bound revises that definition
   and cook key, while changed encoding/meaning needs the appropriate
   format/schema revision. State the enforced 64 MiB manifest bound beside
   noncomposing entry/record/schema/edge counts. This changes no accepted bytes.
2. D1's first resolved recipe reports semantic timebase, actual owner
   capacities and unsupported combinations. F3 admission stays independent
   of target readiness. Test that a pure operational-limit increase preserves
   source identity for unchanged bytes; external policy changes preserve a
   product key only when outside its hashed dependency closure. Changed
   definition/meaning invalidates the affected identities.
3. I2a measures F3/T2 marginal stage/admission work and makes estimate/refusal
   visible. Inject low space, write, sync, checkpoint and uncertain COMMIT;
   prove complete old-or-new root/outcome, retained pins and exact retry.
   Measure small/large edits, WAL debt and observed peaks, not a physical floor.
4. F3 supplies one selected typed source from the accepted pin; F4 shares the
   verified F3 source manifest/index across products, then builds separate
   product/build manifests and complete target demand. Retain the authenticated
   source and product closure, recheck intended source/target selection and
   build generation, and atomically publish only after every required product
   succeeds. Stale, failed or interrupted work leaves the previous ready root.
   The [provisional bridge](../../evidence/i2a/E1-CONSTANT-LIGHT-F4-LEASE-BRIDGE-2026-11-03.md)
   is a frozen earlier-checkpoint proof, not current accepted-pin qualification.
5. I1/I2n prove timebase compatibility and aggregate due work for the selected
   headless/R4 paths, without making water/navigation prerequisites for R0.
   The later integrated recipe adds navigation casts, shallow water and
   exposure **when selected**. Remote retention never inherits required
   local-reader leases.
6. D2/I3/I4 build V1's whole resource-lifetime account across views, frames
   and transitions; test device limits, light/alpha/sample overflow and
   prepared fallback. GPU-discovered overflow gates both current display and
   history **before V4 commit**; delayed readback is diagnostic only. Qualify
   visuals, frame tails, memory,
   startup and thermals on named devices before support claims.

The first implementation should be a narrow owner path with its real consumer,
not a speculative engine-wide resource package. Reconcile shared contract
changes with F3/F4/F1/V1/N1/T2 before dependent code lands.
