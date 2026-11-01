# Engine tooling, composition and delivery — F2, T1, T2

**Initial implementation specification:** [FOUNDATION-SPEC.md](../FOUNDATION-SPEC.md) now selects the concrete baseline mechanisms,
limits and failure rules. This family document retains the wider responsibility; specification does not imply runtime qualification.

Status: selected high-level design, 1 November 2026, under the [architecture decisions](../ARCHITECTURE-DECISIONS.md). Mechanisms are reasoned from the [architecture brief](../ARCHITECTURE-BRIEF.md), [programme](../DESIGN-PROGRAMME.md), [shared contracts](../CONTRACTS.md) and [network session contract](NETWORKING.md); no implementation, device qualification, visual acceptance or release is claimed. Primary evidence and its limits are in [TOOLS-SOURCES](../research/TOOLS-SOURCES.md). This module serves a custom, Bevy-independent 3D engine for contrasting cameras/games on native desktop, browser and native mobile. AI is a **development client** of inspectable operations and evidence; no runtime LLM, paid service or general scripting language is required.

## Observable capabilities and ownership

| Need and basis | Recommended owner and mechanism | Success/failure observable |
| --- | --- | --- |
| E02/E03/E07–E11: select meaningful capabilities, providers and quality for each game/platform | **F2** resolves typed provider constraints against one profile, platform limits and budgets. F1 commits transitions; owners retire resources. | Valid plan states actual work and substitutes; impossible plan returns a cause chain. Invisible clouds can still supply light attenuation. |
| E12: humans and agents edit and inspect the same world/content | **T1** offers typed commands and queries over F3 schemas and W1/P7 authority; UI, CLI and local API are adapters. | An edit has a revision, scope, semantic diff, correlation ID, undo policy and pending/finished state. No editor-only truth. |
| E04/E09–E12: repeatable diagnosis, demos and native delivery | **T2** drives headless scenarios, captures/traces, build/cook/package gates and evidence records. Platform adapters own SDK integration and secrets. | A build can be traced to inputs; failure reports the exact violated gate. Appearance, correctness and device performance stay separate. |

F3 owns stable IDs, units, schema registry and compatibility rules. F4 owns source-to-cooked dependency hashes, products, loading and leases; it is **not** replaced by a tool asset database. W1 owns authoritative scene/instance state, W2 owns region activation, P7 owns save/replay and committed deltas. T1 commands those owners through one boundary and keeps the bounded operation/outcome journal, not a second mutable authored truth. One admitted immutable file-tree revision is the source used by each build; distinct W1 play instances pin that revision and keep private saves. The [asset storage contract](../../ASSET-LIFECYCLE.md) remains canonical for existing factory `asset.json`, selected runtime delivery manifests, paths and retention. New engine formats are projections/adapters of these records where applicable, not a bulk repository migration. F1 owns tick/frame barriers; V1 owns GPU resource lifetime; P2/P3/P4/P6 and A1/U1 remain consumers of validated changes.

| Shared contract | Tool-side obligation and owner handoff |
| --- | --- |
| C01 identity/units | T1 targets F3 stable opaque authored IDs and stable field IDs; it never serializes runtime slots. Import conversion and units are F3/W1 rules. |
| C02 committed time | Operation acceptance does not mutate a tick; F1 admits the prepared active delta at a pre-tick barrier, and T2 records the activation tick. |
| C03 authority/order | T1 cannot write a body pose or game rule around W1/P3/P4/P6; commands enter their declared authority/order. |
| C04 composition | F2 resolves typed provider/quality graph; T1 exposes diagnosis, T2 traces selected plan and switch cost. |
| C05 content/retirement | F4 cooks and leases immutable products; W2 activates region generations; T1/T2 expose admitted versus active and memory headroom. |
| C06 GPU lifetime | T1 never creates GPU handles; V1 prepares/retires GPU products by device generation and fences; T2 inspects passes/resources. |
| C07 views/history | A world edit or profile change carries affected view/history invalidation to V2/V4 and effect owners; captures name the actual view and sample. |
| C08 light/material transport | F2 checks semantic ports/units and substitutes before V3/E1/V4 composition; a material edit checks all supported paths. |
| C09 environment/physics exchange | T1 environmental edits preserve E1/P3 field units and authored-vs-physical scale; T2 scenarios test the coupled effect. |
| C10 persistence/operations | T1 proposes patches against expected admitted revisions; a coordinated writer applies ordinary files, then explicit admission durably pins the exact tree/root/outcome. P7 owns separate save roots/checkpoints. |
| C11 input/audio/UI | U1 editor controls and A1 event handling consume committed state; editor focus/input does not bypass game action priority. |
| C12 budgets/evidence | T2 attributes complete critical paths, peaks and gates; root maturity ledger separates design, implementation, function, appearance and platform proof. |

