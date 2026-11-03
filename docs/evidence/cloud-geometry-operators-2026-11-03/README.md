# Cloud geometry operators — bounded result

3 November2026. **Boundary removal differs from fade; complete breakup remains unaccepted.** One static Cu master is held in place and tested with distance-support removal, density subtraction and an integrated-density-matched alpha control. This is not a selected complete cloud source, growth/death lifecycle, Cb anatomy or weather response. No source/runtime/budget acceptance transfers from these captures.

| Exact build | Retained role |
|---|---|
| [81c3f43f76b490fa](identities/81c3f43f76b490fa.json) | Final instrument: terminal GPU failure handling and12MiB chunk/blob/upload limits. Actual E0 and endpoint coefficient probes, plus a noisy16x12s happy-path recording. |
| [173d261dc10b86c2](identities/173d261dc10b86c2.json) | Four actual canvas recordings: paired SDF/alpha,1x120wallseconds and16x12wallseconds. |
| [fd0181d8140a4ff9](identities/fd0181d8140a4ff9.json) | Manual held SDF/alpha/density-subtraction stills across views and fixed noise. |

[Exact sources](instrument/), [asset/dependency pins](builds.json), [selected observations](observations/selection.json), [retained-file hashes](manifest.json) and [recovery check](checks/recovery.txt) preserve separate implementation, numerical and image evidence. Code/shaders/assets are byte-identical after recovery; historical builds remain historical. The final fixes do not alter optical/source algorithms.

## Inputs and recovery

The03 authored density comes from the already committed [CC0 source package](../cloud-authored-masters-2026-11-03/README.md), with [rights](../cloud-authored-masters-2026-11-03/inputs/LICENSE.txt) and [original provenance](../cloud-authored-masters-2026-11-03/inputs/provenance.json). Its128/256R16F payloads are referenced by exact compressed and uncompressed SHA, rather than duplicated here. The03 raw hashes are `c0365090769b38b48f33b206a0155f2cd0f34be58ba809999d4901bbd99fb844` and `42431f9da2d76e253d649cc35802fd1be7b23ddb698e73c6c4247ac3c90a97c9`.

Four128/256,iso.001/.005 distance volumes and one32³ noise volume are [losslessly compressed here](inputs/volumes/). [Curves and metadata](common/assets/) record exact inputs, coordinate transforms, half-distance error, reference loss, endpoint bounds and numerical native128 topology observations. Distances are opposite-class voxel-centre EDTs of `rho>=iso`, positive inside, in local metres×4worldscale. They are not exact continuous density-isosurface SDFs; surface/normal errors are at voxel scale. Original up-axis is an explicit source-package inference. Source pitch is16/8worldmetres; the2048m pose/centroid is pinned across quality.

Run from this directory with Python standard library and Node:

```sh
python3 verify.py
python3 restore.py --build all --output /tmp/cloud-geometry-restored
python3 test-recovery.py
cd /tmp/cloud-geometry-restored/81c3f43f76b490fa
python3 serve.py --port 8148
```

Choose a fresh output path; restoration refuses existing directories and outputs inside retained evidence. `--master-package PATH` selects a relocated copy of the referenced committed source package. Recovery reads only this package plus those pinned compressed density files, not any live `tmp/` experiment. The test relocates the package, restores all three exact served-file sets, calls each actual server's payload builder and runs every retained scoped test; temporary outputs are removed. It does not invoke a browser/GPU.

[Unchanged cook recipes and tool versions](cook/) permit independent regeneration. `cook/prepare-recook.py --workspace FRESH_PATH` prepares the original relative layout and exact dense inputs; use recorded Python3.14.10,NumPy2.5.4,SciPy1.18.1 to run distance then curves. No install is performed. Exact retained-byte restoration needs none of those numerical tools. Cross-hardware recook bit identity is not certified.

## Operator and clock contract

The reference is `q_ref=q*smoothstep(-32m,32m,d0)`, distinct from rawq. Native reference density-sum losses are1.28/2.47% at128 and4.68/6.82% at256 for iso.001/.005. A fixed multiplier is uniform1 or `.3+.7*N(materialUVW)` with clamp-trilinear32³half noise; no time wobble. SDF uses `q*smoothstep(-32,32,d0-e*M)`, subtraction uses `max(q_ref-rhoThreshold*M,0)`, and alpha uses `q_ref*alpha`. All main/light/shadow/reflection coefficient consumers share that function; other lighting math is unchanged and hash-tested.

LUT comparators match the native integrated density/extinction proxy at calibrated nodes, with fixed voxel volume and kappa. They do not match arbitrary rays, projected opacity or rendered brightness. Manual exposure snaps to nearest nodes. Animation interpolates both matched parameters and records the actual bracket; matching is approximate between nodes. This noisy implicit support front does not have proven constant normal speed.

One proof clock drives `e=100*min(t,120)/120`worldmetres. Zero-wind Weather advances to the same source time; scene.time=t and PDE steps=0. After phase120, E100 holds while actual clock time continues:16x12wallseconds reaches about192source seconds. E100 is below the full-extinction bound. The master/distance/noise remain frozen; the effective field changes. There is no independently moving fragment, independent exposure clock, fluid solution or full lifecycle claim. Pause retains continuous exposure without resnapping; idle has no perpetual render loop.

