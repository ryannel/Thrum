# T2 capture M1 builder claim

Chrome WebGPU on this Apple M3 Max/Metal, 9 November 2026. The ordinary submitted-output tap captures the cooked
moving route at its native 1280×720 scene extent. Fixed-time and elapsed-time drivers preserve the normal web shell,
world clock, history and lifecycle. Browser timer opportunities request 60 Hz. Manifests explicitly exclude DOM,
browser/native UI and physical display observation. Reproduction commands and operational limits are in the
[runbook](../../../CAPTURE.md#chrome-webgpu).

Five consecutive final elapsed-time runs independently verified **2,407 original frames**, with no renderer/export
omission, overload or dropped view. Each stored 600.4–601.6 MB for approximately eight seconds, against 1.77 GB raw.
Largest ordinary submission gaps were 21.8–23.7 ms. The fixed-time run verified 481 originals. All final runs observed
the console locked before/after and the LaunchServices launcher's Screen Recording preflight false before/after.
The local launcher requests no permission. The served Wasm, capture driver, route, source files, cooked root,
browser/adapter, flags and launcher hashes are retained in each session manifest. The exact measured runner is
retained here. Its later source cleanup changes indentation only.

| Control | Observed result |
| --- | --- |
| Actual 250 ms page stall | 270.9 ms image gap, ordinary overload/recovery, then continued route and final hold. Preview preserves the gap. |
| Pending readback plus actual GPUDevice destruction | Old-session collection and new admission refuse while lost. Authority advances 179→191. Retired generation refuses after recovery. A complete 3,686,400-byte generation-2 image is exported. |
| Surface resize to 961×541 | Presenter generation advances, 482 exact source-grid images remain correctly labelled 1280×720. |
| Withhold readback collection for two seconds | 118 omissions, 363 retained images, authority reaches tick 599. Completion refuses. |
| Delay each export request by 500 ms | Eight-request bound holds, 387 export omissions, 94 retained images, continuing authority. Completion refuses. |

The corrected saturation control is the final proof. Its earlier version accidentally drained during timer pacing
and completed. That attempt and every pilot remain in `summary.json` and `sessions/`. The process-sandbox pilot
failed GPU startup and was rejected. Initial host-launched pilots observed permission available. The qualifying
LaunchServices route retains Chrome's ordinary sandbox and establishes a separate denied launcher identity.
Apple documents the non-prompting preflight API, and its process-responsibility model informs this launcher setup.
[Apple preflight](https://developer.apple.com/documentation/coregraphics/cgpreflightscreencaptureaccess()),
[Apple process responsibility](https://developer.apple.com/videos/play/wwdc2023/10266/).

Builder inspection followed `inspection-list.md`. Native approach, mirror/light, pre/post-cut and near-plane
frames retain the scene and overlay without row/channel corruption or stale cut imagery. The declared cut advances
history once. Initial/final holds are stable. Native adjacent frames and timed Chrome playback samples show coherent
motion through the approach, cut and near-plane crossing. Playback stayed at rate 1 with media/wall ratio 0.99961.
The desktop player tool stalled, so the bounded Chrome review replaces that attempt. FFprobe verifies every preview
source PTS to ≤0.34 µs, with one labelled terminal duplicate. `selection.json`, `native/`, `playback.json`,
`audit.json` and the three promoted previews retain the inspection evidence.

`check-full.log.gz` records the passing GPU-locked `./dev check`. Existing actual tabs, resize, device loss/recovery
and all seven deterministic-route refusal controls pass in `lifecycle-order.json`. Final capture-on frame-step wall
medians are about 0.8 ms, versus 1.4–1.5 ms in separately sleep-paced OFF processes. These observations do not isolate
causal observer cost. Export peaks are three against an eight-request cap. Final file sync takes 33–51 ms in the
ordinary series. The browser writer is separately qualified on this content. M0's compression margin is not reused.

Full gzip originals and exact bundles remain locally in `tmp/t2-capture-m1/` through lead review, with a 16 GiB
per-session allowance. Selected native originals, full timing/provenance records and previews are retained here.
The M0 third-review scratch closeout verified its retained roots before deleting 23.52 GB, recorded in the
[M0 closeout](../t2-capture-m0/reliability/closeout.json). M1 awaits the lead's independent review.
