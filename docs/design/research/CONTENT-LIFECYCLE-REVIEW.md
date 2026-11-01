# Content lifecycle review

31 October 2026. Bounded design review of [shared contracts](../CONTRACTS.md), [world/content](../modules/WORLD-CONTENT.md), [tools/delivery](../modules/TOOLS-DELIVERY.md), [platform/runtime](../modules/PLATFORM-RUNTIME.md), and [play systems](../modules/PLAY-SYSTEMS.md), as read on this date. This is a leaf review, not a replacement design or implementation verdict. The reviewer had prior exposure to the repository's engine brief and skills, but received no author conclusions or expected verdict. No code, build, device, or running scene was inspected. The source inspection below tests consequences of the proposed operating model; no source establishes that this custom assembly has passed.

## Capability derivation before comparison

From the raw commission alone: one custom-host engine serves contrasting 3D games on Mac/PC, browser, iOS and Android. Humans and agents must edit the *same durable scene source*. Cooked assets and resident representations are derived from it. Async work, streaming, memory pressure, interruptions and schema evolution are normal operating conditions. Pure 2D and MMO scale are excluded; no exact scene size, networking model or device floor is supplied. The following are necessary implications, not additional product promises.

| ID | Necessary observable property | Consequence for ownership and timing |
| --- | --- | --- |
| L1 | A scene edit by either client survives interruption and can be queried with the same meaning. | One durable authoring authority; accepted, cooked, active and saved are distinct states. Queries identify which state they show. |
| L2 | A save reloads the same game facts after streaming, recook and supported schema/content changes. | Stable object identity is independent of paths, runtime slots and product hashes. A save names its content ancestry and has a defined migration/retention path. |
| L3 | Physics, navigation, gameplay, render and audio do not observe incompatible generations of one edited region. | Required products and authoritative state publish at one tick boundary; presentation may follow only when its compatibility rule allows it. Shared assets require an affected-instance publication scope. |
| L4 | Cancellation, undo, deletion and profile-off cannot revive stale work. | Async results validate target, source/content, region, host and device generations as relevant; accepted operations have explicit durable terminal states. Undo is a new edit with concurrency checks. |
| L5 | A process can crash at any boundary without leaving a save pointing at unpublished or unavailable content. | Durable publication has one recoverable commit decision, or an equivalent journal protocol, linking scene/content and save ancestry. Temporary products remain non-authoritative. |
| L6 | Browser/mobile memory and storage pressure cannot silently corrupt authority. | Admission covers old/new overlap, decode and upload staging, pinned stream cells, checkpoint/journal/source storage and the recovery reserve. Failure is visible and preserves the previous valid generation. |
| L7 | Feature reductions keep gameplay facts unchanged unless the game author explicitly changes the contract. | Profiles identify mandatory versus optional products, permitted substitutes, and retirement/rollback costs. |

These requirements do **not** imply network collaboration, live arbitrary provider toggles, guaranteed recovery after user deletion of all local storage, cross-platform bit-identical physics, or an engine-wide universal transaction service. They do require a concrete publication/recovery protocol before dependent implementations can be called build-ready.

## Comparison and findings

The family designs establish a strong baseline: C01 and world/content distinguish stable IDs, runtime generations and hashes; C02/C05 place mandatory activation before the simulation tick; world/content specifies tombstones, pinned support, cancellation checks and leases; tools/delivery uses the same typed authoring path for UI/CLI/agents with optimistic revisions and forward undo; platform/runtime bounds queues and separates surface/device/world lifetimes. The remaining findings concern the complete lifecycle between those contracts.

### F1 — No single recoverable publication decision for active content, edits and saves

**Severity:** high; blocks integrated high-level lifecycle design. **Affected:** C05/C10/C12; world/content §§49–65; tools/delivery §§42–50, 60; platform/runtime §§43–53.

The proposal distinguishes a durable *desired* authoring revision from an *active runtime* revision and says a pre-tick barrier advances world state, required products and the manifest pointer together. P7 separately checkpoints a save whose header names a content manifest revision. It leaves journal retention, crash recovery and compaction as implementation policy. That does not yet specify a durable linearization point or replay rule across the desired authoring record, active manifest root, activation tick and P7 overlay. An in-memory barrier does not by itself make a multi-record durable commit.

Counterexample: an edit is accepted and durable as desired `R+1`; its products cook and the runtime activates `R+1` at tick `n`; a crash occurs before a new save checkpoint or active-root record is durably published. The prior save names `R` and contains an override/delta for an object changed by the edit. On restart, either applying the old overlay to desired `R+1`, silently dropping the accepted edit by using `R`, or replaying `R+1` without an activation decision can change the world. The current documents do not choose the outcome or define the overlay migration precondition. The symmetric crash (save/root written before required product bytes or source transaction are durable) also needs a rule.

