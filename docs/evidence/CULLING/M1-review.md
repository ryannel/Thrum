# CULLING M1 review

**PASS — all five items pass.** Reviewed 13 November 2026 at `d4b5109f867d4b69fef1464c196e17ef6debf695`, following builder commit `c6e50cf67956`. The intervening script commit changes only acceptance/state records. This is round 2; counts remain script-owned.

## Must Fix

None. The earlier `mixed_mesh_host_lifecycle_not_exercised` finding (round 1, M1.4) is **resolved**. The new witness exercises the existing host's presenter resize and replacement-device publication, rather than constructing unrelated offscreen targets. Its fresh reviewer run passed and produced visible mixed-field output.

## Item results

| Item | Verdict | Judgement |
| --- | --- | --- |
| M1.1 | pass | Mesh admission permits Occlusion. Every churn fixture frame compares Frustum and Occlusion with bucket-ordered Direct; preparation and counts assert the resolved mode, and seeded frames require hidden slots. Both required wrong-prepass/wrong-list mutants fail exact equality. |
| M1.2 | pass | The script-qualified release command sets `THRUM_FIELD_FULL_PATH=1`; the test submits and compares every tick 1–1,930, including three turns and the first cut. It asserts Occlusion never exceeds Frustum on paired counts and halves the list on at least one tick. This is a real game-host comparison with the authored mixed meshes, not a canned image or sampled-only route. |
| M1.3 | pass | The mesh-face witness exercises depth slack at approximately 400 m and 4 km with two in-plane rolls. Its zero-tolerance mutant loses 6,724 winning pixels. The separate bounded footprint search records the permitted negative result: 8,640 valid poses, 3,796 requesting widening, no coverage candidate outside the unwidened rectangle. |
| M1.4 | pass | Fresh reviewer Metal execution compares all 12 submitted drawable frames across initial state, surface resize and device publication with independently prepared bucket Direct. Resolved modes, current authority, 100,513 active slots and renewed hiding are asserted. [Run log](M1-review/round-2/lifecycle-metal.log). |
| M1.5 | pass | The script-qualified literal release-build/cook/matrix command passes. I inspected the comparison implementation and retained manifests: 38 native and four surface comparisons are exact, with no failures. Chrome qualification is outside this item. [Native manifest](M1-resume/native-m1-manifest.json.gz), [surface manifest](M1-resume/native-m2-manifest.json.gz). |

The supplied script result establishes that `./dev check` and all five checks passed on the builder's files in a clean checkout before Ready for Review. I accepted that result rather than rerunning the complete suite, long field route or baseline matrix. The [retained aggregate report](M1-resume/aggregate-check.json) and its compressed logs corroborate the checks. The focused lifecycle rerun above was for independent product observation and captures of the repaired boundary.

## Reviewer questions and source inspection

**M1.1.** `visibility_buckets.rs` compares every frame through class flips, deactivation/physical compaction, reactivation, reveal and target resize. It checks the actual prepared mode and readback mode; after seeding, Occlusion must draw fewer slots than Frustum. Upload validates logical identity in `ids[0]`; bucket layout emits physical rows in logical-slot order. `count_members` reads history by that logical identity, not the changing list index. The final cull clears the whole next history side and atomically sets surviving logical bits, including shared words across buckets.

The class-specific depth twins and forward pipelines share the module, vertex entry, primitive and depth state. The prepass binds its own list and indirect arguments; `draw_camera` consumes the final list and final arguments. Thus an open backface cannot introduce depth absent from Direct, and new visibility is not restricted to last frame's prepass members. The retained [two-sided-prepass mutation](m1-repair-mutant-two-sided-prepass.log) changes 2,747 pixels; the [final-reads-prepass mutation](m1-repair-mutant-final-reads-prepass.log) changes 32,331. I reconstructed both mutations in memory and matched their mutated and restored source hashes against this checkout. [Independent provenance](M1-review/round-2/mutant-provenance.json). No source was mutated during review.