## First-use obligations

The [AI development design](../AI-DEVELOPMENT.md) specifies two clients of these same tools: engine contributors
working across relevant contracts/consumers, and game authors using the public SDK. F3 provides discoverable versioned
schema/type metadata; F2 exposes the selected effective plan and provenance; T1 supplies bounded inspection, typed
edits and job/status queries through the existing authority. T2 gives human-readable and structured results with exact
source/build/target/check scope and passed/failed/skipped/unavailable/inconclusive outcomes. Zero selected tests or an
unavailable target cannot be reported as behavioral proof. Large traces/captures are linked artifacts.

T2's check selection includes code, schema, content, shader and configuration dependencies, including deleted edges;
unknown impact uses a broader relevant check. Compiler/API visibility and selected semantic lints enforce boundaries
before runtime where possible; registration/graph checks and dev assertions cover different guarantees. Capture
provenance includes actual world snapshots and presentation/history samples. A headless semantic-action test is
separate from exercising pointer input/focus/picking and visual feedback. T1/T2 need no agent-only mutation or runtime
LLM; an optional local API/MCP transport shares the CLI/workbench implementation.

Ship contributor and game SDK instructions/examples matched to actual API/tool versions, with explicit inheritance
scope. Proposed contribution/game/diagnosis skills follow implemented workflows, not fictional commands. Standalone
compiler/lint/docs and headless fixtures can precede implementation of the selected A01/A02 source/session protocol;
A03 must supply shader demand/readiness before the supported cooked render path. Actual device/visual evidence and
open A04/A05 combinations remain separate. Current Broadsides policy and lint rules are not changed by this proposal.

The [from-scratch review](../FROM-SCRATCH-WALKTHROUGHS.md) further requires a valid empty authoring draft and distinct
edit, play and build readiness. Applicable schema/operation checks remain mandatory; missing runtime entry or products
must not prevent saving a well-formed draft. A build pins its candidate source before cooking; play activation or
publication of a usable export waits for its selected target's required closure. Readiness is metadata for that
source/target, not another canonical source database. These priorities
refine the blanket validation wording below; D1 must specify exact file/transaction formats for the selected source protocol. Malformed external
files are preserved for repair and never treated as an admitted executable snapshot.

Guided additions and direct setup create the same ordinary source; headless-first requires no game camera or graphics
device. T1 identifies navigation cameras, grids and inspection lights as workbench helpers, not exported game objects.
Runtime visibility and lighting follow the authored view/material/lighting choices, including deliberate empty/unlit
scenes. Inspection follows registration through activation and matched objects, input consumption and state/events;
game-specific rejection reasons must be authored, not inferred from arbitrary code. T1/P6 document the code or reusable
behavior handoff for artists; a visual workbench alone is not a no-code rule editor.

T2 distinguishes source handoff to another developer, a playable development export, and release qualification. A
clean source checkout rebuilds from pinned tools and available masters/dependencies; an export recipient needs no
authoring SDK or warm project cache. Development exports report unqualified claims and need their target packaging
slice, not completion of every release gate. Installed offline play, offline cooking and offline dependency/SDK setup
are separate support claims. This does not introduce a required cloud service or airgapped SDK distribution.

The [first-use review](../FIRST-USE-WALKTHROUGHS.md) prioritizes a small complete developer path within these owners.
T2 checks prerequisites for one selected target and keeps starter code, docs, assets and engine/toolchain versions
aligned. F2 exposes one effective setup with named defaults and provenance; a beginner need not hand-edit every
internal declaration. T1 links a selected object/action to source and shows source, pending and active play revisions.
Diagnostics lead with the object/action, cause and deliberate repair, with technical dependency detail available.
Compile success, content/shader readiness and device qualification remain distinct target statuses.

