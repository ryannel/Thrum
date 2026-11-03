# Foundation specification: source, operations, world and transforms

1 November 2026, revised 2 November. **Selected baseline specification; the constant-light E1 subset has a bounded provisional staged-source proof, while its F4 product and durable E1 build admission remain unimplemented and unqualified.** Scope is D1 source/transaction, WP01 and the bounded first E1 source-binding extension. It refines [decisions](ARCHITECTURE-DECISIONS.md), [C01/C02/C10](CONTRACTS.md), [world/content](modules/WORLD-CONTENT.md) and [tools](modules/TOOLS-DELIVERY.md). N1 owns match/wire policy; this design keeps local authority/session identities independent. Exact API names below are protocol sketches, not Rust signatures promised to SDK users.

## 1. Source format and identity — select one ordinary file language

Use UTF-8 JSON for editable engine records: `project.json` (project ID, format version, entry scenes, target profile references, declared dynamic roots), `scenes/<name>.scene.json`, `prefabs/<name>.prefab.json`, and adjacent `<master>.asset.json` for imported assets/settings/ID mapping. Game Rust/WGSL and binary masters remain ordinary files in the same project tree. Human names/paths are mutable; each record has `id` (lowercase RFC-4122-form 128-bit UUID), `type` (stable ASCII schema type ID), `schema` (positive integer), and `fields` keyed by permanent ASCII field IDs such as `transform.translation`; display labels are registry metadata. Prefab node IDs are scoped stable 128-bit IDs; instance/node pairs follow W1. Field IDs never change meaning/units; a rename changes display text, a semantic change receives a new field ID and explicit migration. UUID collision, duplicate record/node/field IDs and required unresolved references reject admission. Optional unknown fields round-trip as JSON; required unknown fields reject.

Example source record (readable, paths are not identity):

```json
{"id":"2a5ef644-7225-4770-9f4d-e1d9c0a7d515","type":"core.scene-object","schema":1,"fields":{"transform.translation":[2,0,4],"transform.rotation":[0,0,0,1],"transform.scale":[1,1,1]}}
```

### First engine-owned environment records — selected next F3 extension

