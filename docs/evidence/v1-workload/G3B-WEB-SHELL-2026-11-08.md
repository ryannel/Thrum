# G3b engine web shell: V1 page parity, 8 November 2026

Plan: [PLATFORM-RUNTIME-WEB-PLAN.md](../../plans/PLATFORM-RUNTIME-WEB/PLAN.md), milestone M1. The browser
platform runtime moved from `examples/v1-game/src/browser.rs` and `browser/recovery.rs` into `thrum_app::web`
(987a3ac3, review fixes c1e03736). Reference is the witness commit a66352e3 (the loss gates on today's facade), built
in a separate worktree. Both sides ran on this M3 Max, Chrome through `scripts/check_v1_browser.mjs` with the release
`thrum-cook`, on a shared GPU; timings here are contended and not compared.

| Check | Reference a66352e3 | After c1e03736 |
| --- | --- | --- |
| `--case all` (30 cases) | 30/30, 30/30 | 30/30, 30/30 |
| `--case material`, dev page | pass, pass | pass, pass |
| Route check baseline, worst difference over 12 frames | 0.004 / 0.790 / 2.470, both runs | 0.004 / 0.790 / 2.470, both completed runs |
| Route images, 106 per run | the route stops byte-identical between the two runs; the 43 `sky-reflections` diagnostic images differ | the route stops byte-identical to the reference in both runs; one run matches the first reference run in all 106 |
| Route check verdict | FAIL: drift at native and Chrome `night-upslope`, both runs | the same FAIL, both completed runs |
| Default-page Wasm | 2,213,786 B, `7a67950c…` | 2,224,200 B (+10,414, +0.47%), `88c68249…` |
| Dev-page Wasm | — | `c77c2292…` |
| `report()` field names and order | — | identical to the reference |
| `./dev check` | — | pass (third run; see flakes) |

**Rerun with the corrected retained-view witness, on main 65d0c011.** After Thrum Core's checker correction (A and A′
observed before any edit), the branch tip ran again: `--case all` 30/30 twice, every retained-view round `presented:
pass` in both runs, `--case material` on the development page passes, `./dev check` passes. The route stops are
byte-identical to the reference and the route check fails on the same baseline drift. Default-page Wasm at this base:
2,228,890 B with the gates only (`9f7aef86…`), 2,239,811 B with the extraction (`225b3348…`), +10,921 B (+0.49%).

**Route check after Thrum Core's night baseline refresh (main 73fdca0c).** With the extraction rebased on the landed
loss gates (0a9009c1), the route check passes: worst difference 0.003 / 0.004 / 0.005 over 12 frames at thresholds
1.0. The failing rows below are kept as raw evidence of the earlier baseline.

**Route check.** The night-upslope drift beyond the stored baseline is present on the reference with the same worst
differences, so it predates this change (main's renderer has moved since the baseline was saved). The extraction adds
no difference at the route check's grid. One post-change route run stopped when its owned Chrome failed to exit
cleanly after `sky-reflections` had passed; a third run completed with the same result as the others.

**Flakes seen, not caused by this change.** One reference `--case all` stopped at `pause` ("explicit two-second
pause/resume was not observed without overload"); the two reruns passed 30/30. One `./dev check` failed in the native
`cooked_material_gpu` history assertion while the GPU was busy; it passed 3/3 alone. Neither test reaches the web shell.

Run logs: `tmp/g3b-web-shell/ref/` and `tmp/g3b-web-shell/post/` in the worktree (not retained).
