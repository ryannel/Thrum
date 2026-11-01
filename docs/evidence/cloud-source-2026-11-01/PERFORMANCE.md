# Source costs and motion receipts

1 November2026. [Raw source timing samples](measurements/source-kernel-final.json),
[scope/limits](measurements/SUMMARY.json) and [motion manifest](motion/manifest.json) identify actual loaded artifacts.
This is a source instrument, not the Thrum renderer. The measured host was M3 Max36GB, macOS26.6.2,
Chrome156.0.8078.80, Apple/Metal3 WebGPU. It is not the M2 reference target.

| Source-only profile,100 retained samples after10 warmups | GPU compute median /p95 /max | Host command encoding p95 /max |
| --- | --- | --- |
|1x, formed body |0.197 /0.459 /0.983ms |0.300 /3.200ms |
|64x phase progression |0.197 /0.393 /0.721ms |0.200 /0.700ms |

The valid per-pass GPU timestamps exclude the R16 copy, light preparation and every view. Copy timing is unmeasured:
cross-pass timestamp probes produced invalid/nonmonotone totals and were rejected. Observer queue completion includes
the copy, driver scheduling, waits and query instrumentation; its~9.5–11.8ms medians are not GPU execution time.
Raw normal/timelapse samples remain in the receipt. Desktop apps were not isolated; these short component runs do
not certify sustained thermal/frame behavior. Host command-encoding tails exceed the desired0.5ms incremental CPU
allocation in this instrument. The full3ms native/5ms browser cloud service and complete target frames remain open.

Actual runtime source density+header is6.260437MiB of11MiB. GPU source bytes are6,491,440; CPU recovery also retains
encoded content. Offline validation float-frame caches are not the intended runtime CPU query implementation.
The proposed88MiB720p service and512MiB whole-browser cap are not measured process residency here.
The unchanged cold worksheet with actual source bytes predicts~3.89s; code/shader/fixture/readiness values remain
estimates, so this is not cold-start qualification.

[Normal1x](motion/normal-1x.webm), [front16x](motion/front-16x.webm),
[silhouette64x](motion/silhouette-64x.webm) and [side16x](motion/side-16x.webm) are actual768x576 GPU canvas captures.
Their JSON phase/cadence receipts are losslessly stored as `.json.gz` beside each video. Footage uses the source
and instrument at commit`c824069`; the manifest verifies its actual script hash. Later viewer host/measurement
changes retain the same compute shader and source, but are not silently relabelled as the capture build.

Independent review inspected202 extracted samples/four overview sequences and30 native keyframes, including dense
one-second normal windows. The parent inspected overview sequences and native growth/breakup frames too.
This is sampled-motion assessment, not uninterrupted real-time viewing or a proof of every encoded frame.
The [motion review](reviews/MOTION-REVIEW.txt) records remaining faint birth scaffolding, static mature pacing,
rounded late cores and soft detail. Variable encoded frame gaps reached195/127/64/73ms in normal/front16x/mask64x/
side16x respectively; retain that recording limitation instead of claiming ideal frame pacing. Source age is
monotone and reaches240 at each rate, with no deliberate weather-time hold/drop.

[Fresh recovery](measurements/recovery.json) rebuilt the source from committed inputs into new directories,
without solver caches or Broadsides files: packed density bytes, all R8 frames and times match exactly.
Preparation wall times/paths/receipt hashes naturally differ; one repeat took71.1s versus the earlier22.3s.
Both are offline preparation measurements, not runtime cost. The admitted source, master recipe/shape, exact
input pins and selected evidence are versioned; rejected bulk caches can therefore retire.
