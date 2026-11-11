# APPLE-PRESENTATION M1 — builder evidence

11 November 2026 · **mac-metal, Apple M3 Max** · portable recipe · builder claim, independent review owed.
The [prewritten inspection list](inspection.md) defines the visual checks.

| Item | Evidence and finding |
| --- | --- |
| D1.1 | [Layout test](layout-tests.log): all 32 Halton(2,3) values across three cycles, no (0,0), complete packed bytes and WGSL member offsets. ViewRecord is 240 bytes in the existing 256-byte stride; ViewAux is 144. [RENDER-SPEC](../../../design/RENDER-SPEC.md) agrees; its ledger hashes are refreshed. `PreparedSynthetic::commit_frame` alone advances the submitted camera and phase. Replacement preparation inherits the live submitted view, never its own unsubmitted camera. |
| D1.2 | [Metal attachment readback](motion-tests.log): static orthographic geometry and perspective geometry/sky/stars each produce numerically exact zero at every one of 4096 texels over 35 jitter phases. A genuinely moved instance must produce nonzero motion; spawn/cut must produce the invalid marker. This reads the real Rg16Float attachment after the forward pass. |
| D1.3 | Same log: independent scalar projection versus GPU sky pan at three pixels, maximum error below 0.001 px (required 0.05). Still-camera sidereal-only motion is checked separately; steady sky remains zero. Solar-only changes include 0.01 rad and 0.0001 rad (GPU −0.00585556 px versus independent −0.00585798 px). An encoded but abandoned camera does not become the predecessor. Atmosphere radiance/LUT expressions are unchanged; added shader work only computes motion. Small solar rotations use scalar f64 cross/dot/atan2 once per view to avoid f32 rotation-arc snapping; the GPU record stays f32. |
| D1.4 | Invalid motion is `(65504,65504)`, finite and distinct from stationary zero. Synthetic and contribution draws share `synthetic.wgsl::vertex_at`: identity, generation and previous submitted serial must match. Spawn/reuse fails that test; the first committed pose establishes correspondence. Camera cuts pass reset/serial zero for one sample. Device replacement creates fresh transform history; copying the CPU camera never makes new GPU history valid. Cooked `draw.rs::history` requires `Bound.continued && !frame.history_reset`; newly bound/swapped geometry starts false, and `submitted(history_advanced=true)` establishes the next predecessor. Direct and indirect use the same `cooked_vertex`, history byte 64 and shared fragment. Tentative material replacement resets correspondence too. Cooked volumes have no retained correspondence: occupied rays mark invalid, empty rays discard; their motion write mask is enabled. No forward drawer substitutes zero for invalid geometry. Existing replacement, swap, contribution and direct/indirect regressions run in the repository gate. |
| D1.5 | **I watched motion** as ordered fixed-time submitted-frame sequences, and inspected whole frames and native-size crops. [Day](day/preview-fixed-1x.mp4), [night](night/preview-fixed-1x.mp4), [moving route](moving/preview-fixed-1x.mp4) retain every selected tick at 60 samples/s. Rightward camera pan gives positive history-minus-current X, growing toward horizontal edges; terrain, near props and infinity remain coherent across the skyline without cleared holes. Native star-field crops show continuous infinity motion. During the hold, stationary terrain/sky settle to olive zero while the simulated cube moves right with negative-X motion. [Mover crops](day-hold-mover.png), [skyline](night-skyline.png), [near/far](day-near-far.png), [stars](night-stars-pan.png). Colour references locate the actual scene content: [day](day-ref/frame-0090.png), [night](night-ref/frame-0180.png), [moving](moving-ref/frame-0060.png). |

[checks.json](checks.json) records the required milestone command, including the full `./dev check` gate. These checks establish
correctness and regressions, not visual acceptance or performance. No reconstruction, reactive target, MetalFX,
EDR, governor, panel cadence or elapsed-time motion is claimed.

[Rebase comparison](integration-comparison.json): every one of the 667 motion frames and three colour references is byte-identical before/after the surface-graph integration. The refreshed Metal tests also pass.

[Colour/motion identities](reference-pairs.json) agree exactly except the display selector; exposure words 0–6 are equal.

Capture provenance: each directory's `manifest.json` retains exact binary SHA-256, every Rust/WGSL source hash,
route, cooked root, target/adapter, extent, source identity and original/PNG hashes. The captures were refreshed after rebasing the implementation onto APPLE-TILE’s surface-graph changes; final Rust/WGSL files match their source hashes. Binary SHA-256 is
`e662eb045442bdb6c25dca4c9cfd70b0b0b940781fa0ae2377a2385a745362f1`.
[Capture/package tool](../../../../scripts/apple_presentation_m1.py) uses the existing native submitted engine tap,
checks every submission/tick/phase/history/exposure identity, independently decodes each lossless PNG, then verifies
movie frame count. The first four post-cut ticks are excluded; the moving window ends before the authored cut.
The authored store is cooked from `examples/v1-game/assets/showcase` through the generation-1 path in `run_v1_showcase.py`. The `Motion` route fixture adds one real simulation mover (3 m/s, slot 1499) while preserving ordinary Proof/Off.
Raw duplicates and unselected PNGs are reconstructible scratch; selected native frames, crops, complete movies,
identities, recipes and hashes are retained here. Movies preserve fixed sample timing; no physical display viewing
is inferred. The GPU lock was queued normally throughout.

Cost account: reuse the existing motion target and display pass; +80 view bytes, +16 auxiliary bytes, a 208-byte
sky correspondence record and +80 star-view bytes per frame. Geometry adds four interpolated components; stars add two flat motion components. Matrix deltas are per view; covered fragments evaluate a homogeneous difference and divide, and sky rays project camera/solar correspondence. No new steady-frame
allocation or pass is introduced; debug sampling requires attachment storage. Device cost remains unmeasured.

Integration recheck: the [first land attempt](first-land-attempt.json) stopped at the final draw-probe cooked GPU test; its retained tail did not identify the assertion. The [same pinned binary then passed all 13 cases](cooked-pinned-recheck.log) without code changes. The final land gate runs every test with `RUST_TEST_THREADS=1`; assertions and explicit concurrency witnesses remain enabled.
