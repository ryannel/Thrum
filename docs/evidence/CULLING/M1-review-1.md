# CULLING M1 review

**FAIL — four items pass; M1.4 fails its required lifecycle proof.** No Must Fix lies outside the items. No previous M1 review was present.

Reviewed 12 November 2026 at `9cba741733015cc6608322bc505a70a0b138ed0f`, the supplied clean checkout after the plan migration. Builder claim: `4c2690bb7e0f`. The migration changes no renderer, game or script source; it is not charged to the builder. [Build identity](M1-review-1/identity.json).

## Must Fix

**MF1 — Mixed-mesh host lifecycle is not exercised (M1.4; verified coverage defect).**

Run `cargo test --locked --offline -p thrum-render --test visibility_buckets`. All eight tests pass, but the lifecycle witness does not perform the surface resize and device replacement its item asks for. In `crates/thrum-render/tests/visibility_buckets.rs:128`, `Sequence::resize` constructs another `OffscreenTarget`; there is no surface presenter. At line 218 the test loops over generations 61 and 62, requesting a fresh GPU and preparing three new sequences each time. It does not replace the device of the running host. The fixture is 899 cubes and one open plane, rather than the game's mixed-mesh field.

This establishes target reallocation and fresh-device preparation, but can pass even if the host loses the selected meshes, culling mode or current scene during recovery. The actual boundaries are `FrameHost::resize_surface` (`crates/thrum-render/src/runtime.rs:806`) and device preparation/publication (`crates/thrum-render/src/runtime/device.rs:157` and `:189`). The existing runtime test `culled_modes_survive_surface_resize_and_device_replacement` uses `SyntheticScene::grid(1)` and has no hidden-slot assertion, so it does not supply the missing mixed-mesh proof either.

M1.4 needs a witness in its named test target that exercises the running host's surface resize and replacement-device publication with mixed meshes and hidden slots, then checks the next frames against bucket-ordered Direct and verifies resolved Occlusion and renewed hiding. This finding establishes missing proof, not an observed rendering failure. [Reviewer test output](M1-review-1/visibility-buckets-metal.log).

## Item results

| Item | Verdict | Independent result |
| --- | --- | --- |
| M1.1 | pass | All eight bucket tests pass. Every churn/reveal/resize fixture frame compares Frustum and Occlusion with bucket Direct. Preparation and readback assert the resolved mode; frames after history seeding require Occlusion to hide slots. Required mutant evidence matches the current source. |
| M1.2 | pass | Fresh release run compares all 1,930 ticks exactly for both culled modes, with zero missing counts. Occlusion never exceeds Frustum and halves its list on 1,918 ticks. Frustum reaches 21,737 drawn out of 100,513 active. [Log](M1-review-1/field-occlusion.log). |
| M1.3 | pass | All four mesh-tie poses keep their 6,724–6,745 winning pixels. The zero-tolerance mutant fails on this mesh witness. The independent footprint search repeats the permitted negative result: 8,640 valid poses, 3,796 requesting widening, zero coverage candidates outside the unwidened rectangle. |
| M1.4 | fail | Automated command passes, but its reviewer question is not satisfied: see MF1. |
| M1.5 | pass | The literal pinned release-build/cook/matrix command exits 0. All 38 native and four surface-format comparisons match retained bytes; both scripts complete their motion/control arms. [Command](M1-review-1/run-exact-matrix.sh), [log](M1-review-1/exact-matrix.log), [native manifest](M1-review-1/native-matrix.json.gz), [surface manifest](M1-review-1/surface-matrix.json.gz). Chrome baseline qualification is not claimed by this item. |

## Reviewer questions and code inspection

**M1.1.** The upload validates `ids[0]` as logical identity before physical packing. Bucket layout enumerates logical slots and stores their current physical rows. The member shader looks up history through `instances[slot].ids.x`; the final phase clears the entire next history buffer and atomically ORs surviving logical-slot bits. Member inputs reach both culls. The prepass binds its own list and arguments and uses depth-only twins of the ordinary, mirrored and two-sided class pipelines. The final forward draw binds the final list and arguments. The common pipeline factory preserves vertex entry, primitive and depth state, so the prepass cannot introduce a backface the forward pass rejects.

The retained current-source mutants fail with 2,747 changed pixels for the two-sided prepass and 32,331 for the final draw reading the prepass list. I read their logs and reconstructed each mutation in memory: both the mutated and restored SHA-256 values match the current source and the builder's fresh record. [Independent provenance check](M1-review-1/mutant-provenance.json), [two-sided failure](m1-repair-mutant-two-sided-prepass.log), [wrong-list failure](m1-repair-mutant-final-reads-prepass.log). These are inspected builder mutation runs, not reviewer reruns; production source was not edited.

