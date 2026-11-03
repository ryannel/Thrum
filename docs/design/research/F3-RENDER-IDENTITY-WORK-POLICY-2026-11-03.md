# F3 render identity work and host pressure — candidate

3 November 2026. This candidate replaces the proposed fixed
`EmbeddedIdentities` source-capacity dimension in the
[R0 render-source grammar](R0-F3-RENDER-SOURCE-GRAMMAR-2026-11-03.md).
It is a work policy for one bounded native F3 host, not a new source meaning,
an implemented admission path, or a promise that the operating system cannot
exhaust memory. C02 permits an optional bounded host profile with a shared
reservation pool for charged owner bytes; it explicitly does not impose an
engine-wide process-memory ceiling. The existing source byte, JSON node,
nesting, declared-record and required-edge bounds continue to apply.

## Why a fixed identity count is the wrong first control

An embedded UUID is source identity even when no edge targets it. F3 must
reject collisions against the complete declared namespace and resolve typed
edges, including those in unselected records. The [compact-index
probe](../../evidence/render/R0-F3-EMBEDDED-INDEX-PROBE-2026-11-03.md) has
332,016 embedded IDs but only four required edges in four bounded roots;
the [strict typed-decoder
prototype](../../evidence/render/R0-F3-TYPED-DECODER-PROTOTYPE-2026-11-03.md)
decodes the same source-shaped fixture at 24.02 MiB median peak RSS. Neither
measurement establishes a useful maximum ID count or the full F3 peak with
stage buffers and old/new pins. A fixed count chosen from either would make
otherwise valid source depend on an unproved extrapolation. It also mistakes
the transient uniqueness index for retained source ownership.

The first implementation should use a configurable **operational F3 work
allowance** in an explicitly selected native host profile. Its units are
charged live bytes owned by cooperating F3 operations, not IDs, disk space,
RSS, or a schema limit. The allowance may defer or fail a validation attempt
without declaring its source invalid. Changing this host allowance leaves
the source revision, built-in definition hashes, and semantic cook keys
unchanged. A host that cannot complete validation has not admitted the new
root; the previous selected root remains available.

## One bounded pass and explicit ownership

1. The existing bounded inventory/stage captures exact bytes. A versioned
   strict decoder validates each declared render root on authenticated staged
   bytes, including unselected roots. Its F3 projection emits compact
   `(UUID, kind, owning-render-root, source locator)` entries into an
   identity sink and `(source, field, target, expected kind, owning root)`
   required edges into an edge sink. These sinks belong to the temporary
   closure operation. `StagedRender` retains only top-level
   identity/path/definition metadata and checked closure counts/receipt
   needed by later stages; it holds no references to released chunks and
   does not retain mesh/texel bodies. The F4 projection reopens verified
   pinned bytes through the same grammar.
2. Identity entries occupy fallibly allocated, account-charged chunks. The
   exact per-entry representation, chunk size, source locator and any
   borrowed-byte lifetime must be measured and frozen with the
   implementation. `Vec::try_reserve_exact` is insufficient for a strict
   byte charge because the allocator may grant extra capacity. A strict
   charged-owner-byte profile needs a known-layout fixed slab (or another
   allocator with a proved upper allocation bound): atomically charge its
   checked layout size **before** allocation, refund on allocation failure,
   and release only after the slab is dropped. Document and test the slab's
   initialization/drop safety. Any other F3 allocation in that guarantee
   needs the same precharge protocol or a proved conservative bound; a
   best-effort post-allocation capacity report is only observation, not a
   strict pool guarantee. A full chunk is sorted by one total key
   `(UUID, canonical source path, object kind, owner UUID, field/locator)`;
   top-level and embedded entries use the same canonical locator grammar.
   A single chunk sorts directly. At the end, a bounded k-way merge detects
   adjacent duplicate UUIDs globally,
   including project/history/schema and top-level record IDs, then resolves
   typed references by merge join or bounded indexed lookup. Replace the
   current separate registry `HashSet` and record-ID sort checks with this
   one complete namespace pass. Error precedence is fixed: registry
   structure (project, schemas by canonical path, history), declared-record
   structure by canonical path, first global duplicate by the total key,
   then first unresolved/wrong-kind/wrong-root edge by canonical
   source-path/field/target order. This preserves bounded phase-local
   validation without collecting every structural error. If pressure stops the pass
   early, validation is incomplete and no claim is made about later errors.
   Required edges still spend the existing aggregate edge capacity;
   containment emits no
   fake required edge. A duplicate diagnostic names the deterministic first
   pair by this total order, independent of chunk size or discovery order;
   test three-way collisions split across chunks. All required edges must
   close before admission.
