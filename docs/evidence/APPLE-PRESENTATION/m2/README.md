# APPLE-PRESENTATION M2 builder evidence

Clean-checkout follow-up at `8224cda1961a0d04a308c689400fca575695be8d`: all seven item commands and
the repository gate pass with the checker's `--timeout 3600`. Its default 600-second run timed out
during the existing every-tick field oracle without an assertion failure.
[Checker result](clean-checkout-check.json.gz), [repository output](clean-checkout-repository.stdout.log.gz)
and [compiler output](clean-checkout-repository.stderr.log.gz).

The default browser checker now builds the current web bundle before resolving its directory, with
an explicit local Cargo target directory; explicit web roots remain caller-owned. This removes the
clean verifier's dependency on ignored output left by an earlier session. Chrome passes W1–W7 and
all 32 regression cases. The loaded Wasm hash, A=38, C=39, predecessor=38 and 54,687 motion pixels
match the retained witness below. The submitted diagnostic was inspected and is byte-identical to
the retained PNG (SHA-256 `6aa2e933fd4260650d422b4fc15b213327bdd9488ddc061f8745d68526c9e8a2`).
[Temporal record](clean-checkout-temporal.json.gz), [temporal output](clean-checkout-temporal.stdout.log.gz),
[regression run](clean-checkout-all-run.json.gz), [regression output](clean-checkout-all.stdout.log.gz)
and [hardware](clean-checkout-all-gpu.json.gz).

The reported watcher missed edit was not reproduced: 40 runs in four concurrent workers pass outside
the sandbox on the unchanged cooker, as does a run with TMPDIR unset and the complete repository gate.
An additional 30 preliminary sequential runs passed. No watcher code or assertions were changed;
the original missed event's cause remains unconfirmed. [Concurrent stress output](clean-checkout-watcher.log.gz).

Exact verification on the repair commit passes all 1,966 comparisons in 959.18 seconds under the
GPU lock, with references unchanged. [Exact record](clean-checkout-exact.json) and
[verifier output](clean-checkout-exact.log.gz). M2 remains `in_progress` for the lead's checks and review.

The original implementation qualification follows.
The checker started at `c506b2a6` and ended at `d35553c2`. The intervening changes contain evidence only,
and the covered source objects are identical. [Required checker](milestone-check-current.json.gz) and
[verification identity](verification-identity.json).

| Item | Target proof and builder inspection |
| --- | --- |
| D2.1 | Metal reports depth Memoryless without a reader and Private with temporal sampling. Motion has its existing display reader. The new reactive attachment is eligible for transient storage when disabled. Enabling stored inputs preserves ordinary colour byte for byte. [Focused attachment proof](metal-inputs.log.gz). |
| D2.2 | Synthetic, contribution/material, cooked direct/indirect, volume, sky and separate stars declare R8Unorm reactive. Contributions receive all three formats through the public seam. Star graph accesses and attachments cover each shader output. Current Metal pipeline, graph, culling and cooked-draw tests pass. [Graph suite](graph-kinds-current.log.gz). |
| D2.3 | Each frame resolves a real 1×1 R16Float exposure texture. Submitted history retains its GPU storage scale. Changing p from 0.0043963287 to 0.008792657 gives 0.017578125 in the radiance-2 conversion control, against the old stored value 0.008792657. [Focused exposure proof](metal-exposure.log.gz). |
| D2.4 | The real frame host separately asserts two in-flight submissions, held submission, submitted previous matrices, failed-acquisition cancellation, forced submission refusal and background reset. The cancellation and background cases each count one event. Output-grid keys remain tentative until submission. The healthy presentation-drop path retains its existing outcome. [Focused history proof](metal-history.log.gz). |
| D2.5 | Chrome passes W1–W7 through the game's submitted frame path. After one injected acquisition refusal, A=38, C=39, and C's predecessor=38. C writes nonzero motion on 54,687 valid geometry pixels. Values come from T2's submitted diagnostic and the history cursor used for matrix packing. An unwritten reactive target fails W1, and advancing history on the abandoned view fails W6. [Current temporal witness](chrome-temporal-temporal.json.gz). |
| D2.6 | Metal directly reads the reactive attachment. Invalid geometry writes 255, valid geometry 0, cooked volume alpha 0.63212055 becomes byte 161, and Sun and independently edited stars mark then clear. Uncovered pixels are 0. Both targets read one fixture. [Attachment proof](metal-inputs.log.gz) and [cooked volume proof](metal-volume.log.gz). |
| D2.7 | All 32 hardware Chrome regression cases pass, including controls, recovery, valley, field, route and the 16-view exact field oracle. [Current run](chrome-all-run.json.gz) and [hardware](chrome-all-browser-gpu.json.gz). |

The focused Metal logs retain the original numerical witnesses. The current required checker reruns their
assertions, the complete renderer and game suites, Clippy and Wasm checks. The iOS simulator shell also passed
its earlier strict compile check. [Simulator compile](ios-simulator-clippy.log.gz).

The current Chrome witness records 39 consecutive submitted frames and exactly one cancelled reservation.
Its loaded Wasm is `b554141e2e59870dc98cab969c03fb5d47d37e9807a569f58ae30c9f1596da93`. The submitted C diagnostic was inspected at native extent and shows the pan's motion
channel on valid geometry. [Submitted diagnostic](chrome-temporal-C.png), [run identity](chrome-temporal-run.json.gz)
and [shared fixture](../../../../crates/thrum-render/src/synthetic/temporal/witness.json).

Exact verification at `c506b2a6` passes 42 native comparisons and four Chrome sets of 481 records,
for 1,966 exact matches in 895.37 seconds under the GPU lock. Its covered-content key is
`ed86252e47704218c70c5f0951a2aceb6d6c917cb4fd51db6d73dd7074375faa`. Binary, Wasm, cooked root, compiler, reference hashes and comparison records
are retained. These establish image equivalence and correctness. Device performance qualification remains owed.
[Exact record](exact-baselines.json) and [verifier log](exact-baselines-final.log.gz).

The main merge preserves CULLING's shared `field_fly` tests and M2's temporal dependencies. The temporal graph
now supplies main's explicit texture-kind and view fields. EFFECT-GRAPH's scene snapshot was updated for M2's
reactive attachment, exposure texture, lifetimes and eight transient slots. Its full assertion passes all 15 cases.
[Reviewed snapshot differences](graph-snapshot-diff.txt.gz) and [merge validation](merge-validation.json).