T1's minimum visual workbench frames/selects objects, exposes camera/light/material fields and undo through the same
operation surface, and previews the selected runtime or labels a proxy/fidelity difference. It does not require a
general node editor or full authoring suite. F3/F4 own canonical identity and stable import correspondence; T1 exposes
import conversions, unsupported content, authored overrides and added/removed/ambiguous mappings for review. Reimport
must not guess away a broken attachment or overwrite an authored material. W1 and affected consumers validate the
result. FOUNDATION-SPEC now fixes imported-ID mapping, JSON source, transactions and rejected-operation metadata semantics.

The starter example teaches action → rule → committed state → feedback, a rejected action and an explicit save
choice. Editor undo, game undo, private play saves and authored apply-back remain distinct. Explicit private test-save
retention may support restart/migration testing; stopping play discards unsaved state, not every retained test artifact.
Replacing all sample content must preserve the ordinary setup/build path. A plain library integration comes before
specialist provider examples; the latter need a public contract and a small consumer fixture for their claimed
operations/targets. I2 supplies import/operations, I3 the first complete visual path and I4 target export qualification.
Neither a paper recipe nor a CLI-only capability qualifies artist usability. Do not delay a simple starter for
unrelated specialist environmental adapters.

## Capability survey refinements

The [capability survey](../CAPABILITY-SURVEY.md) makes the existing responsibilities visible as complete creator
workflows. H01/H02/H03/H11 require character/transition preview and traces, imported blockout/landscape and vegetation
placement, material/effect look development, and sequence/capture inspection. Each uses its owning module's source
and operations. A generic inspector alone does not establish those workflows; neither is a general graph editor,
terrain sculptor, scripting VM or offline movie exporter mandatory for the first usable engine.

H04 adds translation production to existing U1 runtime support: extract/interchange keyed messages, diagnose
missing/obsolete entries, preview expansion/pseudo-localization, and validate selected locale assets/fonts. F4 retains
asset identity and cook inclusion. H05 requires capture evidence to state which canvas, DOM or native UI surfaces
were included; a canvas-only image cannot certify platform text controls or the complete accessible interface.

H06/H08 expose the existing build graph to creators: named source-controlled build configurations, reasons for asset
inclusion/exclusion, declared dynamic-content roots/families, and output bytes attributed to code, assets and decoders
where available. Unknown attribution is explicit. Build mode/signing are distinct from capability/quality selection.
Report authoring OS, build host/toolchain, player target, backend and feature-specific availability separately;
browser content inside a mobile shell is not native target qualification. Existing startup/on-demand/offline/residency
contracts become understandable target-specific reports, not a second delivery subsystem.

H09 requires an iteration matrix for property, imported-content, shader/material, game-code, schema and profile
changes. Each entry states edit/play scope, restart, retained state, failed-change recovery and target availability.
Game-code rebuild/restart is the baseline; asset reload does not imply arbitrary binary reload. The selected source/session
boundary and A03 shader readiness must be implemented before advertising affected seamless edits.

## F2: finite capability resolution

The [composition model](../COMPOSITION.md) defines direct component/system registration, explicit bindings, public
extension points and content admission, with package/removal rules only where those features are requested. F2 validates
the configured application; F3 owns schemas, F1 validates schedules and W1 owns component mutation. An optional package
does not acquire private world, GPU or tool authorities by being installed.

The **game setup** explicitly selects capabilities and meaningful model/provider choices. Target profiles select
content entry points, supported visual tiers, platform limits and budgets. Named defaults may expand into ordinary
bindings, and an author may declare an ordered presentation fallback. The resolved plan is inspectable and locked;
a general provider search/optimization framework is not the baseline. The
[scenario comparison](../SCENARIO-WALKTHROUGHS.md) favors this simpler composition over a universal plugin registry.

