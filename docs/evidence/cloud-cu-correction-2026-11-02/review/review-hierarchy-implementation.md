# Hierarchy implementation — independent review

2 November 2026. Reviewed actual convective-hierarchy.js, test_hierarchy.mjs, CuSourceField adapter, selected
CLOUD-CU-SOURCE.md and relevant ScalarField/caller synchronization. No source/canonical edits, GPU or commits.
Inspected source hash6ef1c7a1b273fc4c5ca772f80c1bf25847ae5e9d1de7ef6a299d3b642361f402,
CuSourceField hash4f22fe1e892d8cac9d0568c75b2cbc6c6e85a90a02fd9132e416c0e7a4cc8032 and tests
hashaebce9ddc8e8efa5333f7f5512e615910c4c0ba13e52db93e09ed2767cd2384b. Root/source author are addressing the
two findings below; this report preserves the reviewed pre-fix state rather than assuming their resolution.

## Actionable findings

### HIER-R1 — P2: failed asynchronous candidate creation leaks its partial GPU allocation

CuSourceField.create allocates ScalarField textures/uniforms and node/index buffers, then awaits shader/pipeline and
creates bind groups without local failure cleanup. If a later makeFrame/pipeline/bind-group operation rejects, the
promise never returns its partial source. chooseRecipe's `next=await create(...)` therefore leaves next undefined,
and its finally cannot destroy those already allocated resources. Repeated failed replacements can retain candidate
fields/buffers while the previous working source remains active.

**Correction:** make create dispose its own partial allocation before rethrowing, with destruction tolerant of
partially initialized members; similarly ensure any partially created ScalarField is owned through failure. Preserve
the previous valid source. Add a bounded mocked allocation/compile rejection test; no real GPU failure campaign is
needed to establish this control-flow correction.

**Scope:** successful normal destruction/in-flight ordering is otherwise coherent in the inspected caller. This is
a concrete failure-path leak, not a request for general device recovery or an engine provider framework.

### HIER-R2 — P2: structural admission permits malformed same-time index headers

CPU sampleHierarchyFrame, host prepare and WGSL hierarchyInputsValid check selected time/fingerprint/dimension fields,
but omit brick-shape/stride and metadata record-length consistency. GPU checks header logical counts against physical
buffer capacities but not every per-brick range/ID against logical live/entry bounds. Root's pools deliberately contain
unused/stale physical capacity. A compatible timestamp/tag is insufficient to make an inconsistent index usable.

**Concrete CPU counterexample executed without GPU/source edits:** create variant0/seed1729/time650 frame, clone
metadata, set metadata[2] (bx) to15 while retaining256grid, time and fingerprint. The CPU validator accepts it at
q=[95,161.33333333333334,-255] and returns0 instead of0.002618281430486036. The shader's validity predicate also lacks
bx=grid.x/16 (and corresponding by/bz) so does not declare that header unsupported. This does not mean the normal
builder currently emits a bad header; it demonstrates the claimed rejection boundary is incomplete.

**Correction:** one full structural admission per fresh immutable frame should require supported dimensions and
brick shape/stride, sufficient/exact metadata layout, finite admitted source parameters, expected tag/time/maxLevel/
bounds/grid, valid logical live/entry/local limits, contiguous in-range prefix/count records and referenced IDs<logical
live. Stable unique list ordering can also be checked. Mirror essential schema/shape/length/cap tests in WGSL without
scanning all bricks per point. A validated immutable-generation token is sufficient; do not add costly full scans per
voxel. Document that cached admission relies on typed arrays remaining immutable until retirement; a cached scalar
key cannot detect later array-content mutation. Arbitrary malicious omission of a valid contributor is outside this
finding; generated count/fill equivalence already addresses ordinary construction.

**Witness:** malformed same-age bx/stride/short-metadata, bad offset/count, invalid logical ID, tag/time/grid/maxLevel
and source-parameter fixtures reject at admission. A valid pooled frame with unused tail bytes remains accepted.
The source author received the exact counterexample and boundary details for the bounded fix.

## Checks that support retaining the implementation

I reran node test_hierarchy.mjs successfully:13496recipe/index/brute comparisons,2556full-brick corner/centre checks
and5856birth checks. Observed maximum raw candidates15 and nonfull primitive visits12152832 are below selected caps.
Four deterministic recoverable recipes contain488history records and maximum live128/129. These are correctness/count
observations, not timing, peak-memory or visual qualification.

Attachment starts in a filled parent; the admission condition birth+growth<=parent.stop keeps the parent uneroded
through child growth. Current-radius curvature and protrusion checks constrain the actual chosen scale combinations.
The hierarchy uses smaller exposed caps and monotone maxLevel ablation; tests require at least some fine caps to add
actual canonical support, plus local support born/lost across the mature interval. These demonstrate source changes,
not natural cauliflower appearance. Exposure checks specifically cover coarser bodies; overlapping same-level siblings
and their visual pattern remain an image concern, not a guarantee that every cap stays visible forever.

Retired parents retain reference poses independently of compact live nodes; the orphan fixture checks continued child
position/support. Float32-defined pose/erosion inputs are consumed by both CPU packed evaluators and shader. The
terminal plane uses final-radius scale and a bounded smooth front, reaches zero at finite retirement and preserves the
same-condensate union. Selected spheres avoid an unexplained ellipsoid-distance solve.

Count/prefix/fill builds conservative current-pose lists, with padded support ranges and identical repeated traversal.
The full-material test uses one sphere's farthest brick corner and conservative maximum erosion; it does not combine
unrelated corner witnesses or ignore a cleared parent. The tests include an eroded-but-nominally-live counterexample,
forced count/work limits and explicit empty/full/ordinary paths. The .01m numeric guard and full float32 error envelope
still need their promised interval/reference qualification; sampled equality does not certify every finite input.

Normal adapter use writes complete node/index/uniform data before submitting source→mips→consumer work. The viewer
serializes one frame, waits queue completion before next replacement and retires the old source only after readers
finish. Thus a single input pool is acceptable in this bounded proof; double buffering is not automatically required.
Same-source/time reuse and new-instance configuration switches keep current cache keys coherent. Cached typed arrays
are currently used read-only by this caller. General live model swaps or asynchronous multi-frame publication remain
outside this proof.

The material frame has bounded analytic C1 angular tracks. This is authored motion, not a verified physical head-roll
model; native normal/accelerated inspection must still judge whether the resulting small rotations and renewal look
coherent. No coarse/medium/fine image, breakup, lighting or full-sky acceptance follows from the CPU tests. Retain the
source/light seam and the prepared-master fallback if actual geometry still reads as balls, foam or beads.

Close HIER-R1/R2 against their actual revised code and targeted negative fixtures before declaring this implementation
slice complete. GPU field/header correspondence and native source appearance remain separate unrun gates here.
