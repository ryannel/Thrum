# EFFECT-GRAPH M2 review

Reviewed checkout `22376881066df244bc5386e36b0334230c32040e`, whose builder endpoint is `055113576ffc`. The implementation diff reviewed is `33d55bb5..055113576ffc`, including all changed production and test lines, shader source, route, browser runner and plan updates. This is an independent review; no other agents were started and no implementation was changed.

The orchestrator supplied passing `./dev check` and all six automated item commands on the builder's exact files. I read their assertions and implementation rather than rerunning those checks. I rebuilt the release game with `draw-probe,thrum-render/creation-counters` successfully for my own capture. The review does not claim physical display cadence or browser performance.

## Item judgments

| Item | Verdict | Basis |
| --- | --- | --- |
| M2.1 | Pass | `SceneContributions` preserves game order in declaration, upload, compute, shadow, draw and submission callbacks. Local owner IDs are remapped across contributions. The external `v1-game` crate declares the diagnostic entirely through public types. `CookedAssets` retains its callbacks; the no-effect path installs it directly. The unchanged day/night/capture snapshot test pins pass names/order, lifetimes, transient slots and attachment storage. |
| M2.2 | Pass | The literal script seeds at 10, advances to nonempty state at 11, repeats/pauses, discards 12, submits the kept 11, retries 12, changes cadence, captures at another timeline/tick, stalls to 100, goes backward, advances again before timeline reset, and resizes. The oracle dispatches explicit keep/reset cases and implements an independent per-tick recurrence, whereas the shader evaluates a bounded spatial maximum. Every trail/derived/consumer texel and auxiliary kind is compared. The stall from 16 to 100 integrates 8 and checks 76 dropped ticks. Anchor checks cover discard, retry and capture. `EffectState::submitted` is called only after queue submission; abandoned tokens do not commit it. |
| M2.3 | Pass | Each row reads the consumer's actual texture, including run/skip transitions of the trail and derived history and the equal-tick frame after discard. Bind groups for both sides are prepared in rebuild. Runtime cadence changes only booleans and side selection; there is no graph compilation or pipeline creation in that path. The instrumented command checks an unchanged pipeline count. Seed, backward and timeline-reset rows compare every texel, including the full time-sliced volume. Declaration tests refuse transient/imported skipping, loaded attachments, time-sliced history, duplicate pair producers (same and different cadence), owner aliases and early result reads. |
| M2.4 | Pass | The target script requests a replacement device, destroys the old device, publishes generation 2 into the same host, and submits a reset frame. The diagnostic rebuild replaces textures, pipelines, bindings, queue and bookkeeping; exact readback uses the replacement device and checks all reset values plus the published generation. Device preparation starts with an empty effect declaration, then rebuild installs the new generation's declaration. |
| M2.5 | Pass | The Chrome case builds the current default web bundle and invokes the same qualification function on WebGPU. Exact comparisons run in Rust/Wasm and reject failures before returning the browser record. That includes actual device destruction/replacement and every cube face, volume texel and array mip at every scripted row. The supplied successful check is supported by the retained Chrome record, not inferred from native results. |
| M2.6 | Pass | See the prewritten inspection list and capture assessment below. |
| M2.7 | Pass | Independently recomputed all eight named effect medians and sample counts from the retained executor records; all equal the reported costs. Recomputed descriptor storage as 23,840 bytes, including the 128-byte uniform. The no-effect records contain the existing named scene passes and no effect pass. Empty declarations append no resource, pass or copy, and the pinned graph snapshots cover structural identity. Both cost arms record load and capture-on observer conditions. The effect diff does not alter host frame-sample fields. |
| M2.8 | Pass | The unchanged verifier builds native and Wasm products, cooks content, runs both native matrices and all four registered Chrome sets, and rejects content changes during verification. The supplied success covers this source. Retained comparison records report 38 matrix plus four surface comparisons and 481 records per Chrome set. No baseline reference was changed by this milestone. |

## Visual inspection