A selected implementation declares supplied and required semantic ports, units/scope, incompatible owners, valid
content/model regimes, target limits, products, lifecycle and cost/invalidation facts. Direct game schemas/systems
register their actual requirements without a separately versioned package descriptor. Validate each required input
against its explicit provider or declared combiner; reject missing requirements, duplicate authorities and unsupported
cycles. A solver may own a declared internal cycle/order. Defaults and fallbacks cannot silently choose a different
physical/gameplay model. The resulting plan names implementation versions, content/schema closure, target/tier and
why each item is retained. World/domain/view scope remains distinct.

For a configured graph, dependency/topological validation is O(V+E); validate finite selected target/tier combinations
and their declared transitions at build/load. Cache by setup, limits, content/schema and implementation revisions.
Restrict live quality changes to those admitted plans. Do not search an open implementation space or promise automatic
minimum-cost model selection. Diagnostics name the requirement chain and a concrete choice: for example, retained
cloud shadow requires an attenuation source even though cloud drawing is omitted. Optional reusable packages can
contribute declarations through the same interfaces when distribution or reuse warrants packaging.

**Build exclusion** removes code/data/providers and their products from the executable/package; changing it requires rebuild and cannot be promised live. **Load selection** chooses from compiled providers, waits for required products and may enter a visible loading state. **Live transition** first validates target graph and game consent, reserves peak overlap including old/new products, prepares the new graph, and at one F1 safe boundary swaps the profile generation. It then fans out invalidation to V1/V2/V3/V4/E1 and other owners according to affected inputs, retires old work only after leases/fences, and reports completion. Any failed preparation or invalidated epoch leaves the old plan active and releases staged resources. A gameplay-authoritative resolution change cannot be slipped into a view-quality switch. A cosmetic off switch removes avoidable jobs/caches but keeps services consumed elsewhere. If there is no cloud attenuation provider, the Sun visibility input is explicitly `1`, not an absent global lookup.

Within the finite author-selected visual tiers, a controller may lower quality under sustained deadline/memory
pressure and restore it after a longer stable window plus replacement headroom. This is bounded tier selection with
hysteresis, not general model optimization. Exact windows and budgets remain profile data to qualify. F2 reports
predicted cost/removed work; T2 attaches observed timing, residency and thermal evidence. Two views share world
providers while selecting compatible per-view representations and histories. Gameplay model changes require a
separate game decision, normally followed by rebuild/restart.

## T1: one operation and query surface

