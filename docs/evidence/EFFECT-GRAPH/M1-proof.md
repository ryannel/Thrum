# EFFECT-GRAPH M1 — target proof

12 November 2026. Rebased source: `5455a4d0881ccb7a8c092b75287cf9da18cf72ad`, clean at exact-verifier execution.
M1.1–M1.5 have target proof. The required milestone check is running before claim and integration.

| Item | Saved proof and builder inspection |
| --- | --- |
| M1.1 | [Metal log](M1/graph-kinds.log): all 14 tests pass. `metal_every_texel_every_mip_all_scopes_resize_and_replacement` writes and independently reads all four channels of every texel of all three declared mips, for volumes, cube faces sampled as a cube, and arrays. All three scopes run at extents 4 and 8, on fresh generations 41 and 42; the previous device is destroyed. Transients use the graph allocator; retained scopes use effect-owned allocations rebuilt from descriptors. |
| M1.2 | [Named refusal tests](../../../crates/thrum-render/tests/graph_kinds.rs) separately assert volume layers, multisampling, nonsquare/wrong-face-count cubes, cube storage views, volume attachments, out-of-range mips/layers, and binding-view/mip errors. `transient_reuse_requires_identical_dimension` pins distinct slots for otherwise identical 2D and 3D descriptors, then reuse for an identical 2D descriptor. |
| M1.3 | The Metal capability test refuses unsupported storage format/access, excess bindings and excessive 2D/cube/3D extents. [Validation](../../../crates/thrum-render/src/graph/texture.rs) consumes only the immutable generation record; enabled feature guarantees supply format facts. There is no pass-time device query. [Chrome record](M1/chrome-limits.json): Apple Metal-3, Chrome 157; 4 storage bindings/stage, 8192 2D extent, 2048 3D extent, 256 array layers. The record includes each tested core storage format/access and negative controls. Storage views accept 2D, arrays and 3D, refuse cubes; one mip binds, two are refused. Reproduce with `node scripts/effect_graph_limits.mjs <scratch-output>` under the GPU lock. |
| M1.4 | `existing_scene_variants_keep_compiled_order_storage_and_slots` matches the [retained snapshot](../../../crates/thrum-render/tests/graph_kinds_support/scene-baseline.txt), generated on Metal from pre-M1 `6642b0bcba20a26c2fb93fbaaf3dcd8a77758cc4` with only a pass-name accessor added temporarily. Eight combinations cover day/night, SDR/scene-linear and capture off/on. Names, order, attachment storage and every transient slot match; the fixture asserts that the night star pass is actually selected. |
| M1.5 | Both release native matrices pass on this M3 Max under the GPU lock, with a freshly cooked valley. Retained images match in all 38 M1 and four M2 comparisons. Saved complete manifests include source, binary and sample identities: [M1](M1/registered-native-m1.json.gz), [M2](M1/registered-native-m2.json.gz). |

The [registered verifier record](M1/exact-baselines.json) and [log](M1/exact-verify.log) record both native matrices
and all four Chrome baseline sets passing (481 records each), in 953.34 seconds excluding the GPU-lock wait.
Covered-content key: `878336a3f5a4ebbea6458d322bacfc52017946f85f458957594fbb4f06809cda`.
The record includes the compiler, host, native binary and Wasm hashes, cooked root and retained-reference hashes.

M1.1–M1.4's saved Metal log and Chrome limits record came from the implementation checkpoint before rebasing.
The same witnesses compiled after rebasing; the required milestone check now reruns their commands, M1.5's full
named release/cook/native command and `./dev check`. Claim and integration await that result.

The earlier [disk refusal](M1/native-matrix-refusal.json) remains historical evidence. Current verification ran
with sufficient disk. Exact comparisons qualify image identity; device timing and display cadence are unqualified.
