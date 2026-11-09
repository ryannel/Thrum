# T2-CAPTURE M0 evidence — 9 November 2026

Builder verification complete; M0 is claimed for independent lead review. No reviewer was launched.
Commands and limits: [capture runbook](../../../CAPTURE.md); [storage record](STORAGE.md).

Inspection checklist, fixed before final visual review: inspect native initial/final frames, full 1× host-time
sequence, contact sheet and native crops. For the cooked pan, check coherent rock/ridge placement, the authored
sphere sweep and exit, settled yaw-zero/36° endpoints, and no cut, snap or reversal; locate repeats/holds/gaps
against host time. For running motion, check visible actor continuity across adjacent 60 Hz endpoints and
intermediate-alpha samples without history reset. For diagnostics, check the actual render gap and the separately
held input response last for their logged delays and recover. Neither a target PNG match nor panel smoothness
is an acceptance criterion here. Fixed-time output cannot prove elapsed runtime motion.

The writer feasibility probe retained [measurements](writer-result.json), [write timings](writer-writes.json)
and [decoded frame hashes](writer-hashes.json). The exact retained script hash matches `scripts/t2_writer_probe.py`.
The first cached-write probe missed 79/480 records. Darwin uncached sequential writes with eight CPU buffers
retained 480/480 full 1920×1080 records at 120/s: queue peak 2, CPU lease peak 3, 3,981,312,000 raw bytes,
25.016 ms drain including 16.387 ms full storage flush, then 9.128 s deferred PNG packaging/hash validation.
The fixture's extra 8.29 MB pattern and bounded packaging temporaries are separate from its 66.36 MB CPU pool.
This proves writer feasibility on the target disk, not GPU/runtime motion.

Integrated three/eight buffering missed samples; direct-file output removed a pipe bottleneck. Six staging slots
and 32 reusable CPU buffers use 315,187,200 pixel bytes under a 512 MiB allowance. A later capture during a release
build overflowed this profile too; it is retained locally as an explicitly incomplete, contended diagnostic. Final
qualification/cost runs are serial and quiet, under `gpu`, with the same unchanged fidelity and binary.

The first full Check found an unmap of an unsubmitted buffer. Dropping the encoder cancels its deferred map;
there is no active map to unmap. The correction and its regression pass. The next full Check passed. The current full Check passes; [verification record](verification.json) pins its log. A fresh integrated-tip Check
is required under the integrate lock before the non-force main push.

Full local source archives/time records remain under `tmp/t2-capture-m0/` through lead review. Selected native
frames/crops, full 1× previews, hashes and timelines are promoted here after archive/PNG decode verification.
Each case's `retention.json` names its local archive, exact selected hashes and expiry. A local bulk archive is
not a durable backup; no remote/LFS store is configured. Owned raw/PNG duplicates are removed only after verified
source-archive and selected-root retention. Default allowance remains 16 GiB per serial session.

## Target results

All finalized positive cases report Apple M3 Max/Metal, locked before and after capture, and Screen Recording
preflight `false`, without requesting a grant. All use binary SHA-256
`b98b874068e6fb547f2947c612dfb73f741f6807739d3272edda9935dcc1b4ea` (see each manifest for the authoritative hash).
[Machine-readable summary](proof-summary.json) checks same-build identity, complete joins, mode parity and controls.

| Case | Submitted/source images | Result and retained proof |
| --- | --- | --- |
| Cooked elapsed pan | 481 / 481 | [Manifest](pan/manifest.json), [controls/times](pan/runtime.json), [1× preview](pan/preview-1x.mp4), [contact](pan/contact.png), native initial/final frames and crops |
| Running 60 Hz world | 481 / 481 | [Manifest](running/manifest.json), [endpoint/alpha/history records](running/runtime.json), [1× preview](running/preview-1x.mp4), native frames/crops; ticks 0–119, 206 intermediate-alpha samples, history epoch 0 |
| Render stall | 450 / 450 | [Records](render-stall/runtime.json), [1× preview](render-stall/preview-1x.mp4); 255.929 ms actual delay, 264.619 ms image hold/gap; zero capture omissions |
| Input stall | 481 / 481 | [Records](input-stall/runtime.json), [1× preview](input-stall/preview-1x.mp4); 250.229 ms actual dispatch delay, 30 fresh submissions with one accepted yaw and one identical pixel hash, then recovery |
| Slow writer | 480 / 103 | [Incomplete manifest](writer-slow/manifest.json), [continuing app records](writer-slow/runtime.json); expected exit 1, 377 explicit ring omissions, ticks 0–119, GPU/CPU peaks 6/32, bounded finalization in 14.012 s |
| Fixed-time pan | 481 / 481 | [Manifest](fixed/manifest.json), [synthetic times](fixed/runtime.json); initial/final pixel hashes exactly equal the elapsed pan, no elapsed-motion claim |
| Still | 1 / 1 | [Manifest](still/manifest.json), [native frame](still/initial.png); pixel hash exactly equals fixed/elapsed initial frame |

