# APPLE-TILE M10 builder record

14 November 2026. Initial boundary checkpoint: `1d21de31` (on top of `d9ca8c13`).
This is an incomplete builder record, not milestone acceptance. D10.1 and D10.3 pass;
D10.2, D10.4 and attended D10.5 remain open.

## Resumed verification on 07282556

The official `agent_loop.py check --module APPLE-TILE --milestone M10 --worktree . --timeout 3600 --json`
and its full `./dev check` pass on clean source `07282556025cc4fbeb3293b5b106a87c79b37d22`,
outside the sandbox under the repository GPU lock. Retained complete
[stdout](check-07282556.stdout.gz) and [stderr](check-07282556.stderr.gz) include the two graph-native tests,
all four integrated native lifecycle tests, fixture API-separation tests and required Web compilation.
The corrected coordinator test passes exact SDR/scene/output/history comparisons, five rejected
omission/stale/reset controls, cuts, healthy discard, background cancellation and acquire refusal.
The other lifecycle tests pass two held slots, failure/loss/extent replacement and old-generation
retention through completion delivery. These results support D10.1 and D10.3, together with the
earlier small fixture and source inspection; they do not replace T2 or simulator proof.

The [exact baseline proof](baseline-07282556.json.gz) passes 38 native ordinary-scene comparisons,
four native surface/letterbox comparisons and 1,924 Chrome day/night SDR/linear frame records in
951.34 seconds. No reference or tolerance changed. Native release executable SHA-256:
`94718c6c972b55634b89e02e9d3863ed6a0190b9a95cbd955cabe5be32b3dc0f`.
The automatic verifier retained provenance/manifests and released successful bulk sources;
these comparisons establish exact regression results, not a fresh visual inspection.

D10.1 source inspection: the independent scoped design review approved the boundary before implementation
([review](../design-review-1.md)); `native_pass` rejects transient/uninitialized native writers,
attachment access and late-bound surface access and retains the ordinary graph hazards. The
provider initializes distinct output/history textures through wgpu. `NativeFrame::encode` ends
the producer encoder, records a dedicated whole raw encoder and resumes public wgpu recording;
`FrameHost::finish_encoded_with` submits that ordered list once on its existing queue. The native
backend owns no surface, queue or completion. `NativeCompletion` retains its binding/provider and
textures until completion; dropping an uncommitted candidate invalidates provider history. Healthy
refusal flushes uploads, while background cancellation and observed loss avoid old-queue work.
Tests above exercise those distinctions rather than infer target behavior from compilation.

## Earlier implementation checkpoints

The resumed implementation checkpoint is `fe809c7f`. Three integrated Metal lifecycle tests pass at
`d29d2667`: exact two-slot submitted history, retention until old-generation completion delivery, and
failure/loss/extent replacement with exact output against a fresh WGSL twin.
[Actual selected output and executable identity](lifecycle-d29d2667.txt) state their scope.
The coordinator run first stopped on capture accounting before native/twin comparison: its three deliberately
discarded candidates were counted as omissions, while the test expected zero. `342151fc` requires exactly
those three omissions; no pixel metric changes. The corrected run, simulator native background race,
T2 diagnostic captures and clean-tree native/Chrome baseline plus full repository check are queued.
[Continuation and job IDs](../M10-handoff.md) record what remains. No complete item or merge is claimed.

The first implementation is the small [native segment fixture](../../../../crates/thrum-render/tests/native_segment.rs).
It prepares a Metal storage-writing tile kernel and a separate WGSL twin, initializes all textures through
wgpu, then records a wgpu scene producer, one whole raw Metal encoder and a wgpu readback consumer.
The graph orders the three encoders in one queue submission. Textures and pipelines remain owned through
readback completion. Native blits preserve scene/history bits; the native kernel toggles the binary tile.
The fixture now checks eight single-submit frames with distinct ping-pong histories, alternate-submission
prior-history crops and an explicit reset. It requires omitted native commands, omitted storage writes,
omitted reset, stale scene input and stale history selection to fail exact output and history comparisons.
It does not integrate the route or qualify provider lifetime/reset in the frame host yet.

The [graph restrictions](../../../../crates/thrum-render/src/graph/native.rs) reject native writes to transient
or uninitialized resources, attachment operations, and native access to the late-bound platform surface.
Native accesses participate in the existing hazard schedule. Persistent/history outputs do not enter transient
alias allocation. Physical initialization and retention remain the executor's responsibility.

Verified in this session:

- `cargo clippy --locked --offline -p thrum-render --lib --test native_segment -- -D warnings` passes.
- The two `graph::native::` tests pass. The existing 12 `graph::tests::` and two `graph::storage_tests::`
  tests also passed during fixture preparation.
- `./dev check-docs` passes on the graph-ordered checkpoint.
- `./dev check-mobile-core` passes (compile only, including the simulator starter facade).
- The release `v1-game` build with `draw-probe,thrum-render/creation-counters` passes.
- `./dev check-web` passes (Wasm compile and lint only, not browser runtime evidence).

An optional offline tile-shader compile was attempted with macOS SDK 27.2. `xcrun metal` reports the
separate Metal Toolchain component is missing; no shader object was produced and no component was downloaded.
This does not replace or establish the queued runtime Metal compilation/readback proof.

The first device attempt reached the Apple M3 Max Metal adapter, then panicked before encoding because
simultaneous texture HAL guards recursively acquired wgpu's device snatch lock. Failed executable SHA-256:
`4577f13d0ed77e2db1b8470d240118e91cb3c0921f7645d69b235f597d5ccf6a`.
The fix retains each native texture handle separately and drops its HAL guard before taking another
or entering the raw encoder callback. This is a handle-access correction, not an exact-copy/order result.
The native MSL library/compute pipeline and the WGSL twin pipeline were prepared before that failing call;
the runtime compiler therefore worked on this target despite the absent optional offline compiler.
Two additional device tests require locked-wgpu to refuse mixed APIs in both directions, including a no-op
raw callback. Their actual device result is recorded below.

The corrected fixture passed all three tests on the Apple M3 Max at `cdede389`, outside the sandbox under
the shared `gpu` lock. [Selected output and exact scope](fixture-cdede389.txt). This permits route integration;
it does not pass any complete milestone item by itself. The separate integrated lifecycle result above
does not replace route/T2 capture, simulator native background race, repository gate, baseline guard or merge.
An earlier broad `graph::` test filter selected one Metal test in the sandbox, which found no adapter;
the CPU-only filters were then run successfully. That refusal is not a native-boundary failure.

D10.5 needs the owner's unlocked active Mac session and an existing Screen Recording grant for the
OS-window observer. This background session cannot obtain attended approval. Do not automate grants.

The [storage release receipt](m3-expired-bulk-release.json.gz) inventories superseded M3 resume bulk sources
released after M3 acceptance and their through-review expiry. All selected historical evidence and the newer
integrated source set remain. Released raw bytes are not claimed recoverable from hashes. No baseline changed.
