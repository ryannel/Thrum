# APPLE-PRESENTATION M2 review

Reviewed 13 November 2026 at `5b6c8aaaef0678966369373ef83855f3ef33a2f4`; builder endpoint `15eb1a372b66`. **FAIL: D2.6 lacks the required Mac production-pass volume witness.** The other six items pass. This is a verified proof-coverage defect, not an observed incorrect volume value in the shipped pipeline.

## Must Fix

### mac-volume-witness-bypasses-forward-pass — D2.6 (verified)

D2.6 explicitly asks for W1–W5 through the real forward and star passes. Its W3 proof instead calls a custom pipeline in `crates/thrum-asset-gpu/src/draw/tests.rs:41`. That pipeline uses `fixture_vertex`, no depth attachment, unblended Rgba32Float colour and a manually attached R8Unorm texture (`:385–424`), drawing a single fullscreen triangle into a 1×1 target. It splices the production fragment into a substitute shader. The alpha/byte-161 assertion at `:359–375` is useful shader evidence, but it never exercises `Pipelines::draw`, the production volume pipeline, or the graph's forward attachment.

The production path differs materially: `crates/thrum-asset-gpu/src/draw.rs:277–324` uses `cooked_volume_vertex`, front-face culling, reversed-Z depth and premultiplied colour blending; `:713–717` draws the volume through the contribution. The Mac tests in `crates/thrum-render/src/synthetic/temporal/tests.rs` exercise the real forward/star graph for W1, W2, W4 and W5, but attach no cooked contribution and contain no W3 row.

Reproduce by tracing the test named `metal_cooked_march_uses_each_world_sample_and_preserves_sky` from its pipeline creation to its reactive readback, then compare with the production sites above. No production mutation was made. The passing test command cannot establish the missing composition: a wrong reactive blend in the production volume pipeline would not affect this custom-pipeline alpha assertion. Chrome's W3 only requires a partial value and uncovered zero, as its target-specific design permits, so it cannot replace the Mac alpha-equality proof.

To close this finding, drive the shared cooked-volume fixture through the production Mac contribution/forward path, read that frame's actual reactive attachment, and assert the expected alpha and uncovered zero. Keep the existing fragment test as supplementary evidence. This is required by the existing item; no new feature or planner-field change is requested.

## Item judgements

| Item | Verdict | Evidence and answer to the reviewer question |
| --- | --- | --- |
| D2.1 | pass | `scene_graph.rs` adds a sampled depth/reactive consumer only with temporal inputs. The storage report names roles and reports actual native storage. Existing motion already has the display reader and remains stored; disabled daytime depth remains eligible for Memoryless, with the existing night star reader retaining depth. The new reactive target is discard-only without a reader. The always-resolved 1×1 exposure is the separately required D2.3 output. The Metal test checks texture usage, Memoryless/Private and unchanged ordinary colour. |
| D2.2 | pass | Read synthetic, sky, stars, cooked mesh/material and volume pipeline declarations alongside forward attachments. All declare the third R8Unorm target. `SceneSurface.color_formats` carries it through the contribution seam. Both cooked direct and indirect draws use the same mesh pipeline; selection and per-object history logic remain intact. The star graph declares colour/motion loads and reactive load when consumed, and orders the temporal reader after stars. Its pass attaches all three outputs. Pipeline/graph checks passed on the supplied source. |
| D2.3 | pass | `temporal.rs` creates and resolves a 1×1 R16Float texture every frame from the GPU exposure sample. Exposure buffers use the submitted history sides. The modified exposure test changes p, checks the retained prior sample and uses the shader's rescaling function with both GPU samples; it rejects unscaled reinterpretation. This proves input retention/conversion, not the future M3 colour-history reconstructor. |
| D2.4 | pass | `runtime/tests/device/view_history.rs` separately asserts held B reading A before completion, two pending submissions, prior matrices, cancelled acquisition retaining identity/matrices, forced pre-submit refusal and background cancellation resetting the next predecessor. Cancellation/background events are printed. Output-grid key retention and an abandoned key change are asserted in `synthetic/temporal/tests.rs`; the host supplies the presenter extent in `view_with_history_reset`. Queue submission precedes `commit_frame`; healthy `skip` still flushes staging without committing a scene. iOS revoked-lease paths call `cancel_background`. These are Mac host/function proofs, not physical-iPhone qualification. |
| D2.5 | pass | Independently rebuilt and ran the temporal checker in hardware Chrome. T2 images come from submitted frames, and identities come from the transform-history state used in packing. The injected `getCurrentTexture` failure follows scene encoding; C shares the abandoned pose. A=38, C=39, C predecessor=38; one cancelled reservation; 54,687 valid geometry pixels have nonzero motion. An inert target fails W1 and early history advancement fails W6/W7. W1→W2 and phase-edit→steady rows mark then clear. |
| D2.6 | fail | Real graph readbacks establish invalid/valid geometry, sky and star marking/clearing, plus uncovered zero in the geometry fixture. The cooked-alpha test reads real GPU bytes but bypasses the required production forward path. See Must Fix above. |
| D2.7 | pass | Accept the script's successful `--case all` result on the exact builder source. Read the checker changes: existing cases remain, temporal is added, controls use the declared slider step, and the default web root is freshly built. No regression case is removed or weakened to hide a temporal failure. This establishes regression coverage only. |