**Correction condition:** identify the single durable scene/content/save root or journal commit record that names active source revision, manifest/product closure, world tick and save-overlay ancestry. Specify write/flush order, recovery scanning, how pending/failed desired edits resume or remain pending, and whether a save is restored against its recorded root or migrated to a later one. Publication to runtime may remain at the pre-tick barrier, but crash recovery must have one unambiguous committed generation. SQLite's [single-file atomic commit §§3–4 and multi-file commit §5](https://www.sqlite.org/atomiccommit.html) illustrate why coordinated durable records need an explicit commit point; the engine need not use SQLite.

### F2 — Save compatibility is a failure response, not yet a release guarantee

**Severity:** high for the stated supported-schema-change requirement; blocks high-level design of release compatibility policy. **Affected:** C10; world/content §§19, 23, 61–65, 90; tools/delivery §§52, 60–64; platform/runtime §88.

World/content correctly says a semantic prefab or recipe change can require migration, and restore fails without a required manifest or approved mapping while preserving the old save. That protects bytes but does not deliver the promised ability to continue a supported save after a shipped content/schema update. T2's release gate names package load/restore/offline tests, but no compatibility window, prior-save corpus, old-content retention rule, or migration gate is selected. Product hashes being caches is compatible with logical IDs only if the new package supplies either the prior semantic revision or a migration for each saved delta and required dependency.