The [inspection list](M2-review/inspection-list.md) was written before viewing the review capture. It separates source deposition, persistence/fade, cadence transition and ground attachment at the wrap. It calls for whole frames and consecutive native-size crops. This review samples still sequences with their original elapsed-time records; it does not claim to have watched a movie.

Fresh T2 `realtime` capture from the reviewed checkout: Apple M3 Max, Metal, portable recipe, 1920×1080, 963 images over 8.0277 seconds, zero ring/writer omissions. Every lossless source frame was independently decoded and checked against its writer RGBA SHA-256. [Selected-frame identities and timing](M2-review/selected.json), [runtime records](M2-review/runtime.json.gz), [capture manifest](M2-review/manifest.json.gz), and [reviewed source/build](M2-review/reviewed-source.json) retain provenance. The manifest records capture complete with general packaging deferred; the separate selected-frame record records the completed independent hash verification. Bulk source was released after that verification and selection.

| Inspection entry | Observation and evidence | Result |
| --- | --- | --- |
| Whole scene and deposition | [Tick 240](M2-review/frame-0243.png) shows the trail on its checker ground with the cooked terrain, rock and sphere drawn alongside. The diagnostic is deliberately periodic; it is not a sea/cloud visual-quality claim. | Pass |
| Stop and persistence | [12 consecutive native-size crops](M2-review/stop-consecutive.png), ticks 268–274, 2.5025–2.5930 s: the trail remains continuous through source stop at 270. [Whole stop frame](M2-review/frame-0303.png). | Pass |
| Cadence transition | [12 consecutive native-size crops](M2-review/cadence-consecutive.png), ticks 298–304, 3.0029–3.0959 s: no reset, brightness flash or history-side alternation at 300. [Whole frame](M2-review/frame-0361.png). | Pass |
| Wrapping boundary | [12 consecutive native-size crops](M2-review/wrap-consecutive.png), ticks 344–350, 3.7695–3.8620 s: the fading patch remains aligned with the ground checks as the camera pans. Camera X reconstructed from the recorded tick/alpha and route spans 31.7333–33.2923 m; the anchor moves from 0 to 32. No jump at that crossing. [Whole frame](M2-review/frame-0455.png). | Pass |
| Fade completion | The trail is weaker at the wrap than at the stop and is absent by [tick 480](M2-review/frame-0723.png), with the camera held at the route's final position. The exact GPU recurrence supplies the between-frame decay proof. | Pass |

Each crop sheet is left-to-right, then top-to-bottom, at one source pixel per image pixel; no slowed timebase or resized crop was used. I inspected these consecutive stills and whole frames, not movie playback. All 38 selected whole frames are retained beside the sheets, with times in the selection record.

The shared GPU queue delayed capture. Its first preflight was sandbox-refused at `/bin/ps`; automatic approval then allowed the retry, which reached the product and completed. The engine tap worked with the console locked, as permitted by the project's capture policy; no desktop image or physical-panel claim is made.

The relevant edge cases are exercised directly by the scripted GPU proof: repeated ticks, pause, discard/retry, capture on another timeline, long stall, backward tick, timeline change, resize and generation replacement. The realtime route adds distance, camera motion and the wrap transition. Low sun and storm do not alter the unlit integer diagnostic; M2 does not claim the effect's lighting or weather response. Physical full-refresh cadence belongs to its separately attended target proof, not this submitted-content review.

## Evidence audit

[Independent cost audit](M2-review/cost-audit.json). Builder evidence was consulted after reading the design and implementation: [target proof](M2-proof.md), [native state](M2/state-record.txt), [native cadence](M2/cadence-record.txt), [Chrome state](M2/chrome-effect-state.json), [exact verifier](M2/baseline-record.json), and [costs with load](M2/costs.json). The builder explicitly identifies the earlier capture revision and the final integration diff; this review's fresh capture supplies the current-checkout visual check.

## Findings

No Must Fix or Suggestion findings. All eight items pass. Set M2 to `done`; the orchestrator owns counts and clears findings. No implementation, other milestone, commit or push was changed.

PASS M2
