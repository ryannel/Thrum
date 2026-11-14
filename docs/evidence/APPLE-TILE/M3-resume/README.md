# APPLE-TILE M3 current-source requalification

This resumes the existing implementation after the owner's [scope decision](../../../plans/APPLE-TILE/RECORD.json)
moved the attended D3.5 window check to a later milestone. D3.1–D3.4 have fresh target proofs. M3 remains
`in_progress`; the loop owns its final merged/check record and independent review.

The clean proof source is `e41686a0f4164cd164fed403dd0e75c63946937b`, Rust 1.99.1 on the Apple M3 Max.
Native executable SHA-256 is `b8a331bb6646a72ae4b7fc4e6632de0a5eb47c5af269044f9b5d6b96e271c6e3`.
The [registered verifier record](exact/record.json) pins compiler, host, cooked root, native/Wasm artifacts,
reference hashes and commands. Its [content-bound proof](exact/proof.json) passed in 953.73 seconds.
The [capture report](capture-report.json) validates the preserved binary, browser artifacts, actual native recipes
and Metal device. All target commands held the shared GPU lock. This session changes evidence, not engine code.

| Item | Current proof |
| --- | --- |
| D3.1 | [Prepared stage](../../../../crates/thrum-render/src/synthetic/display.rs) binds colour, motion and matching exposure, with extent/generation and graph-read checks. Two [Metal tests](checks/input-tests.log.gz) pass: exact linear/SDR identity rebinding, foreign-read rejection, and a distinct copy-produced reconstruction texture with a copy→tone hazard. Both exposure-side groups are prepared before readiness. |
| D3.2 | The verifier passes 38 full-frame M0 linear/SDR comparisons and four Unorm/sRGB letterbox comparisons under both recipes. Default tone/dither, Rgba8Unorm store quantization and nearest-source presentation remain intact. The [source audit](source-audit.json) explicitly records the upstream optional temporal-diagnostic branch; it does not claim whole-shader text identity. Six fresh native realtime sequences retain complete source accounting. |
| D3.3 | Five [timing tests](checks/timing-tests.log.gz) and eight [optional-layout tests](checks/frame-stats-tests.log.gz) pass. The [consumer/source audit](source-audit.json) traces native, iOS, browser and capture users. The [actual timing audit](timing-audit.json) covers eight native motion records: named maps, separate present values, null transfer/query counters, and 240 star-present/240 absent samples per variants run. CPU/GPU/callback clocks remain separate. |
| D3.4 | All 1,924 Chrome fixed day/night linear/SDR records match their own retained oracle exactly. Fresh realtime day/night runs retain 482 images each; resize retains 480, with zero renderer/export omissions. Resize records generations 1→2→3 and an attributed [1000×800 composed canvas](chrome-resize/presented-canvas.png). The required [moving smoke command](checks/moving-run.json.gz) passes. |

[Visual inspection](inspection.md) covers complete chronological strips, full-size cut/night frames and native detail.
The inherited saturated Chrome night light matches its exact oracle. Original-time previews and selected lossless
sources are retained. Physical cadence, performance, traffic, current OS-window function and physical iPhone
behavior are unmeasured. [Earlier window/simulator evidence](../../render/apple-tile-m3/README.md) remains historical.

One [disk preflight](portable-variants-disk-refusal.log) refused a capture before execution. One
[Chrome export attempt](chrome-day-export-refusal.json) refused two observer omissions; an unchanged-settings retry
completed. The [early check](checks/early-check-stop.json) was deliberately stopped before native tests while
waiting for the GPU. None is counted as a complete pass. Official merge runs the full repository gate on the final
files; the final milestone check runs on merged work.

Reproduce the registered matrix with `python3 scripts/exact_baselines.py verify` under the shared GPU lock.
The retained [capture recipe](capture.py) uses its new verification/output paths and a preserved qualified binary;
copy helpers into owned task scratch before running them. Selected records are in Git; compressed source spools
remain in `tmp/apple-tile-m3-resume/captures` through review, with no remote backup claimed. Historical candidate
cleanup receipts and archived metadata are retained here. No baseline was refreshed.