Actual submission tuples copy controls/assets/transform/camera/E/LUT/source time before awaiting GPU completion. Receipts distinguish requested clock, last submitted/completed frame and a separate post-record settled frame; browser display latency and delivered FPS are unmeasured. Requested capture15FPS is not measured delivery. Any fixture/camera change aborts the comparison. Historical173 clips are all `fixed-fixture`,1280x720,03/256,96rays,3kmoblique,Sun30deg,kappa.019/m; the retained receipts give actual wall/source endpoints.

## Selected observations and limits

[Manual native images and receipts](observations/manual/) retain E0 reference, SDF/alpha/subtraction E60, side pair, front E80 and noisy E100/front/high. [Full original WebMs and receipts](observations/motion/) are the motion authority. Their four native-crop contacts are navigational derivatives:

| Rate | SDF support | Matched alpha |
|---|---|---|
| 1x | [native ROI contact](observations/contacts/sdf-1x-native-roi.png) | [native ROI contact](observations/contacts/alpha-1x-native-roi.png) |
| 16x | [native ROI contact](observations/contacts/sdf-16x-native-roi.png) | [native ROI contact](observations/contacts/alpha-16x-native-roi.png) |

[Contact recipe](cook/contact-sheets.py) records FFmpeg9.0.2,source SHA,fps filter and native512x320crop at(384,200), with no resizing. Sampling uses FFmpeg's actual fps-filter semantics; tiles are not individually exact source-clock samples. The1xcontacts were regenerated deterministically with `fps=1/15`; unreconstructable earlier contact timing was not promoted. The16xfilter uses `fps=1`. Inspect original videos for motion; a contact alone cannot establish stable visible remnants, breakup, independent movement or cloud art acceptance.

Parent and independent review inspected the actual native images and decoded1x/16xframe contacts. SDF erosion contracts the outer footprint, deepens the central cleft and leaves unequal upper remnants; the matched-integral alpha keeps the original broad outline while becoming translucent. Density subtraction also changes support, with a different crenellated body. The distinction survives the matched native-integral control. This is a bounded geometry result, not a preferred whole lifecycle.

**Breakup art remains incomplete:** the persistent two-peak arrangement erodes in place; late thin horns/wisps do not establish independently drifting fragments. There is no attached lobe growth, mature replacement, shared wind response or Cb development. The final noisy clip is an instrument/control witness, not evidence that spatial noise alone solves breakup. Reviews inspected sampled decoded frames, not uninterrupted video playback; temporal cadence/flicker and normal-speed experienced acceptance remain open. No full family gate is closed.

Independent code review of the173 clock/recorder found terminal GPU-loss settlement and unbounded retained encoder chunks. Final81 fixes both; read-only follow-up reported no findings after failure and motion tests. The numerical GPU observer now gates its declared tolerance; actual final81 uniformE197.375/noisyE657.9167 endpoints both return zero sampled positive coefficients. These do not certify exact continuous-isosurface geometry or full native topology.

[GPU probe receipts](observations/probes/) verify actual scalar uploads and32³shared-coefficient samples. The historicalfd probe reported CPU/GPU errors but did **not** gate agreement on them. Builds173/81 gate a declared two-half-ULP-plus-float32 numerical envelope with propagated smoothstep bounds; this is an experimental tolerance, not a portable WebGPU guarantee. Unit-error injection is rejected by scoped tests. Positivity/raw-extinction/phase checks and sampled endpoint zero are separate gates. These sparse queries do not prove native256topology or exact projected/integral equality. The endpoint probe is distinct from the E0 probes; read actual controls rather than intended UI input.

The final81recorder retains at most12MiB encoded chunks, drops overflow chunks and saves prior output as explicitly aborted `encoded-size-limit`; first-chunk overflow reports capacity failure. Partial playback is not automatically validated; its encoded source endpoint is unmeasured. The server and final blob guard enforce the same limit. Device loss terminates current/future waits and recording, including a late fence. These final negative paths have mocked GPU/encoder tests and an actual HTTP413server check, not browser failure-injection admission. Historical173clips predate those fixes; their actual sizes are each below2.8MB, and no old failure path was exercised.

## Cost scope

The03/256actual probes charge78,528,524logical renderer bytes(~74.89MiB), including32MiBdensity,32MiBdistance and64KiBchannel noise, plus qualified renderer resources and small unused dummy/PDE/noise/curl infrastructure. Only one iso/quality is resident per selection. CPU input copies, hashing/swaps, staging and the development server's~114MBpayload snapshot add transient/host cost; process residency and browser-internal encoder buffering are unmeasured. The observer retires requested temporary readbacks; the one-shot lab relies on tab/device teardown for final disposal. No production per-cloud architecture, full-sky cost, frame-time or512MiB admission follows. Package disk cost and exact restoration results are in [the manifest](manifest.json), [check log](checks/recovery.txt) and [structured summary](checks/summary.json). After an intentional reviewed edit (such as final visual assessment), regenerate checksums with `python3 refresh-manifest.py`; use `verify.py` to check retained files without changing them.