[FOUNDATION-SPEC §§1–4](../FOUNDATION-SPEC.md) owns the exact file format, writer/application/admission order, SQLite durability, retry, crash recovery, retention and GC algorithm. [C10](../CONTRACTS.md#c10-persistence-reproducibility-and-authored-operations--p7f3t1) owns the invariant. T1 is their user-facing command/query surface: ordinary working files are the editable source; one explicit immutable admission pins exact bytes for cook/build; source, active run and save revisions are distinct. Initial authoring is native. Browser play/export uses runtime cache and player-save persistence, without a full browser editor/source authority.

A typed workbench/CLI/agent operation takes stable ID, schema version, expected admitted SourceRevision, read/write preconditions, parameters, correlation ID and idempotency token. It produces a **proposed reviewable patch** and semantic/dependency impact report. The UI and agent can inspect it before any write. Preserve-world inputs come from that expected admitted authored revision, never unadmitted dirty files or play pose. A coordinated writer applies the proposal to ordinary files only when affected working paths still match its base; it records branch, intended paths and an unaccepted intent. The user/agent then explicitly admits a complete selected file set. T1 verifies/captures bytes, stable IDs, dependencies, deletions, renames and required LFS payloads; F3/F4 validate and the native internal store commits root/outcome by generation CAS. `accepted` means admitted exact source durability under the adapter's guarantee, **after** visible file application. It does not mean runnable, cooked, active or saved. Builds never read a mutable partially written checkout.

A watcher reports `working-changed` only. A captured immutable Git tree can define a complete set; dirty source requires an initiator-declared finished set and verified captured bytes. Noncooperating writers can race an advisory lease and defeat a point-in-time claim, including ABA. T1 reports conflict/uncertain capture and preserves observed files and proposal rather than silently rebasing or overwriting. Crash during multi-file apply leaves files plus unaccepted intent for review; crash after root commit recovers the exact admitted outcome even if working files subsequently changed. Queries expose working fingerprint/dirtiness, admitted revision, proposal/conflict, readiness, active build/session pin and checkpoint separately. Retry and undo use Foundation's bounded epoch/retention rules; undo is another proposed forward file change, never Git history or save rollback. External publication/signing/upload are separate commands.

`startPlay` pins target-ready SourceRevision, BuildRevision, products and profile; it constructs fresh session/world epochs, clocks, provider instances and private test-save routing. Native development uses an isolated child process by default. A substantive source/gameplay/physics/provider edit builds then restarts the disposable run. A failure leaves the old build visibly pinned and source editable. A finite preloaded world or explicit stop/load boundary is sufficient for the first game; no universal provider state-transfer gate is imposed. Only qualified presentation fields may preview live under C05's version/activation rules. Future live streaming must satisfy coherent region publication and an actual scoped capacity proof. Multiplayer server matches pin gameplay content until their declared restart/migration; replicas cannot apply back. Native game code may have external effects; child-process isolation is not a security sandbox.

T1 queries schema, object/component state, provenance, dependency closure, plan/diff, pending jobs, capability diagnosis, diagnostics and evidence links. Results name revision/snapshot and pagination limit. F3 owns schema/type meaning; W1/P7 and F4 own their respective application and products. P7 player saves use separate authority roots; completed runtime application does not promise a durable checkpoint. Source handoff/export packages exact admitted bytes in a separate bundle, or uses a verified matching VCS tree/LFS closure, with code, masters, recipes and pinned toolchain/dependency declarations. T2 compares its recipient manifest to SourceRevision before release; export does not overwrite working files or carry the internal SQLite/cache store. Current dirty files or an internal snapshot alone are not a reproducible handoff.

The UI inspector and scene/prefab editor render F3 field metadata (type, units, range, ownership, default, provenance and allowed operations). It supports semantic grouping, multi-selection with explicit mixed values, preview using the selected runtime providers, stable-ID selection, dependency/impact view, scoped search, undo/redo and a view of pending cook/activation. It shows working files versus admitted snapshot, target readiness, active play pin and session ID separately. CLI emits structured JSON plus concise human diagnostics; a local authenticated API uses the same command/query schema and rate/size limits. The API is off in shipped games unless a deliberate developer build enables it. Direct file editing is the normal source-control path; explicit admission is the build boundary. The editor never invents a private field that cannot be exported, loaded and inspected by a headless client. Bevy Remote's query/mutation protocol and Godot's inspector/undo facilities are precedents for these affordances, not runtime dependencies or full transaction models; the latter is a derived engine rule.

**Apply-back and spatial authoring.** Apply-back selects only schema-marked authored properties by stable source ID from a pinned developer play session. T1 shows `(pinned base, selected play value, current admitted source)` and creates an ordinary typed batch operation with current read/write preconditions. A touched field changed concurrently, deleted/ambiguous mapping or incompatible schema yields a repairable conflict; no live solver pose, inventory, runtime handle, event cursor or simulated reward is copied by default. The result is a reviewable patch, coordinated file application, explicit admission, build/restart and forward undo. Editor undo, game undo, private play saves and server administration remain separate. For WP05 artist acceptance, trace a real imported prop through place, rotate, signed/nonuniform scale, duplicate/instance, game-camera preview, undo, preserve-local and preserve-world reparent, reimport with material override retention, and selected-target build. Include a rotated/nonuniform ancestor whose preserve-world local matrix requires shear: W1/T1 must reject with residual/limit and a preserve-local alternative, not silently decompose it. Show imported identity/override conflicts rather than retargeting a collider or replacing an authored material. The trace must exercise the actual workbench and runtime preview before claiming artist usability; it does not require a full modeling suite.

Schema compatibility is an explicit matrix, not an unchecked version integer. A schema record has stable type ID, schema revision, field IDs, units/defaults, serialization rule and migration function from supported prior revisions. F3 validates monotonic deterministic migration with a source-hash record; a migration can create a new content revision, never rewrite the only source in place. Unknown required fields or newer incompatible schema fail with path/field and required version; harmless optional extensions may be preserved when the format supports round-trip. P7 decides save migration independently from authoring asset migration. Build gates reject a package if any target runtime cannot read the selected authored/cooked schema. JSON Schema 2020-12 is a useful interchange/structural validation dialect; cross-record semantic rules and native runtime layout remain engine-owned.

**WP04 SDK support policy.** Initial game projects pin one exact SDK/toolchain release and matching schemas, examples, packages and cooked-product recipes. That release's supported target matrix is published; a newer SDK does not silently rebuild or rewrite an older game, custom pass, source file or player save. No automatic N→N+1 upgrade, stable native binary ABI or multi-release support window is promised initially. T2 can reproduce a pinned build only while its recorded SDK/dependencies and required masters are available through a declared durable source. Before offering a breaking upgrade or compatibility window, publish an explicit migration/recook plan and exercise an ordinary game, one public render/feature extension and a player save across old and new releases, including failed-upgrade rollback to the old source/build/save root. Source changes require an inspectable diff and author acceptance; P7 save migration publishes a replacement root before retiring old restore inputs. This policy is not an I1 prerequisite for the single pinned starter, but it is an external SDK release/support gate.

## T2: traces, evidence and package pipeline

Diagnostics follow operation and dependency ownership. Each CPU job, asset fetch/decode/cook/upload, world tick system, view extraction, GPU pass/submission, audio work and package stage records a low-cardinality operation name, start/end or event time, owner, correlation ID, world tick or frame/view ID, profile/content/device generation, bytes/counts and outcome. High-cardinality stable IDs live in sampled trace attributes or targeted inspection, not unbounded metric labels. A trace records causal parent/edge IDs across async queues; a frame report shows critical path, parallel work, waiting on dependency, queue length/age, peak allocations and missed deadline. `sum(stage times)` is not frame time; compute the longest dependency path and compare actual wall/CPU/GPU intervals. Charge shared products once to their owner, report per-consumer lease/use separately, and mark estimated GPU bytes/timestamps as estimates when API support is absent. Debug overlays/captures have an explicit overhead flag and separate measured baseline. Traces have bounded ring buffers and opt-in export; production data does not depend on a telemetry service.

An executable scenario descriptor pins code revision, schema/profile/provider lock, exact content-product hashes, seed, initial authoritative snapshot, input/operation stream with tick order, captured external/nondeterministic results, platform/backend/device, expected assertions, view/camera/time and capture settings. Headless mode checks deterministic world transitions within P7's declared envelope; rendered captures check visual/perceptual differences separately because asynchronous presentation need not be bit-identical. Each capture carries image/video hash, source trace, method, camera/exposure, output gamut and reviewer decision. Comparison thresholds are per cue and platform, with human appearance acceptance recorded independently. Crash reports, live feature-switch failures and invalid configuration explanations include a replay bundle/redacted trace where possible.

The build graph is `admitted immutable SourceRevision+toolchain lock -> validate schemas/graph -> import/cook products -> stage immutable manifest -> compile per target -> assemble package -> smoke/functional/capture/performance gates -> sign/export adapter -> release candidate`. Edges use content hashes of admitted source bytes, recipes, toolchain/compiler/decoder versions, profile lock and target format; changing one input invalidates only its reverse dependency closure. If `A` assets and `E` dependencies exist, finding affected products is `O(A+E)` in the worst case and `O(changed closure)` with an indexed reverse graph. Cooked products and manifests are immutable by hash; staging can run in parallel, but publication is an atomic manifest/root pointer swap. The runtime never fetches a partly updated multi-file package. Record `SOURCE_DATE_EPOCH` and locked dependencies where the toolchain supports them; compare clean rebuild hashes and label nondeterministic outputs rather than asserting universal bit-identical builds. The deliverable provenance record includes admitted source-tree hash, source commit/working-file dirty state, exact SDK/toolchain versions, target triple, profile lock, schema revisions, content hashes, packaging adapter version and gate results. A stale exporter whose selected source or target closure has changed fails publication without disturbing the prior release.

**Gates are specific claims.** A developer preview may permit known visual gaps and unsigned artifacts; a release candidate requires schema/dependency/manifest integrity, a pinned selected active content revision with no pending/failed required edit, no unresolved hard provider edge, deterministic scenario assertions in the declared envelope, required content/asset checks, package load/restore/offline tests and a reviewed visual/performance matrix for supported targets. A failed or skipped test is recorded as failed/unqualified for its claim; no single green build promotes all maturity dimensions. Platform owners provide web bundle/service-worker cache policy; desktop package; iOS archive/export; Android app bundle/asset-pack adapters. Native SDKs handle code signing/provisioning and store submission at a narrow boundary with credentials outside engine artifacts and logs. T2 can prepare and validate candidate artifacts; it never implies store approval or legal compliance. Android asset packs can alter availability after install/update, so the manifest declares boot-critical offline bytes and F4's fetch state. iOS packaging mechanism remains subject to current SDK qualification; no deprecated delivery path is assumed.

The [canonical maturity document](../MATURITY.md) and machine ledger `../maturity.json` are coordinator-owned. T2 should validate each row's `moduleId`, `decisionStatus`, `designStatus` (inventory/mechanism/cross-boundary review), `implementationStatus` for the selected target, `functionalEvidence`, `visualEvidence`, `platformEvidence` and `limitations`, each with artifact revision and evidence URI/hash, method/configuration/date and reviewer where applicable. `unassessedCandidate` records existing code without treating it as absent or qualified. A transition to *designed* needs resolved required mechanisms and both-side contract review; *implemented* needs a build linked to the selected design; *functionally validated* needs reproducible assertions over the relevant interaction; *visually accepted* needs human-reviewed captures on named settings; *platform qualified* needs named device/backend, duration, p95/p99, peak memory/energy or thermal data and acceptance threshold. CI may validate schema, evidence links, hashes and required gates, but cannot auto-accept appearance or turn an estimate into measurement. No percentage-complete field should collapse these axes.

### Agent optimization campaigns

The [AI performance design](../AI-PERFORMANCE.md) specifies an additional T2 development workflow: versioned objectives,
workloads and evaluator identity; isolated candidate builds; bounded evaluation budgets; exclusive measurement leases;
raw/summary results and candidate lineage. Use the existing run/job/build interfaces and F4 products. Evaluators and
thresholds are separately owned/versioned from candidate edits; independent final runs check fresh inputs and the
integrated revision. Report cost, correctness, quality and target coverage separately, including inconclusive results.

F1/V1 expose bounded timing/attribution with backend limits; F2/F4 bound and identify target-specific variants and
specialization assumptions. Module owners supply cost/error models and conformance obligations. Shared-resource and
cross-module changes remain coordinated; local wins cannot promote whole-engine maturity. Start with one local A/A
noise check and A/B candidate path before adding remote workers or broad autonomous search. No runtime AI is required.

The [performance walkthroughs](../PERFORMANCE-WALKTHROUGHS.md) require each commission to define a meaningful end-to-end
completion event and its intermediate milestones. Reports include completed/pending/cancelled work, oldest outstanding
age and explicit blocked-loading durations, preventing deferred work from appearing as saved work. Candidate handoffs
include consumer/validity changes and shared resource/transition peaks. T2 records coherent combined-candidate evidence
and distinguishes local savings, integrated target attainment, quality/capacity gains and target-specific qualification.
These refine the existing result schema commission; they do not certify the illustrative campaigns or qualify the selected architecture as implemented.

## Worked cross-system paths

**Edit → admit → build/restart → capture → inspect/undo.** An agent queries `shore-03` by stable ID at admitted revision `R`, receives schema and linked prefab/material/collision provenance, and proposes a material edit with expected revision, idempotency key and correlation ID. T1 returns a semantic diff and reviewable ordinary-file patch. A coordinated writer applies it to matching working files, then explicit admission captures the complete declared tree and atomically publishes `R+1` with its outcome in the internal SQLite root. F4 cooks the admitted bytes; a fresh local run pins the new build and starts from preloaded gameplay closure. If cook/build fails, old run remains visibly pinned to `R` and working files plus admitted `R+1` remain available for repair. T2 captures the named camera against the new run and reports exact source/build/target, checks, timings and image sample. A rejected look produces a new reviewed inverse file patch and admission, subject to current revision and working-file preconditions. If another edit intervened, report conflict rather than overwriting. Optional live preview of a specifically qualified presentation field follows C05's coherent product/version barrier; this example does not imply live physics/nav/whole-world replacement. Crash and retry behavior is owned by [Foundation](../FOUNDATION-SPEC.md#2-file-application-and-source-admission).

**Live quality-provider switch.** Two views share one W1 snapshot and world Sun. A profile requests no visible clouds but cloud attenuation for a top-down view and higher quality shadows for a first-person view. F2 resolves one world attenuation source, distinct per-view shadow tiers and no cloud-volume draw. A mobile pressure event requests lower shadow quality. F2 checks the prevalidated candidate, reserves replacement peak and ensures it changes only presentation. F1 prepares new V1 resources, commits one profile generation after both views' dependencies are ready, invalidates each affected view/history, then retires old resources after fences. W1 is not ticked again; attenuation remains because its consumer remains. If headroom is absent, old profile stays active and the request reports `insufficient replacement headroom` with the minimum required bytes and a declared lower-cost candidate. A device-loss generation during preparation cancels stale publication and re-resolves against the restored adapter limits.

## Cost and proof order

The engine's worst-case authoring transition is not only steady state: `M_transition = M_live + M_staged_decode + M_upload + M_old/new_overlap + M_transient + reserve`, with physical/estimated/logical accounts separated under C05. At runtime, the critical-path lower bound is `max(longest dependency chain, total work / available effective capacity)` before bandwidth/queue contention; T2 reports those terms for combined scenes. A feature switch may temporarily cost more memory and stall preparation even when the target tier is cheaper, so admission and visible progress are required. Cache reuse reduces cook work only if all recipe/tool/profile/content inputs match exactly.

First validate on paper and in schema tests: provider closure/conflict explanations, authoring transaction/revision/idempotency invariants, and content manifest atomicity against F3/F4/W1/P7 contracts. Then implement the smallest complete path above with one streaming region, prefab/material edit, collision dependency, two views, save/reload and one provider switch; reject if a stale worker can publish, an edit is lost/duplicated, a region presents mixed generations, a consumer reads a missing provider, or rollback clobbers a concurrent edit. After structural correctness, qualify headless replay, capture comparability, clean-build hashes, target packages and device performance. These proofs test a reasoned design; they do not select its missing architecture by trial. Numeric budgets, SDK bridge details, release credentials and UI widgets remain implementation-specific; FOUNDATION-SPEC fixes source format and NETWORK-SPEC fixes the initial session access policy. Cross-module review with F3/F4/W1/W2/P7 and F1/V1 is required before this design is marked integrated.

## Bounded authoring and build storage

The [storage lifecycle](../STORAGE-LIFECYCLE.md) extends T2's resource accounting to peak disk reservations, live
free-space monitoring, private reusable worker/build roots, supported immutable cache sharing and compact campaign
evidence. T1 exposes exact candidate/source selection and publication status; F4 remains the source/product identity
and dependency authority. T2 starts with inventory and owner-scoped dry-run reporting, preserves unknown legacy work,
and coordinates leases and protected roots with publication before reclaiming anything. Expired heartbeat is a
reconciliation signal. Worker reuse requires verified isolation/reset. D1 specifies these protocols; I1 implements
local admission before autonomous bulk campaigns; I2 proves a restored source-to-release package. No tool here owns
app conversation deletion, and no collector is implemented by this note.

The [lifecycle walkthroughs](../LIFECYCLE-WALKTHROUGHS.md) keep creators on existing package/delivery/run records.
T2 reports retained versus summary-only/expired evidence and verifies dirty/generated winner inputs at their adopted
destination before retiring a worker. T1 exposes stale promotion and per-game chosen revisions. Cleanup completion,
source recovery, visual acceptance and integrated qualification stay separate. Retaining a named comparison is a
valid closeout disposition, not an automatic failure to finish. The initial path needs inspected manual operations,
not a universal collector or a creator-authored lease schema.

Preserve-world authoring evaluates old and candidate-parent world matrices from the operation’s expected admitted
authored SourceRevision, including already admitted edits, never a runtime/play pose or unadmitted checkout.
Cycle, conditioning, TRS representability and affected consumer checks complete on the candidate before source admission. Rejection advances neither admitted SourceRevision nor active world/manifest; metadata root generation may advance to record the rejected outcome. Pending unadmitted edits are a separate conflict/rebase concern, not an implicit transform input.