## Review scope and execution

Read the M2 design and criteria before the builder's evidence/log. Reviewed the implementation diff at `90ed21a0`, the browser-build repair at `7f1bb6cc`, and the final evidence commits through `15eb1a372b66`. Read the changed production code, shaders, tests, graph snapshot and contract changes, and followed callers across the contribution, submission, exposure and iOS seams. The final builder commit adds an exact-baseline record, not implementation. No prior M2 review was supplied. Project and skill graded examples were consulted; there is no plans glossary.

The user reports that `./dev check` and all seven item commands passed in the script's clean checkout on these files. Those results are accepted; the broad gate was not rerun. The additional Chrome run addressed uncertainty about the witness's actual frame path. It used:

```text
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.1.0/skills/orchestration/build/scripts/agent_loop.py background -- node scripts/check_v1_browser.mjs --case temporal --output tmp/apple-presentation-m2-review-unsandboxed
```

Waited with the script's `wait` command. The checker requires scratch output under `tmp/`; selected results were copied into this review's evidence folder. An initial attempt to write directly to evidence was refused by that path guard. Sandbox Chrome startup then failed; an approved execution outside the sandbox passed. No access blocker remains.

Reviewer evidence: [successful command log](M2-review-1/chrome.log), [run identity](M2-review-1/run.json), [hardware GPU record](M2-review-1/browser-gpu.json), [submitted-frame witness and all row values](M2-review-1/temporal.json), and [reviewer C diagnostic screenshot](M2-review-1/temporal-C.png). Loaded Wasm SHA-256: `b554141e2e59870dc98cab969c03fb5d47d37e9807a569f58ae30c9f1596da93`.

M2 has no `inspect` proof and explicitly claims correctness only. The review focused on attachment writers/readers, retention under cancellation and phase edits, and disabled-path equivalence. I viewed the C diagnostic at its 640×360 native size: blue motion channel with a reactive volume region. This screenshot corroborates the record; it does not prove alpha equality or display cadence. I did not watch a motion recording. The executed Chrome edges were invalid→valid history, volume coverage, Sun/star edit→steady, and late failed acquisition during a camera change. Low sun/storm/distance/full-refresh visual qualification and iOS rotation are not claimed by this input-only milestone and were not run. No performance or physical-device verdict is inferred.

No Must Fix outside the items. No additional suggestions.

FAIL M2: 1 fail, 0 could_not_judge, 0 outside the items
