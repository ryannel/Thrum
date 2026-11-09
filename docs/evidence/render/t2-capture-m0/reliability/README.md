# T2-CAPTURE M0 reliability rework

Builder reclaim after the [second review](../../../T2-CAPTURE/M0-review.md) failed M0.3 and M0.8.
Target: this Apple M3 Max / Metal, 9 November 2026. Lead review is owed; no reviewer was launched.
[Runbook](../../../../CAPTURE.md), [all commands/exits](order.json), [measurements](summary.json),
[originals/retention](retention.json), [CPU diagnosis](cpu-diagnosis.json), [inspection](inspection.json).

The frozen release binary is `afbfd378083b97876e34389e5e7e7e39962a80b569174841ba74cc44db88a5f3`;
all integrated arms have build-source digest `8975eeb07e0a63723f363c6b258bc294b1324b3cbbf4a392a77b9b906aebac5d`.
Manifests retain individual file pins, recipe, shaders, actual generation/lead, script/environment and original
pre-encoding hashes. Measurement source was the dirty builder patch on `c65a90e6`; the integrated rebuild must
match this binary. Engine lead/provenance remain as accepted in the second review. Positive arms were locked
before/after capture with Screen Recording preflight `false`. No lock, unlock or permission request was made.

## Writer and repeated complete sessions

The writer still receives every full 1920×1080 RGBA record. It hashes the original, XORs with the previous record,
and uses Zstd level 1, two codec workers, 4 MiB window, checksum. A separate bounded I/O worker writes encoded
chunks to the inherited `F_NOCACHE` file. This avoids serializing disk waits with the next hash/codec operation.
The independent zstd CLI and Python XOR reconstruction check **every original RGBA hash, including alpha**.
This is a live lossless intermediate, not live PNG/movie packaging or reduced fidelity; the owner explicitly
allowed fewer disk bytes. PNG/preview packaging and archive verification run after the consecutive series.

| Workload, in recorded order | Result | Storage / headroom |
| --- | --- | --- |
| Five consecutive writer probes, before the integrated series | 5/5 × 480 full records, no missed IDs; all 2,400 originals independently verified | 581,288,893 encoded bytes each instead of 3,981,312,000; CPU leases peak 2–13/32; codec/write active wall 1.946–1.973 s, drain 9.08–10.60 ms, full sync 7.34–29.07 ms |
| Ten ordinary sessions | 10/10 complete, no omitted images or skipped opportunities | Pan, input stall, running world in rotation; no heavy jobs/packaging between sessions; total duration including drain ≤4.027 s |
| Twenty immediately following disk-capped sessions | 20/20 complete, no omitted images or skipped opportunities | Same rotation; strict 200,000,000 B/s maximum at the actual sink, without idle credit; total duration including drain ≤4.826 s |

All thirty retain **14,423/14,423** submissions/images: 119,630,131,200 raw bytes become 11,376,169,463 encoded
bytes. Ten pans require 578.5–580.9 MB/session (~145 MB/s average, ~6.9× fewer bytes); ten input stalls require
505.4–509.7 MB; ten running-grid sessions require 49.2–50.2 MB. No retry or failed positive session was discarded.
The probe fixture's SHA-256 is in `order.json`; all probe input hashes equal its retained decoded source records.
The helper is `thrum-app`'s `capture_spool` example, using the identical writer. Historical raw probes remain
historical failures/limits, not the new throughput claim.

The series ran with 32.84–44.44 GB free on the normally nearly-full data volume (96–97% full), load-1 1.89–3.40.
No user files were cleared. Per-session load, process snapshots, actual disk/CPU samples, write service, full
flush and verification time are retained in `sessions/<name>/`. First iostat row is since boot; later rows are
one-second host-wide rates, not isolated writer bandwidth. Capped writes also incur actual disk latency and wake
overshoot: observed moving-period disk0 samples around 118–166 MB/s in `series-12`, below the review's 280 MB/s
floor. Preparation reads/background traffic can give higher samples and are not attributed to capture writes.

