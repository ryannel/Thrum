# APPLE-TILE M10 builder record

14 November 2026. Initial boundary checkpoint: `1d21de31` (on top of `d9ca8c13`).
This is an incomplete builder record, not milestone acceptance. All five items remain open.

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

The exact Metal fixture is queued outside the sandbox under the shared `gpu` lock. No device result,
route/T2 capture, simulator native background race, full repository gate, baseline guard or merge is claimed.
An earlier broad `graph::` test filter selected one Metal test in the sandbox, which found no adapter;
the CPU-only filters were then run successfully. That refusal is not a native-boundary failure.

D10.5 needs the owner's unlocked active Mac session and an existing Screen Recording grant for the
OS-window observer. This background session cannot obtain attended approval. Do not automate grants.
