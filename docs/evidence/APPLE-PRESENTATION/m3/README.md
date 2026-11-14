# M3 builder evidence

Builder evidence, 14 November 2026. D3.2 reconstruction and D3.4 camera-cut reset
pass on Metal at `c3d052a4`. D3.5 moving-route timing passes at `3f896417`.
Fresh route execution, both targets' motion, exact baselines, the full gate and
integration remain unqualified.

## Moving-route timing

`python3 scripts/apple_presentation_m3_timing.py --output tmp/apple-presentation-m3/route-timing --store tmp/apple-presentation-m3/valley`
passes under the shared GPU lock on Apple M3 Max / Metal, portable recipe,
at 640×360. The 361 submitted samples in each arm have identical world/camera/solar
and alpha inputs. Order is TAAU then off, with capture only at the same route
checkpoints; shared-host load and exact executable digest are in the [report](route-timing/run.json.gz).
The [route](route-timing/route.json) differs from the canonical route only in extent.
The complete [TAAU](route-timing/taau.log.gz) and [off](route-timing/off.log.gz)
records retain per-frame GPU pass times.

| GPU time at 640×360 | Median | p95 |
| --- | ---: | ---: |
| TAAU resolve | 0.046709 ms | 0.049792 ms |
| Scene with TAAU | 2.994001 ms | 3.210628 ms |
| Scene with reconstruction off | 2.778083 ms | 3.036457 ms |

No resolve executes in the off arm. Scene sums exclude surface presentation.
The nine-times-pixel-count heuristic is 0.420381 ms, within the design's 0.2–0.5 ms
1080p estimate, so the departure tripwire is false. This is not a measured 1080p
time, display cadence or photon measurement.

## Motion review in progress

The ten Mac real-time capture arms at `8fa5592d` complete on Metal, with the
portable TAAU pass observed only in enabled arms. [Retained clips and campaign](motion/mac/campaign.json),
compressed per-arm manifests/runtime records and [native inspection notes](motion/motion-review.md)
preserve what was inspected and what remains unjudged. Native crops show an opaque
first spawn and no persistent blue silhouette in the inspected reveal frames.
The four all-frame star-core flux traces and the all-frame scrub sky trace support
the sampled review; the scrub diagnostic's minimum-error lag is zero milliseconds.
These measurements introduce no acceptance threshold.

The [Chrome motion evidence](motion/chrome-review.md) retains all ten Chrome
real-time arms at `3f896417`. A fresh GPT-6.1 Sol medium opinion and the builder
both find a ragged/speckled fringe around the revealed small blocks and the
moving cube, also visible in Mac counterparts. Both motion items stay open;
the reconstruction path needs a disocclusion-edge diagnosis and repair.

The [1× software-decoder record](motion/software-playback.json.gz) delivers every
movie frame. Decoder completion and static source-frame inspection are distinct
from continuous visual observation. D3.3 remains open because direct 1× viewing
has not occurred. Chrome motion also remains unqualified, with the edge concern
above. Fresh route execution remains unqualified.
Original lossless Mac archives remain locally in owned scratch
for follow-up, with hashes and retention reasons in [the retention record](motion/mac/retention.json).

## Reconstruction and cut

`python3 scripts/run_v1_showcase.py reconstruction --target mac-metal --no-build`
passes on Apple M3 Max / Metal at 640×360. The oracle's actual executable digest,
build, adapter, load and arm order are in [the complete report](proof-c3d052a4/run.json)
and [execution log](proof-c3d052a4/oracle.log). The command rebuilds its game-host
test executable even with the ordinary product-build step skipped.

| Still route checkpoint | TAAU p99 ΔE | No-AA p99 ΔE |
| --- | ---: | ---: |
| Sunrise | 0.945706 | 1.123111 |
| Noon | 0.969162 | 1.369814 |
| Sunset clear Sun | 0.813268 | 0.888748 |
| Sunset Sun | 0.944157 | 1.099360 |
| Twilight | 0.960176 | 0.971272 |
| Night upslope | 0.883204 | 0.924525 |

