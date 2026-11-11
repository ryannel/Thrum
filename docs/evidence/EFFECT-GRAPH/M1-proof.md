# EFFECT-GRAPH M1 — partial proof, disk blocked

11 November 2026. Implementation: `3d56bf2103dceb4f4c190e73b8484a7c470d8e4e`, clean source at execution.
M1.1–M1.4 pass on this Mac's Metal device. M1.5 remains open; the milestone is not claimed or integrated.

| Item | Saved proof and builder inspection |
| --- | --- |
| M1.1 | [Metal log](M1/graph-kinds.log): all 14 tests pass. `metal_every_texel_every_mip_all_scopes_resize_and_replacement` writes and independently reads all four channels of every texel of all three declared mips, for volumes, cube faces sampled as a cube, and arrays. All three scopes run at extents 4 and 8, on fresh generations 41 and 42; the previous device is destroyed. Transients use the graph allocator; retained scopes use effect-owned allocations rebuilt from descriptors. |
| M1.2 | [Named refusal tests](../../../crates/thrum-render/tests/graph_kinds.rs) separately assert volume layers, multisampling, nonsquare/wrong-face-count cubes, cube storage views, volume attachments, out-of-range mips/layers, and binding-view/mip errors. `transient_reuse_requires_identical_dimension` pins distinct slots for otherwise identical 2D and 3D descriptors, then reuse for an identical 2D descriptor. |
| M1.3 | The Metal capability test refuses unsupported storage format/access, excess bindings and excessive 2D/cube/3D extents. [Validation](../../../crates/thrum-render/src/graph/texture.rs) consumes only the immutable generation record; enabled feature guarantees supply format facts. There is no pass-time device query. [Chrome record](M1/chrome-limits.json): Apple Metal-3, Chrome 157; 4 storage bindings/stage, 8192 2D extent, 2048 3D extent, 256 array layers. The record includes each tested core storage format/access and negative controls. Storage views accept 2D, arrays and 3D, refuse cubes; one mip binds, two are refused. Reproduce with `node scripts/effect_graph_limits.mjs <scratch-output>` under the GPU lock. |
| M1.4 | `existing_scene_variants_keep_compiled_order_storage_and_slots` matches the [retained snapshot](../../../crates/thrum-render/tests/graph_kinds_support/scene-baseline.txt), generated on Metal from pre-M1 `6642b0bcba20a26c2fb93fbaaf3dcd8a77758cc4` with only a pass-name accessor added temporarily. Eight combinations cover day/night, SDR/scene-linear and capture off/on. Names, order, attachment storage and every transient slot match; the fixture asserts that the night star pass is actually selected. |
| M1.5 | The exact named command built release game/cooker and freshly cooked the valley, then refused before capture: [record](M1/native-matrix-refusal.json), `less than 16 GiB free before capture`. About 9.4 GiB was available. Neither native matrix completed; no image-equivalence claim is made and no retained baseline was changed. |

Commands actually run: `cargo test --locked --offline -p thrum-render --test graph_kinds -- --nocapture`
(pinned Rust 1.99.1, `RUSTC_WRAPPER` empty, outside the sandbox under `agent_loop.py lock gpu`), the Chrome
recorder under the same lease, Clippy for all `thrum-render` targets with warnings denied, and `./dev check-docs`.
The clean-checkout registered verifier also [refuses](M1/exact-verify.log) with
`LOW_DISK_SPACE: verifier requires 16 GiB free`, after reclaiming this task's debug build outputs.
`./dev check` [refuses](M1/check.log) the missing exact-baseline proof before its later checks; it does not pass.

Owned build outputs were cleaned at blocked closeout; rebuild before continuing.

Unblock by making at least 16 GiB available **after builds**, with additional rebuild/capture headroom. Then rerun
M1.5, the registered exact verifier, milestone `check`, and `land` under their required locks. Any rebase requires
fresh milestone checks. Successful source evidence here proves correctness and unchanged compilation, not image
identity, frame cadence or performance. M2 and M3 are untouched.
