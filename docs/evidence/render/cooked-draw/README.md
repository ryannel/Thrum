# COOKED-DRAW M1 evidence

9 November 2026. M1 is claimed with all twelve items passing. Plain `./dev check` passes 666 Rust tests and the
launcher regression cases. The launcher defaults to an empty compiler wrapper and preserves explicitly supplied
`RUSTC_WRAPPER` values. Earlier checks used a caller override. Same-load tripwires pass. Builder inspection and the
lead's independent review are recorded separately.

| Target | Actual facts and selected path | Evidence |
|---|---|---|
| Mac M3 Max / Metal | Adapter-read `INDIRECT_EXECUTION`: I. Explicit record mask clears it: D. | [Oracle](paths/report.json.gz), [inspection](paths/inspection.json), [BASE preservation](indirect/mac.json.gz) |
| Chrome 156 / hardware WebGPU, M3 Max | First adapter-backed record contains `INDIRECT_EXECUTION`; production I; no mask. | [One-session BASE/current/repeats](indirect/chrome.json.gz) |
| iPhone 18 Pro / iOS 27.2 simulator | Adapter record omits `INDIRECT_EXECUTION`; production D; no mask. | [Locked ordinary function/lifecycle](ios-function.json.gz), [rotation](rotation/function.json.gz), [visual inspection](ios-visuals/inspection.json) |

Unknown adapter facts also select D. The public adapter-less host/attach test observes Direct and zero indirect
resources. Both-path GPU/material suites cover discard, history, capability changes, replacement, remote feeds,
promotion and Sun casters. Direct creates no INDIRECT, cull, count or readback resources. [Check](check.json).

Mac D/R is exact in five views: noon 0°/18°/36°, low sun and diagnostic volume/mirrored/edge-straddling meshes.
R independently validates cooked ranges/placements. I/D/R repeats are exact; visible D-only omissions are detected,
R remains unchanged and restored D/R is exact. Retain-all-meshlets equals I. Inspected I/D residuals track draw
partitioning with matching geometry/materials/light/shadows/volume; no tolerance, edge exclusion or low-level cause
is claimed. The refreshed integrated renderer reproduces every retained inspected image. [Assessment](paths/inspection.json).

Production I output/counts and both-build repeats are exact across [BASE](BASE) and current: five Mac and four Chrome
views. BASE is the parent of the first implementation commit. Archived builds have separate Cargo roots and only
caller/fixture/witness overlays. Chrome uses the T2 submitted tap in one hardware session without a Screen Recording
grant. Identical captured payloads are [deduplicated](indirect/payloads.json); individual identities/hashes remain.

Accepted PLATFORM-RUNTIME-IOS M7 resolves the historical [letterboxing failure](rotation/layout-failure.json).
Fresh locked ordinary runs advance counters, hold background submissions, recover the drawn set on generation 2
and rotate on the same process/window/generation. Held qualification uses public pause/Sun/yaw controls separately.
Six native still views/crops pass; held scene pairs are identical (two pairs differ only in status-bar UI). Attachments,
viewport and projection match each oriented extent and the authored 46° FOV. [States/times](ios-visuals/run.json.gz).
Sixteen native pan samples span presented counts 1559–2759 and both holds; largest sample-start gap is 1.118178 s.
Every sample has complete entering terrain. The [lossless sequence](ios-visuals/pan-lossless.mkv) round-trips all samples
exactly; playback uses one second per sample for inspection, with actual times in the record. Unsampled frames,
cadence and speed remain unqualified.

Four release Mac I/D pairs alternate order, 128 completed samples per arm, 1920×1080 valley noon, quiet load.
Per-run median forward: I 1.240–1.244 ms / D 1.283–1.330 ms; contribution compute: I 0.01025–0.01048 ms / D absent;
encode CPU: I 0.270–0.289 ms / D 0.149–0.188 ms. Forward includes contribution compute. No path is promoted.
[Samples/load](cost/paired.json.gz), [command audit](cost/command-audit.json), [passing tripwires](cost/tripwires.json.gz),
[three genuine quiet BASE runs per history](cost/baseline-history.json.gz). Existing busy/saturated history is preserved.

M1.7 and M1.8 require the owner to lock the Mac manually before the loop verification and keep it locked through
the simulator rows. Startup allows a bounded 120-second setup wait and still requires the real lock witness.
The [standalone function run](ios-function-only.json.gz) and [standalone lifecycle run](ios-function.json.gz) both pass
with locked witnesses before and after.

Reproduce the named DONE checks under `agent_loop.py lock gpu`; `scripts/cooked_draw_ios_sim.py --visuals` supplies
stills/pan and `scripts/cooked_draw_cost.py` supplies paired costs (both require a new `--output`). The launcher pins Rust 1.99.1. After M1 acceptance, T2-CAPTURE M2 may consume ordinary cooked attach; it still owns
submitted-frame identity/time and every-frame proof. Retained code/fixtures and [selected hashes](selected-hashes.json)
support reconstruction; owned bulk scratch is retired with [closeout conditions](closeout.json); build output is cleaned at stop.