Every full-frame p99 is below 3 and strictly below no AA. Every history-disabled
control is rejected. The two independent ordered f32 reference means agree bit
for bit at every checkpoint, with all held world/camera/solar/celestial/alpha,
cooked-generation and exposure assertions passing across all four arms.
The retained checkpoint directories contain reference, TAAU and no-AA PNGs,
compressed f32 linear means and complete frame records. [Image hashes](proof-c3d052a4/images.json)
identify the exact displayed RGB and round-trip-verified RGBA PNG bytes.
The builder inspected the noon and night whole-frame pairs at native size.

The post-cut frame is byte-identical to an independently prepared reset host;
the suppressed-reset control differs. Submitted histories are `[65,0]`, `[1,0]`
and `[65,64]` respectively. The [cut](proof-c3d052a4/cut.png),
[independent reset](proof-c3d052a4/independent-reset.png) and
[suppressed reset](proof-c3d052a4/suppressed-reset.png) are retained.

Held-checkpoint resolve timings are 0.0465–0.0520 ms at 640×360, with capture on
in both paired arms and the first 16 frames excluded. These are supplementary
held measurements. D3.5's actual moving-route pair is recorded above; no
measured 1080p time or panel cadence is claimed.

## Implementation and earlier attempts

The portable registry selects TAAU. Its compute pass reads jittered forward colour,
reversed-Z depth, dilated motion, reactive and GPU-resolved exposure; it writes linear
colour through APPLE-TILE's retained pre-tone seam. The existing SDR transform runs
once. Colour and linear-depth history pairs are shared by both game frame slots and
selected with the submitted transform-history cursor. Cancelled work cannot switch
their read side. Capture-stage replacement retains the same history pair.

The separate reference shader sums each 32-phase cycle in RGBA32F and exposes its
average through the same pre-tone seam. The explicit game-host oracle records held
world/camera/solar state and actual GPU exposure on every submitted frame, compares
the two cycles, and includes history-disabled and suppressed-cut-reset controls.
`scripts/reconstruction_difference.py` compares every SDR pixel in D65 CIELAB using
CIEDE2000; its published supplementary-pair checks pass.

Verified for the changed kernel: strict native and pinned-toolchain Wasm Clippy,
both WGSL modules' Naga validation, native release and Wasm builds. These establish
no target image quality, motion quality or GPU cost.
The real Metal alternating-slot/reset execution test now passes, as do the
authoritative tick-60 replacement and sampled moving-light fixture tests. The
game-reference proof and moving-route timing now pass as recorded above. Chrome
execution, motion sequences, exact baselines, the full gate and integration remain
unqualified.

Technique source: [Karis, High-Quality Temporal Supersampling, SIGGRAPH 2014](https://advances.realtimerendering.com/s2014/index.html).

First target attempt at `55ae06ee`: [complete metrics](attempt-1/run.json),
[noon TAAU](attempt-1/noon-taau.png), [independent reference](attempt-1/noon-reference.png).
All six p99 values are below 3, but each fails the required improvement over no AA.
The two reference SDR images agree exactly, the history-disabled controls are rejected,
and the cut equals its independently reset render while the suppressed-reset control
differs. These are historical results, not passes for the changed kernel.
The follow-up corrects depth-history jitter addressing, preserves exact unchanged
colour during clipping/blending, and rescales from full-precision retained exposure.
That follow-up was tested in the second attempt below.

Second target attempt at `9c1349f2`: [complete metrics](attempt-2/run.json).
All six independent scene-linear cycle means agree bit for bit. Sunrise and
sunset-sun improve on no AA, but noon, sunset-clear-sun, twilight and night do not.
Both camera-cut controls still pass. A subsequent uncommitted diagnostic with
depth rejection disabled improves five checkpoints, isolating false rejection as
a remaining cause. Rejection is retained in the implementation, using the range
of the complete reprojected depth footprint instead of one nearest-depth texel.
The follow-up also uses unbiased half-float history quantisation to avoid a
long average becoming stuck below a representable value. It is the passing
`c3d052a4` candidate recorded above.

The follow-up also records two independent f32 means from the T2 submitted
scene-linear RGB16F inputs, comparing all components against a combined 64-epsilon
summation bound. This strengthens the first run's SDR-only repeat check. Reference
SDR images come from the same submission's identity present copy; TAAU/no-AA
images retain the ordinary submitted SDR tap. The one SDR transform is unchanged.
