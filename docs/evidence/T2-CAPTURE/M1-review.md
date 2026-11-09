# T2-CAPTURE M1 independent milestone review

Reviewer: Claude Opus 5.5 (claude-opus-5-5), independent of the Codex builder, 9 November 2026.
Claimed commit: `96687e30` (base `dff3eb5e`, the M0 third-review pass). Target: chrome-webgpu on this Apple M3 Max,
Chrome 156.0.8078.81, `--headless=new`. The console was already locked for the whole review
(`IOConsoleLocked = Yes`, `CGSSessionScreenIsLocked = Yes` before and after every session). I did not lock or unlock it.

**Verdict: PASS M1.** Both items pass. No blocking defect outside the items. Ten advisory findings follow.

## What I ran (verified by running)

All GPU and Chrome work ran serially under `agent_loop.py lock gpu`, from the clean checkout at `96687e30`. I rebuilt
the web bundle (`examples/v1-game/web/build.sh`) and `thrum-cook` myself. The served Wasm SHA-256 was `853172ae…`,
the same in every capture manifest and every legacy check. Scratch was `tmp/t2-m1-review/`, deleted at the end.

- `./dev check` (the profile's Check) under the gpu lock: exit 0. It includes the new
  `live_encoding_reports_the_disk_error_after_its_io_channel_closes` test and the six real-Metal `submitted_capture`
  tests, among them odd-width surface correspondence, pending-capture device loss and saturated leases.
  `./dev check-docs` also passes.
- **Ten back-to-back ordinary elapsed-time sessions** (`--mode realtime`), then stall and resize, so twelve complete
  real-time captures in all, plus one fixed-time session. Every one was locked before and after, with launcher
  preflight `false` before and after and Chrome exit 0.
- Negative controls: `stall`, `saturation` ×2, `slow` ×2, `loss` ×3 (two through a scratch copy of the runner, which
  differs only in reading `submitted_capture_pending()` and the page's own `navigator.gpu` adapter just before the
  device is destroyed), and `off`.
- The affected consumers in `scripts/check_v1_browser.mjs`: `tabs`, `resize`, `device-loss`, `device-recovery` and
  `moving-route-controls` (seven cases). All pass.
- Packaging (`scripts/package_v1_browser.py`) of four complete sessions (rt-01, fixed, stall, resize). Each verified
  every PNG round trip, with preview PTS error at most 0.33 µs. Packaging refused all four incomplete sessions
  (saturation, slow, loss, off) with `incomplete browser evidence`.
- Margin probe: two ordinary sessions with 12 busy-loop CPU hogs on this 14-core machine.
- Responsibility probe: I started the builder's launcher app with headless Chrome and asked the kernel
  (`responsibility_get_pid_responsible_for_pid`) which process is responsible for Chrome.

## Results

### Elapsed-time series (M1.1, M1.2)

| Sessions | Result |
| --- | --- |
| rt-01 … rt-10 (ordinary) | **10/10 complete**, 481–482 images each (4,815 in all), zero renderer/export omissions, zero overload, clamp or dropped view. Contiguous submission IDs; presented-counter head and tail match. Every receiver hash re-verified after decompression. Max submit gap 21.3–23.5 ms (60 Hz timer opportunities). Export peak 3–4 of 8. Final sync 34–413 ms |
| Presented pose against host time | The pose is `(tick − 1 + alpha)/60`. In each of the five sessions I checked, it tracks engine `submit_ms` within 1.4 ms at maximum (p99 1.1 ms), never steps backwards, and has no repeated image during motion (minimum mean frame difference 1.1/255). Repeats occur only in the paused initial hold (61 identical frames, about 1 s) |
| c-stall | Complete, 483 images. The 250 ms page dispatch stall gives a 271.2 ms submit gap (images 268→269). The world sheds the time under the ordinary overload policy (one overload event), ordinals 91–104 are skipped, and motion then resumes |
| c-resize | Complete, 482 images. The canvas becomes 961×541 and the presenter generation goes 1→2 at image 269 with a 20.6 ms gap. The captured source grid stays at 1280×720, correctly labelled, with no corruption |
| hog-1, hog-2 (12 CPU hogs) | **Both refused** (export omissions 13 and 5, export peak 8/8, renderer omissions 0). This is the correct refusal, and it shows where the export path's margin ends (advisory F2) |

### Fixed-time

`fixed` is complete: 481 of 481 opportunities submitted and captured, at synthetic 60 Hz with explicit waits. The
tick/alpha pairs display one tick per sample (alpha 0 at tick T is the same pose as alpha 1 at T−1). There is no
repeat outside the hold, and the first image is byte-identical to the elapsed-time first image.

### Negative controls (M1.2)

| Control | Result, and whether it really exercises the failure |
| --- | --- |
| Loss ×3 | In my probe, `submitted_capture_pending()` was **1 reserved slot at the moment of destruction** (twice), so a readback really was pending. Collection refuses ("graphics device was lost before the capture finished"). A new session refuses while lost. Authority advances 179→191. After recovery the old session refuses ("retired capture generation"). A fresh session-2/device-generation-2 image (3,686,400 bytes) is exported. I viewed it: the complete scene, no stale or black pixels |
| Saturation ×2 | Withholding collection for 2 s gives 118 and 117 **renderer** ring omissions, 362–364 images kept and authority reaching tick 599. Refused |
| Slow ×2 | A 500 ms receiver delay keeps the page at most 8 requests in flight and the receiver at a peak of 6. 386 and 387 export omissions, authority continues. Refused |
| Off | Observer-only arm, no images, labelled `observer_only` |

### Desktop capture and image origin

- **Images are submitted WebGPU output, not the screen (read and run).** `FrameHost::encode_scene_only` reserves a
  tap slot, and `prepared.encode_contributed` copies the final engine target into it. `finish_encoded` then encodes
  the presenter pass from that same target into the canvas view, and sends both in one `queue.submit`. The web shell
  only adds begin, next, recycle and end around this (`crates/thrum-app/src/web/capture.rs`). Pixels reach Node as a
  typed-array copy in an HTTP POST. Neither the capture page nor the runner uses `Page.captureScreenshot`,
  `getDisplayMedia`, `toDataURL` or any OS capture API. Within one tick, images with different alpha differ
  (interpolated submissions, not endpoint redraws), and the history epoch steps once at the declared cut.
- **The launcher honestly meets "without desktop capture" (run).** The kernel reports the ad-hoc-signed
  `ThrumCapture.app/Contents/MacOS/Launcher` as Chrome's responsible process. That is the identity TCC checks, and
  its `CGPreflightScreenCaptureAccess()` reads false before and after. My own shell's responsible process is
  `claude.app`, the host the builder's earlier pilots inherited (they saw permission available). So the launcher
  stops Chrome inheriting the host's grant, rather than hiding it. It requests no permission. In any case no pixel
  path touches the screen, and headless Chrome has no window to capture.
- **Real hardware adapter (run).** The page's own `navigator.gpu.requestAdapter({powerPreference:'high-performance'})`
  reports vendor `apple`, architecture `metal-3`, `isFallbackAdapter: false`. Chrome's GPU info reports ANGLE Metal
  on the Apple M3 Max, with WebGPU `enabled`. The runner refuses software or non-target GPUs.

## Inspection list for M1.1 (written before viewing any image)

1. Initial hold (about 1 s): stable, complete 1280×720 frames, opaque alpha, no channel swap (blue sky, sand ground)
   and no row shear or padding stripes.
2. Native full frames at each route checkpoint (approach tick 225, mirror-light 350, pre-cut 376, post-cut 386,
   near-plane 530): cooked terrain, sphere, rock and instance cubes present and plausibly lit.
3. The cut (images 440→441): exactly one viewpoint change, with no ghost of the pre-cut image after it and the
   history epoch stepping 1→2.
4. Moving segments, using native crops at 1:1 and adjacent-frame differences: monotone progress, no repeated moving
   frame, no temporal trails or stale-history smears at edges.
5. The 1× timeline: every image placed at its engine submit time. Gaps correlate with recorded scheduling, and the
   presented pose follows host time.
6. Final hold: stable and complete, with the running world still visible.
7. Resize: the captured grid stays 1280×720, uncorrupted, after the canvas becomes 961×541.
8. Loss replacement image: the complete scene, neither black nor stale.
9. Stall: the image before and after the 271 ms gap shows a held then resumed view, with no jump that hides time.
10. No DOM, status text or browser chrome in any image.

| # | Judgement (my own captures) |
| --- | --- |
| 1 | Pass. Images 179–240 are identical, alpha is 255 throughout, colours are correct and the rows are clean |
| 2 | Pass. Images 285, 410, 436, 440 and 590 show complete geometry. The near-plane frame shows a cube face filling the view, with correct occlusion of the cube behind it |
| 3 | Pass. 440 (sphere left) → 441 (rock left, sphere right). A 1:1 crop of 441 shows crisp sphere edges and no pre-cut ghost. Epoch 1→2 at 441 only |
| 4 | Pass. A twelve-frame contact sheet (300–630) shows a coherent approach, cut, orbit and near-plane pass. The 1:1 crop of 436 has no trails. Frame differences during motion are 1.1–4.2/255, with the single 41.6 spike at the cut |
| 5 | Pass. Pose against submit time is within 1.4 ms, and preview PTS match within 0.33 µs. I cannot watch video, so the motion was judged from the frame sequence and the timeline, as the proofs reference directs |
| 6 | Pass. The last 75 frames differ by about 0.1/255 each: the running world (sun), with no instability |
| 7 | Pass. Resize frames are clean, with presenter generation 2 and source extent 1280×720 |
| 8 | Pass. The replacement image is the complete scene at tick 197 |
| 9 | Pass. 268 (tick 208, alpha 0.08) → 269 (tick 208, alpha 1), 271 ms apart, then normal stepping |
| 10 | Pass. No status text or chrome appears in any frame viewed |

Against the bar: camera continuity in submitted output is honest at 60 Hz timer opportunities. Images are complete
and every sample's identity is joined. Browser scheduling (headless page, timer opportunities, ordinary lifecycle) and
surface limits (source grid, before canvas presentation) are recorded. Scene artistic quality stays with the
consuming milestone. One example: the ground softens at the bottom of fast-moving frames, which is renderer behaviour
and outside T2.

## Item verdicts

- **M1.1 pass.** I reran the documented cooked-route commands in both modes on hardware WebGPU while the console was
  locked, with no OS Screen Recording access for Chrome's responsible process. Adapter, backend, browser build and
  flags, scheduling, bundle and cooked hashes, clocks and complete joins are verified. Visibility is not faked: the
  page refuses if hidden, and `route.html` and its seven refusal controls are unchanged and pass. Headless is labelled
  as such. The inspection list passes.
- **M1.2 pass.** Pending-readback loss and recovery, ring saturation, slow export and an actual stall each exercise
  their failure. The page loop and authority continue, stale captures refuse, PTS keep the delay and missing samples
  refuse completion. The actual-tab, resize, device-loss and recovery consumers pass. `./dev check` passes under gpu,
  and the target-specific Chrome command fails rather than skipping when the hardware is absent (`hardware()` throws).

## Findings

No blocking findings. All of the following are advisory and ranked.

1. **Advisory, verified (read).** The capture manifest does not record the page's actual WebGPU adapter
   (`adapter.info`, `isFallbackAdapter`). The proof of hardware with no fallback rests on Chrome's GPU-process
   `SystemInfo` (ANGLE/GL device string plus `featureStatus.webgpu`). My probe confirmed `apple`/`metal-3`/not
   fallback, but the evidence should carry it. The design's manifest list also names a recipe/registry digest and
   shader hashes, which are covered only implicitly by the Wasm hash. Where: `scripts/capture_v1_browser.mjs:592-593`.
2. **Advisory, verified (run).** The export margin is modest under heavy CPU load. With 12 busy hogs on 14 cores,
   both sessions refused (export 8/8 in flight, 5–13 omissions). Ordinary peaks are 3–4/8. The refusal is correct, but
   the `gpu` lock does not exclude CPU-heavy concurrent builds, so consumers on a busy machine will see refusals. The
   runbook should say that a quiet CPU is needed, or should record load.
3. **Advisory, verified (read).** Fixed-mode packaging derives PTS from the record index
   (`scripts/package_v1_browser.py:722`, `i * 1e9 / 60`) rather than from each opportunity's declared world time. The
   runner compares records with the presented counter, not with opportunities, and does not refuse `dropped_views`
   (`scripts/capture_v1_browser.mjs:624,639`). A dropped fixed-time view would therefore compress time silently. This
   was not triggered (481/481 here).
4. **Advisory, verified (read and run).** Skipped elapsed-time opportunities are not recorded with reasons. They
   appear only as ordinal jumps (`examples/v1-game/web/capture.mjs:411`). In the stall, ordinals 91–104 were skipped.
   The design asks for skipped opportunities to be recorded with their reasons.
5. **Advisory, verified (read).** The included/excluded UI scope lives in `runtime.json` (`ui`), not in
   `manifest.json`, which states only the encoding stage. Its "engine route overlay" is
   `RouteOverlay::Proof`: scene content (reseeded instances, mirrored basis, light), not an engine UI. This route has
   no engine-rendered UI, so capturing an in-canvas engine UI is untested on Chrome.
6. **Advisory, verified (read and run).** Per-image extent and row stride come from the route JSON in the runner,
   not from the engine's `SubmittedImage`. The receiver checks only the byte count. Chrome captures never exercise
   row-padding removal, because 1280×4 = 5120 is already 256-aligned and resize keeps the source grid. Odd-width
   correspondence is proved only by the shared Metal test in `./dev check`.
7. **Advisory, verified (read).** The stall control is a JS busy loop in the capture driver, not engine-side
   render-actor and input-dispatch actions as on Mac. It blocks the page for real and is labelled, so it satisfies
   M1.2, but it does not exercise an engine-owned stall hook on the web.
8. **Advisory, verified (read).** Chrome observer cost is not isolated: the README reports ON 0.8 ms against OFF
   1.4–1.5 ms from separately paced processes. No M1 item requires it, but the design's general matched on/off
   costing is unmet for Chrome.
9. **Advisory, verified (run).** Final storage sync varies more than the README's 33–51 ms: 34–413 ms in my ten runs
   (rt-04: 413 ms). This is informational.
10. **Advisory, verified (read).** About 48 MB of binary evidence is committed, including three near-identical 1×
    previews and three contact sheets (`docs/evidence/render/t2-capture-m1/`). This is allowed by the storage rules,
    but heavier than a selection needs.

## What I only read (not run)

- The renderer tap internals beyond the M1 diff (`submitted_capture.rs`, `runtime.rs` encode and submit) were verified
  at M0. M1 changes only `submitted_elapsed`, which I traced to `Instant::now()` at session enable.
- The builder's retained sessions, `summary.json` and `final-order.json`. They record every attempt, including the
  failed `restricted-pilot` and `stall-01` and the superseded `saturation-02` that completed. They match the README.
- The M0 follow-ups in the diff (the writer disk-error mapping, the closeout record and README text). The new test
  ran and passed in `./dev check`.

## Diff read

I read all of `dff3eb5e..96687e30`: the code (`writer.rs` plus test, `web.rs`, `web/capture.rs`,
`submitted_capture.rs`, `browser.rs`, `browser/submitted_capture.rs`, `build.sh`, `capture.html`, `capture.mjs`,
`capture_browser_launcher.swift`, `capture_v1_browser.mjs`, `check_v1_browser.mjs`, `package_v1_browser.py`,
`t2_writer_probe.py`), the design, plan, done-when, maturity and CAPTURE.md changes, and the evidence scripts.
I found no hard-coded returns, canned data, test-only branches, weakened or deleted tests, silenced warnings or
default-on flags. Negative controls pass only when their failure is proved, and the off arm is labelled and cannot
supply images.