Counterexample: an Android update replaces a prefab node or procedural recipe; the installed app's old save names the prior manifest, whose products are no longer packaged. On-demand packs may be invalidated during the app update. A valid checksum and intact save still cannot load. Android's [Play Asset Delivery “Asset updates” and “Delivery modes”](https://developer.android.com/guide/playcore/asset-delivery) explicitly describe temporary asset unavailability and invalidation across updates; the package must not assume the old pack remains present.

**Correction condition:** choose a supported compatibility window; for every released prior save in that window, require a migration chain and new-content dependency closure, or retain/fetch the exact prior revision under a defined offline policy. Gate package publication by old-save fixtures across schema, prefab/recipe and asset changes, with transactional migration that preserves the original save until success. An unsupported historical save may fail explicitly, but “supported change” cannot mean a load failure with intact bytes.

### F3 — Publication scope for shared edits remains ambiguous

**Severity:** high if shared gameplay products can change; blocks integrated high-level publication design. **Affected:** C05; world/content §§43, 51–55; tools/delivery §§44–48, 68; play systems §§40–50.

Tools/delivery requires full dependency closure for a shared prefab/material edit, while world/content explains an active *region* replacement and one manifest-pointer commit. The docs do not explicitly define whether all active dependent instances/regions and their P3/P5/gameplay representations form one publication set, or whether old/new versions may coexist under version-pinned region manifests. This is consequential for physics and navigation, not a request to replace all optional visuals atomically.

Counterexample: prefab `P` supplies a collider to cells A and B, both active. The cook updates `P`, the global manifest points to its new product, and W2 replaces A at the next barrier while B still has the old P3 collider/nav tile. A query or object crossing the border can see inconsistent semantics even though each region individually committed. F4's source-hash closure alone does not install the affected consumer representations.

**Correction condition:** define the affected active-instance/region set from the dependency graph, then publish required authoritative replacements for that set at one barrier after reservation, or explicitly pin per-region product revisions and prove their cross-border compatibility until all are replaced. Include retiring/inactive regions and shared references in this rule. Test a shared collider/nav edit across two active cells with one pending retirement. The documents' “complete replacement” language is directionally right; this finding asks for the missing scope, not another asset database.

### F4 — Durable storage headroom and browser persistence are outside the admission model

**Severity:** medium; blocks detailed build readiness and platform qualification, not the core ownership model. **Affected:** C05/C12; world/content §§47, 61–63, 86, 90; platform/runtime §§59–61, 88–102; tools/delivery §§48, 74.

The memory model accounts for decode, upload and old/new overlap, but the durable workflow also temporarily retains old/new scene source, cooked products, checkpoints and journals. Save cost is estimated as dirty records/journal bytes, yet a checkpoint generation and prior valid generation coexist until publication. A full disk or browser quota can therefore reject the *next save* even when RAM admission passed. The design states that browser cache is not save authority, but it does not choose what backing store/permission or export path makes browser-authored scene changes durable enough for the stated workflow.

**Correction condition:** add a separate storage-space admission and garbage-collection ledger: `D_peak ≥ D_prior_valid + D_new_checkpoint/journal + D_staged_source/products + D_required_retained_revisions + reserve`, counting deduplicated data once and including worst-case journal/old-reader retention. Define how save/write failure is reported while retaining the prior generation; define browser best-effort versus requested persistent storage and a user export/backup policy when local durability cannot be promised. The [WHATWG Storage Standard §§4.5, 6, 7.1](https://storage.spec.whatwg.org/) says default local buckets are best-effort and can be cleared under storage pressure; [IndexedDB 3.0 transaction durability](https://w3c.github.io/IndexedDB/#transaction-durability) distinguishes strict from relaxed hints. Neither source promises protection from explicit user deletion.

### F5 — Bounded retry journal needs an aging discriminator

**Severity:** medium; detailed operation-contract gap. **Affected:** C10; tools/delivery §46.

The tools design says a bounded durable deduplication journal returns the original result for a matching key, while an aged-out token is rejected as ambiguous. Once the key has been erased, a raw repeated key cannot be distinguished from a never-seen key; the stated rejection needs an additional protocol rule. Without it, a disconnected agent can replay a completed edit after journal compaction and create another revision.

**Correction condition:** include a durable operation epoch/expiry in the token and reject all tokens from retired epochs, or retain a compact high-water/tombstone record sufficient to distinguish aged-out keys. Define the query-by-operation-ID lifetime and how a client obtains a fresh expected revision after expiry. The exact index and retention duration are implementation choices; the ambiguity rule must exist in the shared contract.

## Lifecycle invariants to carry into reconciliation

1. **Identity:** stable scene IDs and field IDs survive recook; runtime slots, product hashes and spatial cells do not become save keys. A deleted ID has a tombstone through every pending publication and save overlay that could refer to it.
2. **Publication:** the authoritative world, required P3/P5/gameplay representations, activation epoch and compatible manifest/root form one visible generation. Every async completion must prove its generation and lease before publish. Optional presentation can follow only under a declared compatibility/version rule.
3. **Cancellation and undo:** cancellation prevents future publication; it does not pretend to undo already active gameplay. Undo/redo are validated forward edits with current revision preconditions. Crash recovery returns a pending edit to a named state rather than silently reapplying it.
4. **Save/recovery:** a checkpoint names the source/content root and overlay ancestry it was derived from; restore either reconstructs that generation or runs an approved migration before exposing W1. The previous valid checkpoint remains until the new one is durably committed.
5. **Cost:** memory peak includes live, staged, pinned, replacement and transient resources; storage peak separately includes two valid generations, journal and retained content. Profile-off frees avoidable work only after dependent leases and GPU fences retire. Unknown device budgets remain unqualified.

## Check of the coordinator's proposed reconciliation

After the findings were reported, the coordinator proposed one immutable `RecoveryRoot`, published by an atomic pointer/transaction with compare-and-swap generation. It would link the durable desired authoring head and accepted-source journal to a self-consistent checkpoint tuple: tick, base authored revision, active manifest, overlay/schema/provider versions. Referenced restore inputs would be flushed and verified before root publication. Runtime may run ahead; a crash restores exactly the last durable checkpoint tuple, exposes unapplied desired edits as pending, and reuses the normal cook/activation transaction. The proposal also retains prior content closure for supported saves or migrates and publishes replacement roots before eviction/update. This is sufficient to resolve F1/F2 **at high-level**, provided “completed runtime edit” means applied to the running world while only “completed save” promises a durable checkpoint, and the periodic rollback window is explicit. A root may reference verified immutable products *or* durable, reproducible recook inputs/toolchain; it need not keep every platform variant resident, but it cannot depend on unavailable data.

For F3 the coordinator proposed staging the entire active dependent instance/region closure and atomically swapping required owner representations with the manifest at one pre-tick barrier; inactive regions upgrade lazily against recorded source/overlay ancestry. This closes the two-cell counterexample if the barrier rechecks the affected set against concurrent activation and retirement. A region made active after the closure snapshot must either join the same publication or force retry; merely validating a prefab hash is insufficient. Version-pinned optional presentation products may coexist only under a declared compatibility rule. These are review conditions on the proposed reconciliation, not claims that the governing documents have already been revised or built.

## Primary evidence inspected and its limits

| Source / inspected section | Relevant fact and limit |
| --- | --- |
| SQLite, [Atomic Commit In SQLite](https://www.sqlite.org/atomiccommit.html), §§3.7–3.12, 4, 5.1–5.6 | Flush order, recovery journal and a coordinated multi-file commit point show why a pointer rename alone is not a complete cross-record recovery protocol. This is transaction precedent, not a mandate to use SQLite or proof for browser storage. |
| W3C, [Indexed Database API 3.0](https://w3c.github.io/IndexedDB/), “Transaction” durability hint and `transaction()` options | `strict` and `relaxed` have different commit-durability behavior; a browser adapter must choose/report its policy. A hint is not absolute power-loss or user-deletion insurance. Editor's draft inspected, 30 October 2026. |
| WHATWG, [Storage Standard](https://storage.spec.whatwg.org/), §§4.5, 5, 6, 7.1 | Local storage starts best-effort; quota is implementation-defined; best-effort buckets can be cleared under pressure. Supports treating browser-local scene/save persistence as a policy, not assuming a cache is durable. Living standard inspected 31 October 2026. |
| Android Developers, [Play Asset Delivery](https://developer.android.com/guide/playcore/asset-delivery), “Delivery modes” and “Asset updates” | Fast-follow/on-demand packs can be absent or invalidated during updates. Supports an explicit save/content availability rule; no measurement of this engine's Android package was made. |
| Unity, [AssetDatabase.RegisterCustomDependency](https://docs.unity3d.com/ja/current/ScriptReference/AssetDatabase.RegisterCustomDependency.html), description | A changed declared dependency hash triggers reimport. Supports dependency-aware cooking, not atomic publication to physics/nav/save consumers. |

The review did not inspect source code, create a prototype, exercise a crash injector or qualify hardware. Findings F1–F3 require contract reconciliation by the coordinator and affected owners; F4–F5 can follow as bounded detailed policies once their invariants are accepted. Source material was read as documentation/specification, not as measured behavior of this proposed engine.

## Closure read of revised governing design

31 October 2026, after coordinator reconciliation. This section reviews the *revised design*, not runtime behavior. Inspected SHA-256 file identities:

| Governing file | SHA-256 |
| --- | --- |
| [CONTRACTS.md](../CONTRACTS.md) | `0ca5008fd95e9d27178a8c6a1f452ac03498987513ebc5315d0982eb09200d95` |
| [WORLD-CONTENT.md](../modules/WORLD-CONTENT.md) | `a21f9e8baba9126f3001479a1a5470858dff12ca311e43cc4dc226453c17e462` |
| [TOOLS-DELIVERY.md](../modules/TOOLS-DELIVERY.md) | `154bbd4fc22d88d0dee0a1c32a7c3dde2e00a4f29cef4296e6d828f1dd4aaeff` |
| [INTEGRATION-REVIEW.md](../INTEGRATION-REVIEW.md) | `97772795556bca1bfc0bb6bb891524508f6f98418aded6cdf03c9ab0e7bf177b` |

| Finding | Closure check against original counterexample | Disposition at this design revision |
| --- | --- | --- |
| F1 crash pairing | C10 now makes one immutable RecoveryRoot the sole durable restore choice. Its checkpoint tuple binds tick, base authored revision, manifest, overlay and schema/provider versions; root publication has a generation precondition, verified restore closure and operation outcomes. Runtime can run ahead, but restore chooses exactly the checkpoint tuple and exposes newer desired edits as pending. A crash between active `R+1` and its save therefore restores `R`, without applying `R`'s overlay to `R+1` source. | **Closed at high-level.** The adapter must implement the promised atomicity/flush order; crash injection remains unrun. Periodic checkpoint rollback is explicitly disclosed. |
| F2 supported save after update | C10 now requires retained required products or qualified recoverable input/toolchain closure for supported save roots, and migration plus replacement-root publication before eviction or package update invalidates that closure. The old Android save can therefore either restore against retained content or be migrated before old content disappears. | **Closed at high-level.** The compatibility window and old-save fixture matrix remain product/release qualification details; unsupported or externally deleted saves remain diagnosed failures. |
| F3 shared prefab edit | C05 and world/content now stage the entire active dependent instance/region closure, reserve its required products, recheck the active-set generation at pre-tick commit, and include a newly active dependent or abort/reprepare. Manifest and required representations swap together; inactive regions keep base/overlay ancestry and upgrade before activation. The two-cell collider/nav counterexample cannot publish a mixed required generation under this contract. | **Closed at high-level.** Actual multi-region atomicity, shared-reference coverage and peak stress are unbuilt. |
| F4 durable storage peak | C10 now separately reserves prior valid generation, new checkpoint/journal, staged source/products, retained revisions and headroom, counting shared bytes once. Failed admission leaves the prior root valid; browser persistence/eviction and export/backup are declared adapter policies. | **Closed as an architectural obligation.** Exact quota estimates, persistence grants and storage-adapter failure tests remain detailed/platform work. |
| F5 retry after compaction | C10 now includes a durable operation epoch in tokens, retains its deduplication index until pending outcomes are recoverable, and explicitly retires epochs. A token from a retired epoch is rejected as expired; it cannot be mistaken for a never-seen key. Root/journal publication records enough outcome state for a lost-response retry. | **Closed at high-level.** Token format, compaction schedule and crash tests remain implementation work. |

No remaining high-level contradiction was found in F1–F5 at these file hashes. The old finding text above remains the pre-reconciliation challenge and counterexample record; this closure table is its disposition. I did not inspect an implementation, exercise recovery, verify actual browser storage durability or certify any device budget. If the root/manifest/save publication model changes again, these closures require a new check against the same counterexamples.
