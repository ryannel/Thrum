# PLATFORM-RUNTIME-WEB M1: record

Moved from the web shell plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape. M1 closed on 8 November 2026 before the loop, so this section is its acceptance record, unchanged; the loop accepted nothing here. Evidence: [G3B-WEB-SHELL](../v1-workload/G3B-WEB-SHELL-2026-11-08.md). Reviews are in [the log](../../plans/PLATFORM-RUNTIME-WEB/LOG.md).

## M1 Engine web shell carries the V1 page unchanged: done

Done when:
- [x] `thrum_app::web` owns the instance, surface, device request, loss signal, reconstruction, clock policy, frame
      step, loss gates, status and samples. The facade holds a `WebShell` and no runtime, so the runtime's GPU methods
      are unreachable from it except through the shell's gates. This command prints nothing:
      `rg -n 'wgpu::Instance|set_device_lost_callback|get_current_texture|\.present\(|FrameHost|Runtime\b|\.host\.' examples/v1-game/src/browser.rs examples/v1-game/src/browser/`
- [x] A camera and time changed during loss are retained, and the recovered device both renders and presents them:
      the retained-view witness passes, first on today's facade and then on the extracted shell.
- [x] While lost, material reload is refused and attach is deferred, then drawn on recovery: the two new witnesses
      pass, first on today's facade and then on the extracted shell.
- [x] Capture stays refused while lost, and input, pause, visibility, overload and solar time stay usable while lost.
      These are the existing loss and recovery cases. Camera yaw is covered by the retained-view witness. Camera pitch,
      parameter files and overlay go through the same CPU edit path, but no case exercises them while lost, and this
      plan claims nothing for them.
- [x] The Chrome checker passes: `check_v1_browser.mjs --case all`, which includes every case but `material` (and
      now `valley-late-attach`), plus
      `--case material` on the development page. The checker is not edited after the witness commit.
- [x] The G7 route check passes against the committed baseline. The pre-change work item records two runs of the
      witness commit's route and checker images. If those two are byte-identical, the post-change images must be
      byte-identical to them. Otherwise, the post-change route grid difference against the pre-change run must be
      zero at the route check's own cell grid.
      The route stops are byte-identical before and after. The check failed on the reference too until Thrum Core
      refreshed the night baseline (73fdca0c); on that baseline it passes with the extraction (0.003 / 0.004 / 0.005).
- [x] `report()` text keeps its field names, order and number formatting. The default-page Wasm size, before and
      after, is recorded.
- [x] `./dev check` passes, and an independent review agrees. Its brief and verdict go on the Reviews line.

Work:
- [x] Loss gates and witnesses on today's facade, in one change: the material refusal, the deferred attach, and the
      three checker witnesses. Run on the base commit plus this change, then posted to Thrum Core as a seam diff with
      its evidence and landed first. Serves the witness items.
- [x] Pre-change reference: two runs of the full checker and the route check on the witness commit, recorded under
      `tmp/g3b-web-shell/`. Serves the checker, route and report items.
- [x] `thrum_app::web` with the shell, recovery, loss signal and gates moved from the game, `WebWorld`, and native
      unit tests of `web_clock` (origin, refusal, clamp, visibility ∘ pause, a play change moving the last time).
      Overload recovery is the shell calling `pause` then `resume_after_drain` on `WebWorld`; it has no clock state. Serves the ownership, witness and checker items.
- [x] The v1-game facade rewritten over `WebShell`; `recovery.rs` moved into the shell; the `rg` check prints
      nothing. Both wasm feature sets and `thrum-app` lint clean. Serves the ownership item.
- [x] Checker, route and Wasm-size runs; review brief and review. Serves the remaining items.