The ~580 MB pan burst is concentrated in two moving seconds. Its ~180 MB backlog at a 200 MB/s sink motivates
**64 × 4 MiB encoded buffers (256 MiB)**, separate from six GPU / 96 CPU pixel leases (846,028,800 bytes under the
1 GiB *pixel* allowance), two full-frame codec scratch buffers, a 4 MiB sink buffer and bounded codec allocations. The encoded peak
was 63/64, but pixel peaks stayed **2/6 GPU and 2/96 CPU**, leaving 94 CPU leases behind codec backpressure;
this is clear admission margin, not a nearly-full raw queue. It is not a four-second raw session in RAM.
Measured peak process RSS: pan 286–537 MB, input stall 284–473 MB, running 225–239 MB; reserved pixel bytes and
RSS have different scopes. Full sync was ≤15.97 ms across the series. Qualification covers this workload/device;
new/noisier content and slower disks need measurement, and exhaustion still refuses continuous proof.

Failed alternatives are retained compactly in `pilots/` and [codec candidates](codec-candidates.json): raw/LZ4
still wrote too much, spatial prediction increased bytes, Zstd level 3 saved little, software SHA-256 overflowed,
and a 32 MiB encoded reserve overflowed at the strict cap. These are not kept passing samples.

## Why OFF varied, and the stable cost control

Actual active-thread clocks vary with wall clocks in the sleep-paced OFF runs: the spread was not primarily
waiting. IODeviceTree maps CPUs 0–3 to E and 4–13 to P. In `core-pilot/off-0`, same-endpoint P observations have
median 0.555 ms (216), E 1.894 ms (135), migrated 1.918 ms (129). Another arm's P group is slower too: core
placement explains much of the mixture, while frequency/cache/cluster state can still vary. These latter effects
are an inference, not a frequency measurement. Privileged `powermetrics` was unavailable (`sudo -n` needs a
password); no authorization was requested. CPU endpoint IDs do not guarantee continuous residency.

The capture writer creates sustained runnable work while OFF sleeps between small jobs. Requested interactive
QoS alone (actual 0x15→0x21) did not stabilize OFF; a scoped latency-critical process-activity pilot also failed.
That activity was removed. Both the failed mechanisms and all per-opportunity CPU/wall/core observations are
retained. We do not interpret the original ON/OFF inversion as an engine speedup.

The explicit `THRUM_CAPTURE_PACING=active` diagnostic keeps the same render owner runnable while waiting for
identical 120 Hz deadlines in **both** arms. It does no extra scene work or image capture. Its CPU burn is outside
the measured step/loop regions and inside whole-child CPU. The ordinary proof above still uses sleep pacing.
Matched order is OFF/ON/ON/OFF twice, same frozen binary, cooked recipe, camera schedule and QoS:

| Arm | Step active CPU median ms | Loop active CPU median ms | Scene GPU median ms | Copy GPU upper bracket ms | Whole-child CPU s |
| --- | ---: | ---: | ---: | ---: | ---: |
| cost-00 OFF | 0.518 | 0.520 | 2.646 | absent | 4.767 |
| cost-01 ON | 0.655 | 0.829 | 2.668 | 1.191 | 10.248 |
| cost-02 ON | 0.669 | 0.849 | 2.676 | 1.191 | 10.226 |
| cost-03 OFF | 0.516 | 0.518 | 2.643 | absent | 4.778 |
| cost-04 OFF | 0.521 | 0.524 | 2.656 | absent | 5.432 |
| cost-05 ON | 0.673 | 0.851 | 2.662 | 1.184 | 10.717 |
| cost-06 ON | 0.671 | 0.849 | 2.674 | 1.191 | 10.856 |
| cost-07 OFF | 0.526 | 0.529 | 2.655 | absent | 4.801 |