**M1.3.** `mesh_face_on_its_bounds_keeps_flush_ties_at_rounded_far_poses` uses a planar mesh front at local z = −0.731, a referenced degenerate rear triangle to fix its bounds, and a flush larger occluder. The front wins tied pixels in Direct. Origins near 400 m and 4 km, each with rolls 0 and 0.3 radians, pass. The retained current-source zero-tolerance mutation loses all 6,724 winning pixels at the first pose; its reconstructed hash also matches. [Failure](m1-repair-mutant-zero-depth-tolerance.log).

The widening search is separate: three origins, 32 yaws, five distances, four sizes and five lateral offsets, at a 256×128 pyramid extent. It tests transformed vertices against the unwidened projected bounds. Its bounded absence is allowed by M1.3; it neither proves widening unnecessary nor proves the rasterizer interpolation assumption. [Fresh search and tie output](M1-review-1/visibility-buckets-metal.log).

**M1.4.** The test genuinely allocates new Metal devices and compares both resized and newly prepared targets. It does not call the host lifecycle APIs. Those are separate boundaries that retain source state, rebind presenter resources, refresh instances and reset history; fresh construction alone does not test that composition.

I read the milestone's changed production and test lines, including mesh admission, culling glue, bucket pipelines, shader history, the field-test extraction and persistent failure recorder, plus the scoped shadow-position invariant and retained diagnostic sources. I traced upload, bucket order, the pipeline factory, final-list consumption, the field capture caller and runtime lifecycle consumers. No weakened equality, successful GPU skip, canned image or test-only rendering path was found. Unrelated EFFECT-GRAPH changes interleaved in main are not attributed to CULLING.

## Execution and visual scope

The first sandbox attempt could not enumerate Metal; the approved GPU-access retry passed. GPU commands are serialized with the repository `gpu` lock. The required bucket command was run once for its three identical item checks, adding only `--nocapture`. [Initial environment failure](M1-review-1/visibility-buckets.log), [Metal pass](M1-review-1/visibility-buckets-metal.log).

The full field command adds only `--nocapture` and evidence destinations to the required environment/command. These images are fixed-time endpoint readbacks through `FrameHost::capture_pixels`, not elapsed-time motion or presented-frame cadence. The checklist was written before opening captures: [inspection plan](M1-review-1/inspection-plan.md). First-turn native-size inspection shows the mixed props, walls and shadows without obvious holes: [tick 421](M1-review-1/field-turn-421.png). I also inspected the native-size [second turn](M1-review-1/field-turn-1021.png), and three-frame strips at [420–422](M1-review-1/first-turn-strip.png), [1620–1622](M1-review-1/third-turn-strip.png), and [1800–1802](M1-review-1/first-cut-strip.png), left to right. Props, wall silhouettes and screen edges show no obvious missing geometry; the abrupt camera changes occur at the route’s turn/cut events. These are sampled stills, not watched real-time motion. All 567 retained event-frame pairs were independently byte-equal between Frustum and Occlusion before compacting the evidence to these selected PNGs and a [capture hash manifest](M1-review-1/field-capture-manifest.json). The test compared Direct at every one of the 1,930 ticks, not just these selected captures.

Distance, near-plane crossings, snap turns, the first cut, mirrored props, class flips, deactivation, compaction, reactivation and reveal are covered by the scoped checks. M1's byte-equality bar is not a full-refresh performance qualification. Chrome mixed meshes, simulator fallback, valley defaults and elapsed-time visual judgement remain M2/M3 work.

The builder's two earlier intermittent failures are retained and individually unattributed. The final mesh-caster `@invariant` closes a real arithmetic-guarantee gap, while leaving the legacy flat caster unchanged; it is not retrospective proof of either old failure's cause. The current every-tick run and unchanged-baseline checks judge the current claim.

The independently generated [native variants contact sheet](M1-review-1/native-variants-contact.png) was inspected as downscaled samples, and the [sunset still](M1-review-1/native-sunset.png) at 1920×1080. Both contain the intended scene output; no motion-quality conclusion is drawn from the contact sheet. Native retained-image verification passes all 38 comparisons and all of its capture arms: [complete manifest](M1-review-1/native-matrix.json.gz).

Documentation checks pass both before and after writing the review: [initial log](M1-review-1/check-docs.log), [final log](M1-review-1/final-check-docs.log). [Command result summary](M1-review-1/check-results.json). `./dev check` also exits 0, including the renderer, field, Occlusion, simulation/replay, doc-test and cooked-scene regressions. [Full log](M1-review-1/dev-check.log). These broader passes do not fill the specific mixed-mesh lifecycle witness missing from M1.4.

No production code, builder status, counts, other milestones or retained references were changed. M1 returns to `in_progress` with the single finding above.

FAIL M1: 1 fail, 0 could_not_judge, 0 outside the items
