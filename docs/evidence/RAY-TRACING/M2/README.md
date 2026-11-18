# RAY-TRACING M2 on Metal

The automated builder checks ran at `e43e3b6d253b9e7265c5b7a951f0eed9e710a24b` with Rust 1.100.0 and
wgpu 30.0.1 on a 30-GPU-core Apple M3 Max with 36 GB and macOS 26.6.2. They used Apple tile with
`ray-query` required.

| Item | What the builder checked |
| --- | --- |
| M2.2 | A contribution using the public seam offered two placements of one mesh after its upload. A shadow ray hit the off-camera placement with both bits set and missed the one with only the ambient bit. An empty contribution preserved the ordinary raster and instance list. Cargo metadata confirmed the renderer has no dependency on cooked GPU residency. [Test output](seam.txt) |
| M2.3 | Each submission's raster readback was followed by queries on that submission's scene structure before another frame could replace it. The rays cross separate patches unique to the old and new placement and entity meshes. Both switches changed the raster digest and reversed the corresponding ray answers on the same submission. Discarding the first recording left the previous raster and GPU ray answers held. The repeated recording switched both together. Refusing the coupled oversized row left both readers on the previous publication. [Submission records](edit.json) |
| M2.4 | The oracle used the scene fixture's logical entity slots and the cooked placement recipe. The inactive slot stayed absent. All five opaque instances held mask 3 before and after turning the camera, with Sun maps on and off. An entity and a placement at opposite sides of the scene hit shadow rays even though their boxes were outside the camera frustum and all three fitted Sun pass volumes. [Instance and probe records](casters.json) |
| M2.5 | The first ready frame had the five arena mesh builds and the two distinct cooked placement builds. Two placements sharing a product shared its build. An edit rebuilt one cooked mesh. A fresh device on the same Metal instance rebuilt the five arena meshes and two cooked products and recovered the held raster and ray answers. Steady frames and three advancing world ticks recorded zero mesh rebuilds. [Device-generation records](lifecycle.json) |
| M2.6 | The two valleys held residency at the same time, on the same ray-enabled generation. Only the ray-demand scene added `BLAS_INPUT` to cooked positions and indices and had mesh structures. Their ordinary output matched byte for byte. Its submitted RGBA8 SHA-256 also matched the reference drawn before M2. [Paired result](demand.json) · [Main reference](demand-main.json) |
| M2.7 | Two contended release runs recorded resident allocation, a build and a matched control interval per mesh, and the edited terrain's build interval in its publication switch frame. [Builder check](valley-cost.json) · [Repeat](valley-cost-repeat.json) |
| M2.8 | The exact-image comparisons for both Mac recipes and Chrome are owned by the shared `exact-images` main-branch check. |

To reproduce the main reference, place [the reference test](demand-reference-main.rs.txt) at
`examples/v1-game/tests/ray_demand_reference.rs` in revision `b4321abdc43f9e130f481d881cced926e48f8cb2`.
Set `THRUM_REQUIRE_GPU=1` and `THRUM_RAY_DEMAND_BYTES` to an output path, then run
`rustup run stable cargo test --locked --offline -p v1-game --features draw-probe --test ray_demand_reference`
under the GPU lock. The fixture cooks the portable valley, pauses at noon with the proof population, and
submits the 320 × 180 standard display output. Hash the saved bytes with SHA-256.

## Valley build costs under load

The matched resident-scene delta on Metal was 15,384,576 bytes in both runs. The host held the settled valley
on each arm and prepared the traced view on the ray-demand arm. Allocation was measured before adding timing
resources. The timestamp intervals include a one-instance measurement structure and its consuming query.
Each control repeats those commands on the already-built mesh. Their difference estimates the added GPU work,
with staged copies and the ordinary scene structure build outside the estimate.

| Mesh | Triangles | Build interval, first / repeat | Control interval, first / repeat |
| --- | ---: | ---: | ---: |
| Terrain | 58,880 | 76.246 / 58.505 ms | 0.075 / 0.076 ms |
| Sphere | 2,208 | 0.185 / 0.224 ms | 0.494 / 0.345 ms |
| Rock | 224 | 0.427 / 0.236 ms | 0.831 / 0.188 ms |
| Arena cube | 12 | 0.211 / 0.222 ms | 0.502 / 0.214 ms |
| Arena reversed cube | 12 | 0.224 / 0.241 ms | 0.194 / 0.268 ms |
| Terrain edit | 58,880 | 0.806 / 0.976 ms | 0.068 / 0.071 ms |

The first terrain build tripped on load-time latency and repeated at 58.505 ms with a lower one-minute load
average of 17.83 versus 27.69. The topology-preserving edit estimates were 0.737 and 0.905 ms after subtracting
the controls, both in the frame that published the replacement. The terrain was the first measured build of
each initial frame and the largest mesh, while both edits ran after all measurement pipelines had executed.
These samples establish a repeatable first-use difference and leave the contribution of command scheduling
and pipeline warm-up unresolved.

The small meshes tripped on apparent scaling when the consuming-query controls cost as much as, or more
than, the build intervals. Each pair contains fixed encoder and probe work, and the signed deltas span zero
across repeated shapes. The records retain the raw intervals and the signed subtraction. They give the
observed latency at those triangle counts and do not resolve the smaller meshes' isolated build duration.