OFF step medians span only **0.516–0.526 ms** (~2%); no same-endpoint E samples occur. No core-group filtering was
used to obtain these medians. Against the surrounding OFF average in each block, ON adds **0.138–0.152 ms per
step** and **0.310–0.330 ms per loop**, including draining. All ON arms retain 481/481 exact images. Copy markers
measure the existing dependency-ordered upper bracket, not pure blit or whole-GPU time. Positive brackets remain
on every image; scene GPU is separate. The baseline is the uncaptured active-pacing arm, not a captured timing.

Whole-child CPU includes pacing, preparation, three-second warmup and drain; its remaining spread is retained,
not used for a precise observer percentage. The stable region measurements establish observer cost. ON peak RSS
285–287 MB versus OFF 85–86 MB. Separate matched `profile-off`/`profile-pan` samples expose codec, hash, row-copy,
write and wait work; sampled/blocked stack counts are not CPU percentages. This is substantial opt-in capture
cost, not an uncaptured native/display cadence or performance promotion.

## Fidelity, controls and retention

[Pan preview](views/pan-1x.mp4) and native full/cropped pairs were inspected with host-time records: coherent
rock/ridge translation, sphere exit, monotone accepted camera yaw, no cut/reversal or copy corruption. Initial
and final pan holds are 1.0106/1.0066 s; max image gap 12.13 ms. Still, fixed and all pan endpoints have identical
original hashes. Fixed mode remains synthetic. Running retains ticks 0–119, history epoch 0 and 113 strict
intermediate alphas in `series-02`; the 123 interior alpha-one samples remain exposed, owned by world/presentation
pacing, not fixed here. Native frames show consistent actor/grid movement without new history reset/copy trails.
Render-stall delay is 254.69 ms with a 260.05 ms image gap and recovery; input stall is 258.58 ms with fresh,
pixel-identical held submissions, then recovery. Host-time previews retain all source PTS (max error ≤0.5 µs).
Full previews/native originals for all four cases remain local; only one preview is promoted to avoid duplicates.

`writer-slow` with the compressed grid surprisingly completes: its small stream fits the encoded reserve. It is
retained, not claimed as a saturation proof. The explicit **raw** slow-writer control refuses: 479 continuing
submissions, 109 images, 370 ring omissions, 6/96 pixel peaks, ticks continue through the interval, writer deadline
refused and `complete:false`. No compression failure is hidden: compressed write, flush, byte-limit, changed-size,
truncation/hash/extra-byte and round-trip failures are automated; slow raw admission/deadline tests remain.

Each session has a 16 GiB output/free-space allowance. Bulk `frames.zst` originals, exact binaries, selected full
PNG sequences and host-time previews remain in `tmp/t2-capture-m0-r3/qualified/` **through the next lead review**.
They are not a durable backup. Gzip metadata is byte-for-byte reversible; selected hashes and restoration paths
are in `retention.json`. The raw negative-control partial spool is not a source master. After source verification,
80.71 MB of historical duplicate pictures/movies were retired with exact hashes/Git restoration in
[historical retirement](historical-retirement.json); the old measurements and review remain. New selected views
are 13.99 MB. Cargo outputs are reconstructible; frozen binaries are outside `target/` before cleanup. This
worktree stays because it contains the retained review roots. Superseded pilot spools (10.63 GB) were removed
with original metadata retained; see `pilot-cleanup.json`. Renderer timing failure still invalidates capture,
and the existing device-wide mappable-buffer feature scope is unchanged (second-review minor F4).

`recipes/` preserves the actual qualification/continuation/analysis scripts; `order.json` is the actual outcome
record. The first script stopped when the weak compressed negative control did not saturate; the continuation
records explicit raw saturation and separate profiles. Use new output directories when rerunning. Runtime and
manifest gzip files can be read with `gzip -dc`. No window, compositor/panel, EDR, physical-device, Chrome or
simulator qualification follows. Mac M2.3/M2.4 and attended panel M2.2 remain with their existing owners.

Full `./dev check` and six real-Metal `submitted_capture` tests with `creation-counters` pass before commit, under
`gpu`; gzip logs are retained here. Integration requires another fresh Check after rebase while holding
`integrate` and `gpu`, exact qualified-binary equality, a non-force push and ancestry confirmation.