**M1.3.** `mesh_face_on_its_bounds_keeps_flush_ties_at_rounded_far_poses` uses an actual planar mesh face at local z = −0.731, with a degenerate rear triangle fixing its bounds. It first seeds only the flush occluder, then enables the mesh and verifies that it wins pixels in Direct and retains them in Occlusion. This is not the insufficient cube witness. The [zero-depth-tolerance failure](m1-repair-mutant-zero-depth-tolerance.log) is on this mesh, and its source hashes also match the independent reconstruction. The footprint search is separate: three origins, 32 yaws, five distances, four sizes and five lateral offsets, with a 256×128 pyramid. Its negative result satisfies the explicit alternative in M1.3; it does not prove widening unnecessary or establish the rasterizer interpolation assumption.

**M1.4.** `tests/support/mixed_field_host.rs` imports the game's field, mesh authoring and daylight policy. It retains Frustum and Occlusion hosts while calling `FrameHost::resize_surface([481,271])`, then `device_prepare_inputs().prepare(...)` and `publish_device(...)` on a distinct newer device. Authority advances from tick 1 to 61 between detached preparation and publication. Direct is newly prepared from explicit authored mesh selection/current state for each phase, preventing matching recovery bugs in both paths from making the oracle vacuous.

I traced the production side in `runtime.rs` and `runtime/device.rs`: resize updates the presenter and rebinds slots; preparation reconstructs retained scene inputs; publication refreshes from current authority, swaps device-owned state and resets history. The witness drives those paths, drains submitted samples and reads the real presenter's output texture. This proves the engine lifecycle boundary requested by M1.4. It is not a physical display, OS surface-loss injection or platform callback test.

I read the milestone's changed rendering and test lines, including admission, member history, bucket pipeline/list wiring, scoped mesh-caster position invariance, field-test extraction and persistent failure recorder, plus the new lifecycle fixture and its game imports. I checked the producer/consumer paths above and the exact-matrix scripts. The admission test replaces the obsolete Occlusion refusal with successful mesh-route preparation; equality assertions are not weakened. No successful GPU skip, hard-coded result or separate test-only rendering implementation was found. Interleaved work from other modules is not attributed to CULLING.

## Independent product inspection

The [inspection list](M1-review/round-2/inspection-plan.md) was written before viewing captures. The initial sandbox launch could not enumerate Metal; the approved GPU-access retry passed in 12.66 seconds. [Environment failure](M1-review/round-2/sandbox-attempt.log), [Metal run](M1-review/round-2/lifecycle-metal.log). No owner access intervention was needed.

| Inspection entry | Result |
| --- | --- |
| Initial field has visible daylight geometry, varied mesh silhouettes and cover | pass — rocks, columns, spruces, walls, ground and shadows are visible in the native-size [initial image](M1-review/round-2/captures/before-3-Direct.png). |
| First and settled frames after presenter resize remain complete and equal | pass — the [first resized Occlusion image](M1-review/round-2/captures/resized-0-Occlusion.png) retains the field and cover. All four frames match Direct; Occlusion draws 3,186 versus Frustum's 21,930. |
| First and settled replacement frames show current state, equality and renewed hiding | pass — the [first replacement](M1-review/round-2/captures/replaced-0-Occlusion.png) shows the advanced camera/scene and equals Direct. The [seeded replacement](M1-review/round-2/captures/replaced-3-Occlusion.png) is unchanged in pixels while drawing 1,145 versus Frustum's 22,215. |

I inspected these four stills at native size, not elapsed-time motion. The test compared complete RGBA bytes on all 12 frames; all six retained first/settled RGB triplets also match independently. [Capture hashes and extents](M1-review/round-2/captures.json). The PNGs are lossless encodings of the reviewer's PPM readbacks. Cargo resolved the relative capture destination under the renderer crate; those generated files were moved into this review folder, leaving no changes there.

The scoped edges are slot churn, reveal, near-plane/distance poses, snap turns and a cut (script-qualified tests), plus reviewer-executed nonintegral surface resize, cold replacement history and authority advance. Low-sun/night baseline cases are included in the accepted exact matrix. Storm appearance and full-refresh motion/cadence are not M1 claims; M2/M3 retain device selection, Chrome mixed meshes, defaults, valley and motion qualification. The historical uncaptured field failures remain individually unattributed; the scoped caster invariance and current successful checks do not retrospectively identify their cause.

No new Must Fix or Suggestion was found. M1 is set to `done`; all item verdicts are `pass`. Builder statuses, counts and other milestones are unchanged.

PASS M1
