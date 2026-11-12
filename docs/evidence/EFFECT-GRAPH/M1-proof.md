# EFFECT-GRAPH M1 — target proof

12 November 2026. Source commit `15ba828710a36e18a25ebfbdb4d80ef16441d881`, clean at registered-verifier execution.
M1 is claimed for review. All five items and the repository gate pass.

| Item | Saved proof and builder inspection |
| --- | --- |
| M1.1 | [Metal log](M1/graph-kinds.log): all 15 tests pass. `metal_every_texel_every_mip_all_scopes_resize_and_replacement` writes and independently reads all four channels of every texel of all three declared mips, for volumes, cube faces sampled as a cube, and arrays. All three scopes run at extents 4 and 8, on fresh generations 41 and 42; the previous device is destroyed. Transients use the graph allocator; retained scopes use effect-owned allocations rebuilt from descriptors. |
| M1.2 | [Named refusal tests](../../../crates/thrum-render/tests/graph_kinds.rs) separately assert volume layers, multisampling, nonsquare/wrong-face-count cubes, cube storage views, volume attachments, out-of-range mips/layers, and explicit binding-view/mip errors. `successive_mip_views_share_one_storage_binding_slot` admits the existing aggregate range used by sequential mip dispatches. `transient_reuse_requires_identical_dimension` pins distinct slots for otherwise identical 2D and 3D descriptors, then reuse for an identical 2D descriptor. |
| M1.3 | The Metal capability test refuses unsupported storage format/access, excess bindings and excessive 2D/cube/3D extents. [Validation](../../../crates/thrum-render/src/graph/texture.rs) consumes only the immutable generation record; enabled feature guarantees supply format facts. There is no pass-time device query. [Chrome record](M1/chrome-limits.json): Apple Metal-3, Chrome 157; 4 storage bindings/stage, 8192 2D extent, 2048 3D extent, 256 array layers. The record includes each tested core storage format/access and negative controls. Storage views accept 2D, arrays and 3D, refuse cubes; one mip binds, two are refused. Reproduce with `node scripts/effect_graph_limits.mjs <scratch-output>` under the GPU lock. |
| M1.4 | `existing_scene_variants_keep_compiled_order_storage_and_slots` matches the [retained snapshot](../../../crates/thrum-render/tests/graph_kinds_support/scene-baseline.txt), generated on Metal from pre-M1 `6642b0bcba20a26c2fb93fbaaf3dcd8a77758cc4` with only a pass-name accessor added temporarily. Eight combinations cover day/night, SDR/scene-linear and capture off/on. Names, order, attachment storage and every transient slot match; the fixture asserts that the night star pass is actually selected. |
| M1.5 | The exact named release/cook/native command passes on this M3 Max under the GPU lock at the claimed source, with a freshly cooked valley. All 38 M1 and four M2 retained comparisons match. Complete source, binary and sample identities are saved in [M1](M1/claimed-native-m1.json.gz) and [M2](M1/claimed-native-m2.json.gz). |

The [milestone check](M1/milestone-check.json) passes all five named commands and `./dev check`.
The [registered verifier record](M1/exact-baselines.json) and [log](M1/exact-verify.log) record both native matrices
and all four Chrome sets passing (481 records each), in 909.15 seconds excluding the GPU-lock wait.
Complete registered native manifests are retained for [M1](M1/registered-native-m1.json.gz) and [M2](M1/registered-native-m2.json.gz).
Covered-content key `6f5cf7001adff625109e2f6d36d9647ef5097ba2734b61ca97a94858d9ef610c` identifies the checked source across subsequent evidence and state commits.
The record includes compiler, host, binary/Wasm hashes, cooked root and retained-reference hashes.

The full gate exposed an aggregate Hi-Z mip range incorrectly treated as one storage view. The correction keeps
successive per-mip views in one binding slot valid and explicit storage views single-mip. All 242 renderer library
tests [pass](M1/render-lib.log), including the eight affected consumers, and all-target [Clippy passes](M1/render-clippy.log).
The Chrome limits record remains the original target record. The earlier [disk refusal](M1/native-matrix-refusal.json)
is retained as history. Exact comparisons qualify image identity; device timing and display cadence are unqualified.