The cooked pan has 1.013 s initial and 1.010 s final source holds, tick 0 throughout, no skipped opportunities or
capture omissions, 241 distinct pixel states (motion plus legitimate repeated holds), and a largest submission gap
of 12.370 ms. Accepted endpoints are 0° and 35.999989° after normal f32 slider quantization. Preview PTS differ from
host submission records by at most 0.5 µs. Native full frames/crops and the sequence/contact views show coherent
rock/ridge placement, the authored sphere exit and settled endpoints, with no cut, reversal, anomalous local hold
or history smear. Running-grid views show continuous mover/bob evolution and no history discontinuity. The two
explicit stalls hold/delay content for their logged durations and recover. Source archive index-time playback would
hide the render gap; only the host-time preview/time records prove its duration. No display-smoothness verdict follows.

The inspection used the declared checklist, native full frames and crops, and sequence views with host-time records;
the retained 1× clips are independently PTS-checked. No target-image quality matching or owner approval was used.
The running fixture includes the ordinary 2,000-mover grid and authored diagnostic camera bob; it is not cooked-scene
artistic qualification. In that case `control_dispatched=false` means the unused yaw candidate is not an applied edit.

[Legacy parity](legacy-parity.json): two native fixed-time route replays each retain 361 samples at phase 1/2;
all five checkpoint PPM pairs are byte-identical after migration to the renderer tap. The endpoint route remains at
world tick 0. Real-Metal tests also compare tap pixels with actual RGBA and BGRA presenter outputs at odd width,
including independent known-channel/padding controls. An archive extraction was restored to native PNG and its
decoded RGBA exactly matched the retained initial source hash.

## Observer cost and bounds

Run order was capture ON, OFF, OFF, ON; the final reverse pair includes exact-child Darwin `wait4` resource accounting.
[Capture OFF](cost-off/manifest.json) and [ON](cost-on/manifest.json) use the same binary, recipe, scene and schedule.
The observations below are baselines, not performance gates or uncaptured cadence qualification.

| Observation | OFF | ON |
| --- | ---: | ---: |
| Render CPU median | 0.646 ms | 0.805 ms |
| GPU scene-pass median | 2.677 ms | 2.677 ms |
| Engine process peak RSS | 84,230,144 B | 162,971,648 B |
| Raw source bytes | 0 | 3,989,606,400 B |
| Full storage flush | 8.832 ms | 10.842 ms |
| Total deferred packaging/verification | 0.000051 s | 67.005 s |

The ~24.5% CPU increase is near the investigation tripwire. The enabled path adds graph copy/map recording,
completion callbacks and bounded staging bookkeeping; CPU image collection additionally copies the charged bytes
outside `FrameSample.render_cpu`. There is no second scene render or queue. Both arms account for all scheduled app opportunities and bounded queues: OFF submitted 480/481 views and
explicitly reported one optional-view admission refusal; ON submitted 481/481. No capture omissions or unexpected
scaling occurred. GPU figures are **scene-pass sums**, excluding present
and the uninstrumented readback copy; they are not a whole-GPU span or a zero-copy-cost claim. Map-observation lag
and actual host intervals include the observer path and remain separately recorded. RSS is an OS process observation,
not a physical GPU-memory account or a hard process cap. The 315.19 MB pixel allowance is a separate worst-case
owner charge; lazy buffer commitment and other engine resources make it different from RSS.

The ON writer serviced 481 full records in 1.254 s aggregate write time, followed by the explicit storage flush;
compression/archiving/movie work ran only after capture. Peak session output stayed below the 16 GiB allowance.
All decoded raw/PNG/archive hashes and movie frame/PTS coverage passed. Every ordinary schedule has an empty
diagnostic-action list. Saturation is an honest incomplete outcome, not a visually passed interval.

Writer probe reproduction: `python3 scripts/t2_writer_probe.py tmp/t2-capture-m0/new-writer-probe --uncached`
under the profile GPU lock. Target cases use the runbook command with a new output directory and the table case/mode;
`running` and `writer-slow` omit `--store`. Exact source/cooked/shader/binary/scenario identities remain in manifests.