3. A host-owned operation account starts before inventory/stage capture and
   is passed through that path and the current
   `validate_staged_candidate`/`NativeSourceStore::admit` boundary. It spans
   the entire attempt through final admission outcome resolution,
   with phase-local charges and explicit transfer/release. This requires an
   API integration; no current F3 object supplies that lifetime. The account
   charges **live reserved capacity** for identity chunks, merge state,
   edges, decoder scratch, stage buffers,
   and any other F3 allocation claimed under the selected work guarantee.
   It shares an atomic reservation owner with concurrent F3 work in that
   host. Peak accounting includes overlapped old/new work and live leases
   where their bytes are actually retained. The identity chunks can be
   released after collision/closure validation and before SQLite source
   publication; old pins retain authenticated source bytes, not a permanent
   identity index. Account charges follow real allocation lifetimes, not
   estimates based on the number of source objects. The complete store path
   is covered only after its allocations have been audited; the first
   implementation may claim a narrower audited validation phase. Concurrent
   F4/T2 work is measured as process and volume overlap, not silently charged
   to this F3 memory account unless those owners explicitly join it.
4. When the operation cannot reserve a charged allocation, propagate a
   distinct host-level `ValidationIncomplete(ResourcePressure)` outcome with
   owner, validation stage, selected host profile,
   requested/charged/available bytes where known, outcome
   `validation incomplete`, and a retry/raise-allowance action. Preserve
   the old selected root and do not emit a `LimitExceeded` source-capacity
   detail or reuse `SourceStoreErrorKind::SpacePressure`, which denotes
   filesystem/WAL pressure. Pressure before the final admission transaction
   is a pending or explicitly retryable attempt under the **same**
   `(epoch,key)` and admission request; bind the captured-candidate hash
   once capture completes. It must not commit a terminal validation-rejection
   outcome: that would make a later funded retry replay the old refusal.
   Preserve or reconstruct the durable unaccepted intent and exact staged
   bytes according to the admission phase, and check the keyed outcome after
   any uncertain final transaction before retry. A semantically invalid UUID
   or unresolved edge remains a source refusal. Other allocation failures,
   process termination and foreign allocations cannot be represented as a
   guaranteed typed pressure result
   without additional mechanisms; recovery must inspect the durable old/new
   selection state rather than assuming a callback ran.

This is a design for cooperative owned allocations, not a blanket claim that
Rust or serde allocation is fallible. The current F3 path uses infallible
`Vec::with_capacity`, `collect` and parser allocations, among others. Before
activating this policy, audit and precharge **the complete F3 preparation
path** and use checked/fallible growth where the selected guarantee says
pressure is reportable. If a dependency cannot meet that contract, narrow
the guarantee to the audited owner set and state the remaining failure mode
in the host profile. No generic process-survival or OS-RSS bound follows.

The host work account is distinct from T2's
[same-volume job account](T2-SAME-VOLUME-JOB-ACCOUNT-2026-11-03.md): that
account owns temporary filesystem claims, stage/copy/output roots and
cleanup. Neither turns the other into an APFS physical reserve. F3's
current project-bucket scan remains until T2 integration replaces its
coverage. A future external UUID sort would register its spill root under
T2, charge output and merge scratch, and prove ENOSPC, interrupted-run
cleanup, restart quarantine and no duplicate retention. It is not the
initial implementation and has no implied zero-cost fallback.

## Proof before selecting the profile

- Implement the production strict typed decoder and one shared F3/F4
  grammar. Exercise escaped and plain decoded-string equivalence, duplicate
  decoded keys, complete namespace collision, unselected-root closure and
  stable diagnostics under reordered files and chunk sizes. Replace split
  collision checks without changing existing source meaning; verify
  parse-before-collision-before-edge error precedence.
- Audit every allocation in the chosen F3 guarantee, including generic
  project/schema/history parsing, stage/capture buffers, summaries, merge
  state and overlapping operations. Inject reservation failure at each
  phase; prove no partially validated root is selected, the old root
  remains readable, and the same token/candidate can retry after the
  allowance increases. Test cancellation, concurrent work and recovery.
- Measure charged high-water, process RSS, end-to-end latency and allocator
  behavior separately on small, 16 MiB dense, escaped-ID, many-root, and
  all-fallback numeric fixtures. Include old/new pin overlap and F4/T2
  concurrent work in the selected host workload. Select the initial
  allowance from those measurements with explicit headroom and a named
  machine; expose it as a host setting/receipt, not source identity.
- Verify the semantic source bounds still hold independently. Prove a
  sufficiently funded host can complete the selected finite source profile
  or publish the narrower supported profile honestly. Never turn an
  operational pressure result into a source-invalid result merely to make
  a test pass.

No numeric allowance, production account, end-to-end admission or target
performance claim is selected by this note. The candidate must be reviewed
with F3, F4, T2 and diagnostic consumers before changing governing C4
contracts or activating `core.scene@3`.
