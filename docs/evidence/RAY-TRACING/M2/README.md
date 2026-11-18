# RAY-TRACING M2 on Metal

The builder checks used Rust 1.100.0 and wgpu 30.0.1 on a 30-GPU-core Apple M3 Max with 36 GB and
macOS 26.6.2, with Apple tile and `ray-query` required. The saved records name the revision tested.

## Fixed valley inspection

The T2 capture submitted the 1280 × 720 SDR output at tick zero on the same hardware and recipe.
The four pairs use the cooked valley and the game's visible proof population with cooked entity rows.
The drawn arm uses standard lighting and Sun maps. The traced arm colors primary-ray hits by instance
identity, with shading from the local surface and distance. [Capture identities, source revision and hashes](stills/capture.json)

| View | Whole drawn frame | Whole traced frame | Native-size crop, drawn on the left |
| --- | --- | --- | --- |
| Valley, level | [Drawn](stills/valley-drawn.png) | [Traced](stills/valley-traced.png) | [Contacts](stills/valley-contact-pair.png) |
| Down to the ground | [Drawn](stills/contact-drawn.png) | [Traced](stills/contact-traced.png) | [Contacts](stills/contact-contact-pair.png) |
| West | [Drawn](stills/west-drawn.png) | [Traced](stills/west-traced.png) | [Contacts](stills/west-contact-pair.png) |
| East | [Drawn](stills/east-drawn.png) | [Traced](stills/east-traced.png) | [Contacts](stills/east-contact-pair.png) |

- The foreground terrain covers the same part of each frame. The far ridge peaks and intervening slopes
  line up in the level and turned views. The downward view retains the foreground up to the bottom edge.
- The large sphere and the rock have matching contours and positions in each crop. Their lower edges
  meet the same part of the terrain. The sphere's raster reflection and the ground shadows change the
  apparent brightness near those edges.
- The eleven smaller entities between the sphere and the rock keep the same order, screen positions
  and partial overlaps. The terrain is green, the sphere is pink and the rock is blue in all four traced
  views. The entities retain their individual colors as the camera turns. The traced view has a narrow
  shading fringe at some facet and silhouette edges. All four drawn RGBA hashes matched the prior evidence.

The coplanar Metal fixture offered two adjacent copies of one plane at the same depth. The pixels on
each side of the shared edge retained each placement's color with no sky gap. Moving the camera back
while keeping the planes at the same screen positions darkened both colors. [Submitted RGBA samples](seam.txt)

Reproduce with the stock release V1 game and a cooked store made from `examples/v1-game/assets/showcase`.
Run `python3 scripts/ray_tracing_proof.py stills --binary target/release/v1-game --store <store> --output <new-directory>`
under the GPU lock. The script requires ray queries, selects Apple tile and records fixed T2 checkpoints.
For the retained crops, take the rectangle in each pair's `crop` record from both images and place the
two rectangles side by side without resampling.

## Automated checks

| Item | What the builder checked |
| --- | --- |
| M2.2 | A contribution using the public seam offered two placements of one mesh after its upload. A shadow ray hit the off-camera placement with both bits set and missed the one with only the ambient bit. An empty contribution preserved the ordinary raster and instance list. Cargo metadata confirmed the renderer has no dependency on cooked GPU residency. [Test output](seam.txt) |
| M2.3 | Each submission's raster readback was followed by queries on that submission's scene structure before another frame could replace it. The rays cross separate patches unique to the old and new placement and entity meshes. Both switches changed the raster digest and reversed the corresponding ray answers on the same submission. Discarding the first recording left the previous raster and GPU ray answers held. The repeated recording switched both together. Refusing the coupled oversized row left both readers on the previous publication. [Submission records](edit.json) |
| M2.4 | The oracle used the scene fixture's logical entity slots and the cooked placement recipe. The inactive slot stayed absent. All five opaque instances held mask 3 before and after turning the camera, with Sun maps on and off. An entity and a placement at opposite sides of the scene hit shadow rays even though their boxes were outside the camera frustum and all three fitted Sun pass volumes. [Instance and probe records](casters.json) |
| M2.5 | Observed five arena builds before recording any frame, on first load and after device replacement. The replacement prepared the edited 128-triangle entity row from its committed geometry. In both generations the two distinct cooked meshes built during staging and completed before their first publication. Each placement or row edit recorded one build. Recovery preserved raster bytes and ray answers. Steady frames, advancing world ticks and a same-device visual replacement recorded zero mesh builds. [Device-generation records](lifecycle.json) · [Visual preparation test](seam.txt) |
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
| Arena cube | 12 | 138.727 / 156.540 ms | 0.062 / 0.082 ms |
| Arena reversed cube | 12 | 0.059 / 0.189 ms | 0.057 / 0.090 ms |
| Terrain | 58,880 | 1.102 / 1.002 ms | 0.066 / 0.064 ms |
| Sphere | 2,208 | 0.372 / 0.339 ms | 0.125 / 0.061 ms |
| Rock | 224 | 0.214 / 0.249 ms | 0.163 / 0.145 ms |
| Terrain edit | 58,880 | 0.800 / 1.124 ms | 0.067 / 0.064 ms |

After moving arena builds into preparation, the cold outlier moved from terrain to the first arena
cube. It repeated at 156.540 ms under a one-minute load average of 29.86, versus 138.727 ms at 29.62.
The next cube has the same triangle count and took 0.059–0.189 ms. Terrain, previously the first measured
mesh, took 1.002–1.102 ms. The repeat and the changed order associate the outlier with the first
observed build rather than triangle count. The records do not isolate scheduling, driver work or probe
warm-up. Submission 0 in the records is the arena's preparation submission before any ordinary frame.

The small-mesh controls still account for a substantial share of their intervals, so the signed
differences remain sensitive to fixed probe work and scheduling. The topology-preserving terrain edit
added an estimated 0.733–1.061 ms after subtracting the controls, in the same frame as its publication.