The bounded [E1 authored source](modules/ENVIRONMENT.md#selected-bounded-authored-source-and-light-handoff)
needs engine-owned numeric tracks, grids, structural predicates and typed
cross-record links. The original staged first profile accepted `game.*` text
and a restricted scene reference only. The bounded constant-light F3 slice now
validates its numeric source and typed light link provisionally; the
[proposed storm and fixture records](research/ENVIRONMENT-IMPLEMENTOR-DOGFOOD-2026-11-02.md#proposed-f3-source-records)
remain unadmitted.
Select a **fixed, versioned engine-owned schema table with direct bounded
parsers** for these first `thrum.environment.*` and `thrum.light.identity`
types. This extends the existing exact staged tree and its `project.records`
membership; it does not widen every game-owned source-carried descriptor to
arbitrary nested numeric schemas. Game descriptors retain their separate
source-carried meaning history. F3's one immutable built-in schema authority
records field/nested meaning, units, interpolation, support, reference target,
version and definition hash in the selected SDK/toolchain. A changed meaning
requires a new field/version and explicit source migration, not a changed
parser under the same identity. The bounded proof fixture additionally
registers `thrum.fixture.static_surface_receiver` and
`thrum.fixture.closed_shallow_receiver` as **fixture-only** built-ins; they
do not define a general public receiver model. For
`thrum.environment.authored`, unknown optional keys are permitted only
directly under `fields` with the literal prefix
`x.thrum.environment.`. Admit at most 16 such keys, each raw JSON value at
most 16 KiB, total raw optional values at most 256 KiB and at most 4096
nodes **across all opaque values**; each value has depth at most 16 and any
decoded string/key at most 4096 UTF-8 bytes, within F3's
whole-record bounds. Preserve each untouched value's exact bytes when editing
a neighboring known field; enclosing object whitespace/order may change.
Unknown source keys outside that prefix, and unknown required
profile/version/field meanings, refuse. The first light/fixture built-ins
are strict and reject unknown fields. Opaque values do not become runtime
controls.
`thrum.environment.authored` schema 1 is fixed to `constant-light-v1`.
The proposed `authored-vertical-rain-v1` profile uses schema 2 with its own
immutable definition hash; adding it must not alter schema 1's meaning.
An isolated [schema-2 grammar and closure proof](research/E1-AUTHORED-RAIN-F3-SCHEMA-2026-11-03.md)
pins candidate definition bytes and tests their bounded parser, but C09/F3/F4
have not adopted them. That storm profile has no ambient or background fields,
so it cannot by itself close the first R0 render-light requirement.
`core.scene` schema 2's `environment.source` UUID targets the stable authored
environment **type**, with the selected record's supported schema validated
separately. This lets a later schema-2 source bind without changing the
scene field's meaning. The current staged F3 slice validates schema 1 only;
schema 2 and its companion fixture records remain unsupported.
For the constant-light record, decoded f64 Sun direction must be finite,
nonzero and within `1e-9` of unit length; its nonnegative finite photometric
colour must satisfy `|0.2126R+0.7152G+0.0722B-1|<=1e-9`. F4 normalizes the
accepted vector and colour in its typed product. This tolerance is source
validation, not a GPU shading-error budget. Illuminance, ambient and
background channels must be finite and nonnegative; later F2 recipe and
device half-float envelopes can impose tighter upper bounds.
`constant-light-v1` declares exactly the unsupported quantity set `wind`,
`cloud_tau`, `precipitation`, `air`, `thermal`, `moon`, `air_quality`, with no
duplicate. A missing or contrary declaration refuses; it does not imply a
weather provider.

Use `core.scene` schema 2 with the permanent field ID
`environment.source` to select one stable
`thrum.environment.authored` source UUID for the first single-entry-scene
R0/E1 closure. Scene schema 1 remains valid for scenes without that selected
capability; an R0 scene requiring light cannot silently invent a source. A
source record appearing in `project.records` without the entry-scene binding
is declared content, not the active environment. Wider multi-entry and local
override composition must check nonoverlapping quantity authority explicitly.
The field has typed required resolution when present. W1 light identity
records carry one incoming E1 driver binding and no rival transform/intensity
descriptor. The first R0/E1 profile refuses multiple `project.entryScenes`.
F3 dispatches declared `records/*.record.json` by checked top-level
type/version to either this fixed built-in table or the existing `game.*`
descriptor path; it does not assume every non-scene record is game-owned.

F3 validates the complete declared UUID namespace, including embedded patch,
cell, domain and bed identities, before F4 cook. It checks required reference
type, reciprocal source/exposure linkage and one selected Sun driver; the
storm source→exposure→source record cycle is allowed only with those checked
relations. The *F4 product dependency graph* is acyclic: typed source/grid
and geometry are inputs to exposure, then consumers bind the resulting
products. F4 requires an authenticated, pinned admitted-source lease anchored
in the selected committed root, with verified manifest and hash-checked
immutable record reads; a provisional stage alone cannot authorize a ready
build. F4 records a build binding from admitted SourceRevision and stable IDs
to exact typed products. A domain-separated cook key covers canonical
schema-validated relevant values, dependency products, pinned built-in
definition hashes and schema/cook/evaluator/target cache versions; an immutable
product manifest binds that key to the hash of actual product bytes. The build
binding separately retains the exact admitted SourceRevision. An unrelated source
edit may reuse product bytes under a new binding, while a geometry/driver edit
invalidates affected products. The implementation must prove old-stage
preservation on rejection, exact opaque neighboring edits, bounded peak work,
and source/save compatibility before this extension is called admitted.
The [first constant-light product](modules/ENVIRONMENT.md#selected-first-f4-constant-light-product-and-binding)
selects the initial binary payload, normalization, cook key and binding;
the admitted-source lease and F4 builder remain unimplemented.

Parsing is strict I-JSON: no duplicate keys, NaN/Infinity, invalid Unicode, out-of-range integer IDs or implicit units. JSON numbers are finite f64 and negative zero normalizes to zero on semantic write. SourceRevision hashes **exact file bytes**, so formatting changes yield a new source revision; recipes may separately hash schema-validated canonical values using [RFC 8785 JCS](https://www.rfc-editor.org/rfc/rfc8785.html). Registry and root JSON are JCS encoded for their own hashes. Records may use comments only in a separate documentation field, not JSON comments. Version 1 has no automatic major-version coercion; an admitted migration creates a new file tree and records old/new revision and migration ID. Client writes use stable field IDs and schema version, never array positions or human labels.

`SourceRevision = SHA-256(JCS(TreeManifest))`. The manifest contains format version, project ID, sorted normalized relative paths, entry kind (`file` only), byte length, SHA-256, stable record ID/type where parseable, and declared dependency roots. Source paths use `/`, Unicode NFC, no `..`, absolute paths, symlinks, devices or case-fold collisions, so the same project has one cross-target membership interpretation. Exclude `.git`, `.engine` stores, build caches and known generated outputs by versioned project policy; no wildcard that silently drops an undeclared master. A referenced Git LFS pointer is insufficient: resolve and hash the master payload or mark unavailable. Membership, deleted paths and renames are part of the manifest. The selected v1 **supported source profile** admits at most 100,000 tree entries; JSON record bytes ≤16 MiB, JSON value nodes ≤1,000,000 per record (including containers), nesting ≤64, operation payload ≤16 MiB, and a single immutable blob ≤16 GiB. The first native accepted-store adapter separately bounds the encoded canonical manifest to 64 MiB; 100,000 entries do not imply every path/metadata length fits that byte bound. The first semantic profile also independently bounds declared records, schema descriptors and aggregate required edges; their individual maxima need not coexist in one candidate. Publish these dimensions together in the resolved profile and diagnose the one actually exceeded. The node bound limits adversarial expansion of a parsed record and is checked before each value is built; parser heap peak still needs a measured budget before semantic admission is active. These are finite capacity promises, not performance results or intrinsic limits of the exact-byte source identity. A larger safely supported capacity with unchanged bytes and meaning requires a qualified profile. A bound written into a hashed built-in definition, including the current environment record and opaque limits, also requires a new definition hash and dependent cook key when widened; an external operational allowance does not. Changed encoding or field meaning requires the relevant format/schema revision and migration. Build and cook never read the mutable checkout.

`RecoveryRoot` is JCS JSON with `format:1`, authority ID/kind (`authoring`, `private-play`, `standalone-save`, `server-save`), monotonically increasing `generation`, admitted SourceRevision (authoring only), `journalHead`, dedup epoch/index head, and a nullable checkpoint tuple `(tick, baseSourceRevision, BuildRevision, manifestHash, overlayHash, schema/provider versions)`. The tuple is copied unchanged when admitted source advances; never pair old overlay with newer source. A save root always has a non-null tuple. Root hash and referenced closure are verified before use. Working files are the creator's editable source; this root identifies an exact admitted snapshot for builds/recovery, not a second editable property authority. MatchId, local SessionId/WorldEpoch, wire ID, W1 slot and source ID have distinct type tags; none is reconstructed from another. N1's initial u32 action/type numbers are **indices in a pinned protocol manifest** mapping these F3 stable type/field/action IDs to wire numbers; neither Rust `TypeId` nor source path is a wire identity. MatchId is a fresh 128-bit value, not a SourceRevision.

### Imported object identity

An asset package has one stable source ID and a sidecar map from source object IDs to engine node/material/skeleton
IDs. Prefer exporter-preserved UUIDs (for glTF, an agreed `extras.engine_id`); duplicate IDs reject. If a format/exporter
has no persistent IDs, first import assigns sidecar IDs and marks them `path-bound`, keyed by the unique semantic
parent/name/kind path, never a raw array index. Exact unchanged unique paths/kinds reuse those IDs on reimport;
renames, ambiguity, removals or duplicate names require an explicit mapping/removal decision before activation. There
is no guessed nearest-mesh/skeleton reassignment. The preview shows added/removed/remapped objects and dependent
placements/overrides; confirmation is an ordinary typed operation, which an authorized agent may perform. Retained
node/material IDs preserve game-owned placement/overrides while rebuilt mesh payloads receive new content hashes.
Exporter recipes should preserve UUIDs to avoid repeated remapping. This is the selected identity policy, not a claim
that arbitrary external format edits always carry enough information for automatic matching.

## 2. File application and source admission

An authoring mutation is `{kind, schemaVersion, targetStableId, expectedSourceRevision, readSet[(id,fieldId,revision)], writeSet, parameters, actor, correlationId, token:(authorityId,epoch,key)}`. A batch produces one reviewable file patch and semantic diff. It is a **proposal**, not an accepted edit. T1 derives transform inputs, including preserve-world matrices, from the expected admitted authored revision, never a dirty file or play pose. It validates schemas/units, stable IDs, hierarchy/cycles, TRS and affected consumers on the proposed tree. To apply, the selected working files must still match the declared base and read/write preconditions; unrelated dirty files must be declared in the admission set or left untouched. A stale or ambiguous basis conflicts and requires review, not an implicit rebase. Apply-back uses a three-way `(play base, selected play fields, current admitted source)` proposal with the same file/application boundary; no remote replica/server path.

The initial native flow is: (1) identify admitted base/root generation and an explicit complete working-set declaration, including additions, deletions, renames, code, metadata and binary masters; (2) coordinate a file-writer window among cooperating tools, record branch/base/path hashes and a durable **unaccepted** application intent containing the reviewed patch and recovery copies of affected prewrite bytes not already in the admitted store, then check the selected physical-space plan; (3) apply the patch to ordinary files, using staged replacements and rechecking each expected path, or capture an author-declared completed external change set; (4) capture the declared full tree into immutable staging, reread membership/hashes and validate exact bytes, schema and dependency closure; (5) recheck available space and durably stage missing SQLite blobs/manifest, then in **one metadata transaction** compare root generation, commit admitted root plus outcome/dedup record and convert staging pins; (6) acknowledge `accepted` only after that transaction's declared durability condition. This stages only affected old files before application, not an unnecessary second whole project master copy. The application intent is **not** an accepted source root or permission to replay writes blindly. A losing CAS or drift leaves files and candidate visible as a conflict; it never silently rebases. `accepted` means this exact captured revision is recoverable and available to cook/build; it does not mean working files will stay unchanged, a build succeeded, or a run changed. F4 cooks only from admitted bytes. The first game restarts a disposable run for substantive source changes; qualified visual preview alone may use the existing coherent publication barrier. Undo/redo produce forward reviewable file changes, then use this same admission path.

Idempotency applies at **admission**, after file application/capture. Within an authority's live operation epoch, `(epoch,key)` plus canonical admission request/captured-tree hash has one stored outcome; same key/different payload rejects. Accepted outcome and source-root advance commit together. A validation rejection commits its outcome and metadata generation with the admitted SourceRevision unchanged. A crash before admission may leave a mixed working tree and an unresolved application intent; retry returns `needs-reconciliation` with observed bytes, not an invented accepted or rejected answer. After admission, a lost response may query/retry and recover the exact outcome. An epoch accepts at most 10,000 requests or seven days of new requests; then a new epoch opens. At most 1,024 admissions/cooks may be pending per project; excess returns `busy` before file application. Pending cook/preview reaching seven days becomes terminal `failed` without undoing admitted source. Keep closed-epoch outcome/dedup data at least 30 days and until terminal/recoverable, then record retirement; old tokens return `expired`, requiring a fresh root read. Keep inverse field deltas for the last 1,000 operations or 30 days, also bounded by the undo-only byte cap below; report expiry explicitly. Stronger save/release/source pins remain independent. Correlation ID groups diagnostics, not deduplication. Signing/upload/payment are separate non-undoable actions.

External admission has `(baseRootGeneration, selectedGitTreeOrCapturedManifest, intentDeclaration)`. A watcher only marks `working-changed`. An immutable Git tree is a strong capture boundary after resolving LFS payloads, but Git HEAD alone omits dirty files. For a dirty checkout, the initiator declares the complete finished change set; capture copies its full closure and rereads membership/hashes to detect drift. This cannot prove a single instant against noncooperating writers, including ABA; record the declaration and leave uncertain capture pending. A branch switch or competing admission changes the expected root or branch and produces conflict; no branch files are overwritten by recovery. A clean source handoff exports the exact admitted file tree byte-for-byte into a separate bundle, or identifies a verified immutable VCS tree with resolved LFS payloads whose manifest hashes to the admitted SourceRevision. The bundle includes real game code, required masters, import/recipe metadata and pinned toolchain/dependency declarations; it excludes the internal SQLite store and rebuildable caches. T2 rehashes the recipient bundle against the admitted TreeManifest before release. Export never overwrites current working files. Current dirty files or an internal database alone are insufficient when they differ from the admitted snapshot.

## 3. Native admission and browser runtime persistence

**Native desktop/build host.** Use **one SQLite database** for admitted exact source bytes, manifests, roots, outcomes, pins and GC state. It is an internal immutable snapshot/recovery store, not the creator's edit destination or a mutable property database. Keep the selected file tree in ordinary project files. At database creation set and verify `page_size=4096` and `auto_vacuum=INCREMENTAL`; other existing modes need an explicit migration. Store blobs as 4 MiB `blob_chunks(hash,index,bytes)` with a `blob_header(hash,length,chunkCount,state)`; prewrite in bounded transactions, reread/verify full SHA-256 and length, then mark `complete`. Use `journal_mode=WAL`, `synchronous=FULL` on a qualified local SQLite VFS/filesystem. On qualified macOS VFS, set/verify `fullfsync=ON` and `checkpoint_fullfsync=ON` before claiming the strongest local power-loss condition. Report a weaker guarantee if the VFS/device cannot support these claims. The **single final SQLite transaction** checks root generation and referenced complete/non-deleting blobs, then commits root/outcome/dedup/pins together. Successful commit is native `accepted`. This avoids a cross-file blob/root flush race, subject to SQLite's VFS and honest-hardware assumptions. Failed reserve, blob write or verification, and a confirmed failed final transaction, leave the old root selected; working-file changes stay visible but unadmitted, and orphan chunks are reclaimed later. An uncertain final commit is resolved by the exact keyed outcome before reporting selection. Do not split roots/outcomes/chunks across attached WAL databases. A separate content cache is never accepted-source authority. [SQLite WAL/synchronous](https://www.sqlite.org/pragma.html), [atomic commit](https://www.sqlite.org/atomiccommit.html), [WAL database limits](https://sqlite.org/tempfiles.html).

**Browser runtime.** Initial browser delivery consumes a packaged admitted build; it has no full browser source editor or browser authoring root. Cache API/IndexedDB may cache immutable runtime products, but cache eviction never changes source identity. Player saves use a separate versioned IndexedDB database with `payloads`, `roots`, `pins` and `gc` stores. Prewrite immutable checkpoint payloads in bounded transactions, await completion, reread/check hashes, and reserve required old/new closure and quota. A final single `readwrite` transaction spanning those stores checks expected root generation and complete/non-deleting payloads, then writes new save root, outcome and pins together; report save completion only after its `durability:"strict"` transaction completes. GC marks from roots/pins and rechecks absence of references in a transaction sharing that scope before deleting payloads. `strict` is a hint, not universal power-loss proof; close/quota can abort. Request/report `navigator.storage.persist()` where available, label best-effort storage honestly, and offer explicit export/backup for a durable handoff. A completed transaction cannot promise survival of user deletion, profile loss or a device that violates storage guarantees. Browser authoring needs a separate file, admission and durability adapter before being advertised. [IndexedDB 3.0](https://www.w3.org/TR/IndexedDB/), [Storage Standard](https://storage.spec.whatwg.org/).

The adapter reports `commitGuarantee`, root generation and verified closure. A player checkpoint has its own authority root and exact tuple. Running world may exceed the last checkpoint; a crash restores exactly that tuple, while newer admitted authoring source remains available for a fresh run. Periodic checkpoints bound gameplay loss; final suspend callbacks are insufficient. A private play save never writes a released/server authority root; a server pins its BuildRevision/gameplay manifest until explicit restart or migration/rehandshake.

## 4. Working files, crash recovery and GC

Ordinary working files are the edit surface. In a shared checkout, T1/AI returns a reviewable patch and does not write until cooperating participants enter a coordinated writer window with recorded branch, base manifest, affected paths and lease. One writer applies the reviewed set; explicit admission then captures and verifies the complete snapshot. A filesystem cannot atomically replace an arbitrary multi-file tree or exclude a noncooperating writer by advisory lock. Per-path hash/rename plus before/after closure checks detect ordinary drift, but not all ABA races. If a noncooperating writer may touch the set, keep the patch proposed or require an author-declared completed dirty snapshot; report uncertainty rather than claiming a single instant. A crash midway through application leaves visible files and a durable **unaccepted** intent. Recovery compares branch/bytes with the intent, shows partial application and preserves both; it neither replays nor rolls back unknown paths. Repair/apply again, then admit. A crash after root commit restores admitted exact bytes and outcome, then separately reports any later working-file difference. Git diff exposes changes as soon as files are applied, before acceptance; Git itself does not make application, root commit or multi-file admission atomic.

GC uses two publication epochs. Every durable admitted root/checkpoint/release, retained private save, live build/session, reader/serializer lease, unresolved application intent and staged admission pins its transitive internal blob/product closure. A pin is registered in a short metadata transaction **before** a reader or publication can depend on bytes; release follows CPU readers/GPU fences and backup/export duties. Collector first marks from a consistent root/pin snapshot and records a deletion generation; in a later exclusive store transaction it rechecks unreferenced status, marks `deleting`, then removes SQLite chunks in bounded transactions. A publisher treats `deleting` as unavailable and restages before root CAS. Crash cannot make a published root lose newly needed bytes. Plan prior root, staged new bytes, journal/outcome, supported save/release closure and headroom counting shared hashes once; refuse an estimate that exceeds the selected plan. A precommit failure leaves the prior root selected, while an uncertain commit is resolved by its exact keyed outcome and later maintenance failure cannot undo a committed root. An initial bounded native authoring profile may keep all verified blobs, manifests and outcomes append-only with collection disabled; it reports space pressure and refuses new admission without losing prior roots. Collector activation requires the complete pin/closure/deletion protocol above, not merely a mark-only proof. Browser runtime-cache collection is separate from admitted source and player-save roots.

**Native physical-space planning and admission.** The first native adapter defaults to configurable planning targets of 52 GiB for SQLite main, 2 GiB for WAL, 2 GiB for managed temporary/cache and 8 GiB of observed volume headroom (64 GiB in total). These are early-admission and operations targets, **not hard file-size caps or a guarantee that SQLite or another process cannot consume more space**. An operator may raise or lower them after inspecting the actual volume and retained closure; the source format's independent file/record bounds still apply. Do not fix `max_page_count` at the 52 GiB planning target or require a custom VFS solely to enforce these defaults. Before file application or capture, estimate the old/new retained closure, working-file staging, unique missing blobs, expected main/WAL growth and managed temporary use, counting shared hashes once. Compare the estimate with the selected plan and actual available volume space; refuse early with `insufficient-space` if it does not fit. Recheck live free space and observed main/WAL lengths before each bounded write and checkpoint. These checks are conservative planning and backpressure signals, not certified upper bounds on SQLite page rewrites, sparse-file allocation or external writers. Expose estimated versus observed peak and any overrun separately.

Use ≤4 MiB blob chunks and short transactions. With the default plan, begin `PASSIVE` checkpoints near 1 GiB WAL and defer new staging around the observed 2 GiB stop until a `TRUNCATE` checkpoint succeeds after readers drain; those watermarks are configurable and one in-flight transaction may cross them. A blocked checkpoint remains visible and cannot count as reclaimed. `journal_size_limit` is not a prewrite cap. No SQL read transaction spans CPU decode, GPU upload/fence, editor review or network lease: register a logical pin in a short transaction, close it, then use bounded chunk reads. A cooperating cross-process writer lock coordinates Thrum-owned admission and checkpoint work; its free-space checks are planning guards, not proof that an APFS operation cannot overshoot the observed headroom target. A real `SQLITE_FULL`, `ENOSPC`, I/O error, failed checkpoint or later free-space shortage leaves the prior admitted root selected unless the final root/outcome transaction committed successfully; reopen verifies that pair together, incomplete blobs stay unselectable, and working changes remain visible for retry/reconciliation. Diagnostics state the qualified filesystem/VFS assumptions and any observed overrun. Sparse file length and allocation hints are not physical reservations. Bounded small-file batching also belongs to the first accepted-source path: a separate transaction sequence per tiny file would make the 4,096-file provisional profile require 12,292 commits including manifest and root. Measure WAL, edit latency and peak memory with the selected batch policy before promotion. [SQLite WAL](https://www.sqlite.org/wal.html), [checkpoint pragma](https://www.sqlite.org/pragma.html#pragma_wal_checkpoint).
After GC deletes chunks, `freelist_count × page_size` is reusable **inside** SQLite, not bytes returned to the volume. Outside the simulation tick run `incremental_vacuum(256)` per slice, checkpoint `PASSIVE`, then use `wal_checkpoint(TRUNCATE)` when readers drain and inspect its result. Continue bounded slices; report logical unpinned bytes, reusable freelist bytes, main/WAL/temporary physical lengths and observed physical bytes reclaimed separately. Never advertise GC completion as returned disk space. Full `VACUUM` is not normal maintenance because it can need up to twice the database size free. [SQLite auto-vacuum](https://www.sqlite.org/pragma.html), [VACUUM requirements](https://www.sqlite.org/lang_vacuum.html).

### Native admitted-source lease for the first F4 cook

Production F4 receives a narrow, owned read capability, never a bare digest,
SQLite connection, checkout path or provisional `StagedCandidate`. A store
operation selects the current committed authoring root under an expected
authority/project/generation/revision, then registers a durable read pin in
the **same short transaction**. The returned lease identifies that exact
source revision and commit guarantee, exposes its verified canonical manifest
and supports lookup by declared stable record ID. One build request shares
that manifest/index and pin among its products; it does not rescan the full
source tree for each light or asset. A record read verifies the
manifest entry, complete/non-deleting blob state, exact length, chunk layout,
SHA-256 and selected source envelope before exposing immutable bytes. The
first JSON record read buffers at most the 16 MiB record bound; larger binary
masters need a separately qualified bounded streaming completion barrier.
Manifest acquisition checks identity, membership and every direct file's
complete matching storage metadata; claiming a fresh full-byte sweep would
require actually rereading all source bytes. External dependency roots remain
unsupported in the first profile.

The lease remains bound to revision **A** after authoring selects **B**.
Readers and clones share one pin; GC/retirement cannot remove A's closure
until its final reader releases it. No SQL read transaction spans parsing,
cooking or consumer work. Pin release failure retains durable protection and
is reported/retried; age alone never proves a live reader abandoned its pin.
The first single-owner native profile may attach an ephemeral reader-pin owner
nonce to the store's exclusive process lock. A new cooperating owner can retire
that dead owner's ephemeral reader pins only after acquiring the lock and
rechecking root/owner state in a short transaction; build, save, release and
unresolved application-intent pins have different durable lifetimes. A new
process may not infer death from a PID or timeout alone.
Cook completion publishes only if its requested selection/build still matches
the intended target, while a valid old cook may remain cache content. The
smallest initial production store may disable destructive collection and
conservatively retain complete closures under a finite planning budget;
enabling deletion requires the C10 two-epoch recheck over roots, leases,
staging and build/save owners. The `sqlite_store` path is production compiled
and exposes the accepted-root/pin candidate used by the F4 headless fixture.
The separate format-2 `ProvisionalSourceStore` is compiled only for in-crate
recovery and checkpoint proof fixtures; its publication does not grant
accepted-source authority. The accepted store has an owning read lease, an
exclusive cooperating process owner from before SQLite open through the last
lease, final-COMMIT quarantine and an indexed selected-outcome head audit.
Admission now holds one permit across the attempt while releasing the SQLite
core between committed blob chunks and bounded tiny-file batches, allowing
already-pinned reads to interleave. Pin registration/release and the final
root/outcome transaction remain serialized; stored-blob verification and
manifest staging now copy or commit bounded chunks under the core and hash
outside it. Final direct-file closure/publication can still hold the core for
a longer transaction.
The admitted lease still needs canonical RecoveryRoot/application recovery,
truthful durability diagnosis, selected planning and space-fault recovery,
complete pin-owner lifecycle and safe GC or conservative retention under its
observed budget. The [owner reconciliation study](research/SOURCE-OWNER-RECOVERY-RECONCILIATION-2026-11-03.md)
tracks accepted-source interruption and format-2 intent semantics still to
reconcile. The current lock protects cooperating openers on the qualified
Unix path; Windows path identity, noncooperating SQLite openers and host
power-loss behavior remain unqualified.

### Large-source and undo retention cost

The initial SQLite store contains an additional immutable copy of admitted source bytes; it is not a zero-copy view
of Git LFS or working files. This cost buys an exact durable build/recovery pin while ordinary files continue changing. Unchanged hashes are shared across revisions, but a changed binary master can
require another full blob. Admission estimates that copy, working-file staging and WAL/maintenance peaks
against actual free space before staging, then observes growth during writes. The default 64 GiB native plan is
configurable guidance, not preallocation, a hard cap or permission to fill the disk. Browser storage is separately capped by its reported quota and selected workload, never this native cap.

Transient generations stay outside admitted source until selected. Undo has a further 1 GiB cap on unique bytes pinned
only for undo, in addition to its operation-count/time window. It expires oldest undo-only roots when that cap requires
it and reports the remaining horizon; a single larger asset replacement is explicitly not guaranteed undo retention.
It never evicts current source, a retained previous publication during commit, supported saves/releases, active readers
or required selected masters to make that cap. Those stronger roots must be migrated/retired under their own contract.
A typed operation exposes its undo/retention class before execution and reports actual retained revision afterward;
no acceptance response promises that every binary iteration will remain available for 30 days. Compact operation
outcomes retain IDs/hashes after their undo payload expires, avoiding loss of idempotency evidence.

## 5. WP01 numerical transform admission

Select initial **authoring envelope** for the reference workloads: world coordinates within ±`10^6` m on each axis; each object's authored local geometry/subtree radius ≤`10^3` m, and every transformed bound must remain inside that world extent; finite nonzero local scale magnitudes in `[10^-4,10^4]` for preserve-world operations; new-parent linear condition number ≤`10^3`. Static render-only zero-scale records may exist but cannot be preserve-world operands or physical/normal-transform inputs. These are v1 schema admission limits, not an astronomical-world promise, and can change only by a versioned profile with a new error proof. With a view origin ≤512 m from the visible position, f32 conversion has ulp scale `2^-23·512 ≈ 0.061 mm`; the P3 selected local physics radius ≤2048 m including collider/query support gives `≈0.244 mm` and has its own contact/query qualification. Choose a 1 mm initial game placement tolerance and a much tighter **authored preserve-world round-trip budget of 1 µm**. At ±`10^6` m, f64 roundoff is order `10^-10` m; the `10^3` condition cap can amplify this toward `10^-7` m before the explicit residual test, leaving analytical margin below 1 µm. These are rounding-scale estimates, not measured solver or GPU guarantees. A game needing smaller/larger spatial regime selects a separately specified envelope before those operations are admitted.

For `preserveWorld`, evaluate old object and candidate new-parent **authored** world matrices from the operation's expected admitted SourceRevision, not unadmitted working files or runtime/play pose. Reject cycle, dynamic-body authority violation, nonfinite values, singular/condition limit and unsupported transformed descendant/consumer regimes before file application and source admission. Compute `L*=inverse(WnewParent)·Wold` in f64. Compute the 3×3 singular values of the parent linear map by fixed-order Jacobi sweeps on `AᵀA` (plane order XY, XZ, YZ, at most 32 sweeps; off-diagonal norm must fall below `10^-14` times diagonal norm); reject nonconvergence or condition above `10^3`. Let `L*`'s linear columns be `c_i` with lengths `s_i>0`; if determinant is negative, assign the one negative scale to the largest `s_i` (tie X then Y then Z), otherwise all scales positive. Form sign-adjusted unit-column matrix `N`. Compute its proper polar factor by `X_0=N`, `X_(k+1)=0.5·(X_k+inverse(X_k)ᵀ)` for at most 32 iterations; each 3×3 inverse uses fixed-order partial pivoting (largest absolute pivot, row-index tie), rejects a nonfinite/zero pivot, and stops when `||XᵀX−I||_F≤10^-12`. Reject nonconvergence. Set `R=X`, convert to normalized quaternion, canonicalize sign (`w>0`, or if `w=0`, first nonzero x/y/z positive), and recompute signed scales by column projection `dot(R_i,c_i)`. Recompose `Lhat=T·R(q)·S`; all threshold comparisons use unrounded f64 values, with diagnostics rounded only for display.

Reject unless `||linear(L*-Lhat)||_F / max(1,||linear(L*)||_F) ≤ 10^-10` **and** `||Δt_world||_2 + r·||ΔA_world||_2 ≤ 10^-6 m`, where `Δ = WnewParent·Lhat−Wold`, `r` is the declared object/subtree local bound (≤`10^3` m), and the matrix norm is spectral (a conservative Frobenius bound may be used). This bounds displacement of every supported point, does not loosen with distant world position, and rejects shear rather than projecting it visually. Use a proper quaternion and signed parity for reflected static visuals; render normal/tangent/winding, physics shape and skeletal consumers each declare support, bake or reject. `preserveLocal` keeps exact stored TRS and explicitly changes world pose. A 45° rotated object under a `(2,1)` new parent requires nonorthogonal local columns and fails the residual. The rejection reports `base SourceRevision`, object/parent IDs, condition, linear/spatial residuals, thresholds and `preserveLocal` alternative. Admitted SourceRevision, working files, active W1 generation and product manifest remain unchanged; the metadata root generation may advance solely to durably record the rejected outcome.

## 6. W1 typed access and structural publication

W1's public runtime handle is `(WorldEpoch:128-bit UUID,slot:u32,generation:u64)`; a fresh WorldEpoch is generated
for each constructed world, slots never wrap, and a slot is permanently retired before generation wrap. Authored
hierarchy depth is capped at 128 and cycles reject; its private side table maps to current `hecs Entity`. That backend token may persist **inside W1's identity table** but never leaves W1; query borrows are stage-bounded. F3 schema IDs/field codecs do not use Rust `TypeId` or hecs serialization. F1 registers systems as `(phase, read component/resource IDs, write IDs, explicit before/after, authority role)` and rejects conflicting unordered writes, unknown schema/phase and forbidden cross-role mutation. A stage-scoped `WorldRead/WorldWrite` resolves public handles and exposes typed views only for declared access; borrows cannot cross a structural barrier. Order-sensitive queries sort stable IDs before reduction. Structural commands are `spawn/despawn/add/remove/reparent` with expected world/content/region epochs, stable IDs, dependency and product leases, plus a deterministic order key. No system receives raw `hecs World` or `Entity`; disallow `spawn_at`, `clear` and backend freelist restoration.

**Initial game loading and edit policy.** Construct a new W1 world from a target-ready pinned build and preloaded finite authoritative gameplay closure. Substantive source, gameplay, physics, navigation or provider changes take effect through stop/build/load/restart into a fresh world; they do not require every provider to transfer full live state. A target may choose an explicit stopped loading transition for a new region, with no tick or observer reading partial state. Future live streaming/editing remains subject to C05's coherent pre-tick publication, active dependent closure, leases and budgets, but needs a separately designed scoped transfer/residency protocol before it is advertised. Qualified presentation-only fields may preview live when their product/version and view-history rules are explicit. Ordinary per-tick gameplay commands mutate the exclusive world under F1's tick barrier after validation; readers see only committed ticks. Engine-managed events/effects buffer until commit. Unexpected failure after mutation is a terminal world-generation fault: publish no X[n+1] or effects, stop and recover from a valid checkpoint; no generic rollback or reversal of arbitrary native external effects is claimed.

**Initial low-peak stop/load transition.** Before retiring the old world, verify the new pinned build/content and required restore closure on disk, prepare only bounded metadata, and reserve load scratch plus persistent caches, retained snapshots/leases, save data and transition metadata under the target's peak budget. A player level transition first commits a compatible P7 checkpoint of the old authority state; if capture, storage or save-root publication fails, abort while the old world is still live. Then stop ticks/input publication, cancel or invalidate stale async work, quiesce snapshot/serializer readers and GPU submissions, release their leases/fences, and retire the old W1 and provider storage. Construct the new bounded world from the pinned target and checkpoint/transition data, validate every required provider and product, and publish its first complete snapshot only after all required state is ready. Old and new world storage do not coexist by default; persistent caches and transition metadata still count toward peak. If construction fails after retirement, remain visibly loading/error and reload the old pinned build/checkpoint through the same complete construction path; never claim the retired world remained live. An authoring play restart may discard unretained private test state and need not checkpoint it. This path requires no universal live provider-resume codec; a future seamless transition needs a separately admitted transfer and overlap budget.

**Initial active N1 server match:** preload and validate its entire finite authoritative gameplay region/collision/nav/query closure before accepting peers. No authoritative source/content/region structural activation or full-world pause is allowed during that match; network interest changes visibility only and cannot unload required solver support. Source edit/recook produces a future build, never mutates the live match. A request to enter unsupported/unloaded gameplay space rejects against the match's predeclared bounds; expanding it requires a new match or a separately specified non-stalling staged server transaction protocol. Client replicas may locally pause their own synchronization/view during a content rebuild while the server continues. This restriction fits the four-player/128-replicated-entity initial profile and avoids contradicting N1's slow-client/server-tick rule.

## Normal/adverse closure traces

| Trace | Required result |
| --- | --- |
| T1 reparent of a static prop on admitted R | F3/W1 calculate authored matrices from R and propose a patch. Coordinated writer applies it to ordinary files; full captured closure and TRS/consumer checks pass; SQLite root/outcome admits R+1. A new build/run pins R+1. |
| Shear reparent or dynamic pose used as source | Validation rejects proposal before file application. Admitted R, working files and old run remain unchanged; rejected token/outcome can advance metadata generation only. |
| Branch switch or noncooperating write during apply/capture | Stop with proposal/observed bytes and conflict. A dirty set is admitted only under an explicit completed-set declaration; watcher never activates intermediate files. |
| Crash mid multi-file application | Prior admitted root remains. Recovery shows partial working files and unaccepted intent; it does not overwrite unknown bytes. Author repairs/applies and admits again. |
| Crash after admitted root, before response or while files later change | Query token returns R+1/outcome; build uses immutable R+1. Working files show their actual current diff and may be ahead/dirty. |
| Crash between chunk flush and SQLite root commit | Old root remains selected; unaccepted working changes remain visible; orphan bytes are GC candidates after pin/grace checks. |
| R+1 admitted, cook fails, prior checkpoint uses R | R+1 remains admitted but not runnable; last good build/run and exact R checkpoint tuple remain. Never combine R overlay with R+1. |
| Player stop/load fails before or after old-world retirement | Failed compatible P7 checkpoint aborts with old world live. After retirement, failed construction stays visibly loading/error and reconstructs from the old pinned build/checkpoint; it never exposes a partial new or purported still-live old world. |
| Private play save and live server match | Each save root and build pin has its own authority/source closure. Replica cannot publish or apply back to server source. |
| GC races admission | `deleting` blob cannot satisfy root CAS; publisher restages/pins. Published closure remains retained until root replacement and reader/retention release. |

**Design closure:** source syntax, working-file proposal/application/admission, SQLite root/outcome durability, recovery, GC exclusion and TRS admission are selected here. First game loading uses restart or explicit stopped load; live streaming needs its own scoped design. **Unrun qualification:** forced SQLite/process failure, disk-full and concurrent writer/branch races, source handoff/LFS restore, transform fixtures, browser save persistence, and artist preview. No runtime implementation, target qualification or artist acceptance is claimed.
