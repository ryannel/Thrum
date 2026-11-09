# T2-CAPTURE M0 independent milestone review

Reviewer: Claude Opus 5.5 (claude-opus-5-5), independent of the Codex builder, 9 November 2026.
Claimed commit: `770bb4ef` (base `5385aa64`). Target: mac-metal, Apple M3 Max, console locked throughout
(`IOConsoleLocked = Yes` before and after every run), Screen Recording preflight `false`, no grant requested.

**Verdict: FAIL M0: 3 fail, 0 cant_tell, 0 outside the items.**
M0.3, M0.4 and M0.8 fail. M0.1, M0.2, M0.5, M0.6, M0.7 and M0.9 pass.

## What I ran (verified by running)

All GPU work ran serially under `agent_loop.py lock gpu`, from the clean worktree at `770bb4ef`.

- `./dev check`: exit 0 (4 min 28 s). It includes the six `submitted_capture` real-Metal tests, the readback
  odd-width test, both writer tests and `test_t2_capture.py` (4 tests).
- I rebuilt `target/release/v1-game`. Its SHA-256 `b98b8740…dcc1b4ea` is identical to the builder's manifests,
  so the retained evidence and my runs use the same binary. I cooked a fresh store in `tmp/t2-review-m0/valley`.
  Its product hashes match the builder's; only the store bookkeeping files (`EPOCH`, `HEAD`, `stat-cache.bin`) differ.
- I ran the runbook's capture command for the pan (five times), running, render-stall, input-stall, writer-slow,
  fixed, still and off cases. All runs were locked and reported preflight `false`.
- I ran the writer probe `scripts/t2_writer_probe.py <new dir> --uncached` three times.
- To check legacy parity, I built the base commit `5385aa64` and ran its route and endpoint captures. I then ran
  the same captures with the claimed binary, using the same store and route.

Results that pass:

| Case | My result |
| --- | --- |
| Pan, realtime (run 2, used for inspection) | complete; 481 opportunities, 481 submitted, 481 images; 0 ring/writer omissions; tick 0 throughout; IDs contiguous; largest submit gap 12.32 ms; 241 distinct pixel states (121-frame initial hold, 239 single-frame motion states, 121-frame final hold); accepted yaw 0 → 35.99999°, no reversal; preview PTS error 0.5 µs |
| Pan endpoints | initial and final hashes are identical to the builder's pan, to my still and to my fixed-time run |
| Fixed-time pan | all 481 frame hashes identical to the builder's retained fixed run (deterministic) |
| Running world | complete 481/481; ticks 0–119; 218 intermediate-alpha samples; alpha never regresses within an endpoint pair; history epoch constant 0 |
| Render stall | logged delay 258.2 ms; image gap 266.4 ms in the records and in `timeline.ffconcat`; 31 opportunities skipped as "driver busy"; then one catch-up jump and normal motion |
| Input stall | logged dispatch delay 256.3 ms; 32 identical fresh submissions spanning 267.6 ms while rendering continues at 120/s; then recovery |
| Writer slow | explicit incomplete manifest (exit 1); 376 ring omissions; 104/480 images; ticks 0–119 continue; 480/481 submitted (one optional-view refusal); peaks 6 staging/32 CPU; finalized in 14.0 s |
| Off | 480/480 submitted, no tap usage, no pixels |
| Legacy route, pre vs post migration | all five checkpoint PPMs, `route.jsonl` and the noon endpoint PPM are byte-identical between the base binary and the claimed binary. The endpoint also matches the historical `parity-20261109.json` hash `d7d3d2fc…` |

Results that do not pass:

| Case | My result |
| --- | --- |
| Pan, realtime run 1 | **overflowed**: 57 ring omissions, 423/480 images, writer 4.33 s; correctly refused as incomplete |
| Pan, realtime runs 3–5 | complete; writer time 1.90, 1.86 and 3.11 s (builder's run: 1.24 s) |
| Writer probe | **2 of 3 runs overflowed** (missing IDs 380/386/393, then 417; queue peak 7/8, CPU peak 8/8). One run passed (queue peak 4, packaging 8.77 s) |

Observer cost (my pan ON against my off run, same binary): render CPU median 0.703 against 0.573 ms (+22.6%);
scene-pass GPU 2.689 against 2.668 ms; peak RSS 204.0 against 81.5 MB.

## Findings, worst first

### F1. The writer does not reliably sustain the required workload (M0.3 fails; verified)

The probe overflowed in 2 of my 3 runs on the target disk, with no other GPU or capture job running. The
integrated pan overflowed in 1 of 5 runs. Writer service time in the integrated runs varied from 1.86 s to 4.33 s
for about 4 GB. M0.3 requires a writer that "sustains" the four-second, 120 records/s workload "without overflow".
The retained evidence is the one passing probe run (`writer-result.json`). One pass out of several attempts shows
the writer can work, not that it sustains the load on this machine.

The failures are honest: every overflow is refused as incomplete, so this is not fake evidence. However, the
workflow later milestones depend on will fail some fraction of runs on an ordinary, in-use owner Mac. Load average
was about 2–3 and disk activity was light. A 0.5 s fsync stall consumes the whole queue margin at 1 GB/s.

Reproduce: run `python3 scripts/t2_writer_probe.py tmp/<new> --uncached` under the gpu lock three times. Then
compare `missing_ids` and `queue_peak` across the runs.

To pass, the evidence must show the probe and the integrated pan completing repeatedly, for example 5 of 5, with
the rates recorded. The other way to pass is to revise the writer (deeper bounded queue, larger write units, or a
storage-throughput preflight that refuses before the run) and record the measured headroom. Do not shrink the workload.

### F2. Headless real-time driver omits the render actor's presentation lead (M0.4 fails; verified by reading)

`crates/thrum-app/src/offscreen.rs:164-180` calls `FrameHost::drive` directly from its own thread and never calls
`set_presentation_lead`, so every capture samples interpolation with lead 0. The windowed `render_actor.rs:273-275`
sets the lead from the display target on every frame.

The design (§Mechanism) says the headless adapter "replaces only the OS opportunity/surface source". It says the
render actor gets "actual elapsed time and declared lead", and that render-actor admission is retained. Here the
lead is neither declared nor recorded in `manifest.json` or `runtime.json`. As a result, M0.6's interpolation
samples are not the windowed path's alpha, and nothing in the evidence says so.

The fix is to declare and record a lead, ideally the same `display_target_lead` policy the windowed path uses, or
to label the zero lead as an explicit limit. Then show alpha with that lead.

### F3. Per-frame device and surface generations are hard-coded constants (M0.4 fails; verified by reading)

`examples/v1-game/src/window/capture.rs:73` writes `"device_generation":1,"surface_generation":1` literally into
every image record. `metal_device()` also passes generation `1` to `DeviceCapabilities::capture`. Neither value is
read from the submission or the tap.

The design (§Completeness) requires per-submission "content/device/surface generations". M0.4 asks the reviewer to
inspect per-frame provenance. A record that always says 1 cannot show a generation change, so it is canned
provenance. In today's driver a loss aborts the run, so the value is true by construction, but the record does not
measure it. The fix is to expose the tap or sample generation on `SubmittedImage` and record it, or to drop the
fields and state the limit.

### F4. Observer-cost records omit load, and the main observer GPU cost is unmeasured (M0.8 fails; verified)

M0.8 asks for paired results "with run order and load". Run order is recorded (`proof-summary.json`
`observer_order`). Neither the manifests nor the proof summary record system load or background activity. The cost
also excludes the readback copy's GPU time (the README says so). That copy is the main GPU work the observer adds.

The CPU increase is +24.5% (builder) and +22.6% (mine), against the design's roughly 25% investigation tripwire.
The README explains it by reasoning but includes no profile.

The fix is to record load, such as load average and disk activity, for each arm. Also time or bound the copy pass,
or explicitly scope GPU cost to the scene pass plus an unmeasured-copy limit agreed in the item. Investigate the
CPU delta with a profile rather than an argument.

### F5. Lower-severity issues (do not fail items on their own)

- **Legacy parity evidence is weaker than claimed (verified).** `legacy-parity.json` compares two post-migration
  runs with each other, which proves determinism, not parity. My pre/post run shows parity does hold.
- **The history test is narrow (read).** `submitted_tap_matches_the_actual_odd_width_surface_and_preserves_history`
  only checks that the `submitted_view_history_epoch` counter is unchanged, not history content with capture on
  and off. The pre/post route checkpoints, which are mid-sequence and history-dependent, cover this in practice.
- **The readback test uses one uniform clear colour (read).** `metal_readback_preserves_rgba_channels_and_removes_odd_width_row_padding`
  would not catch a row-offset error that keeps the length. The grid-scene tap comparison covers spatial content.
- **The disabled zero-allocation test is narrow (read).** It times only `reserve(None)`, not a whole disabled
  `drive`. The disabled hook code is a `None` branch, so the risk is low.
- **A failed map can leave a slot reserved (read).** In `submitted_capture.rs`, `try_submitted_image` on a map
  error leaves that slot `reserved` with its sample set, permanently, for the session. The session is incomplete
  anyway, but the error path should release or retire the slot.
- **Stall timing records (read).** The stall controls live in the V1 example driver, not as engine scenario
  actions. `requested_ns == start_ns` by construction. The input stall drops intermediate edits and then applies
  the newest value, rather than delivering queued input late. The behaviour is acceptable for M0.7, but the record
  should say so.
- **Engine-side observation, not a T2 defect (verified).** In my running-world capture, submission 459 clamps to
  alpha 1.0 at tick 40, and the next sample jumps a full tick (diff 0.79 against a median of 0.39). There were 19
  alpha-1 samples among 236 running samples. The builder's run had 27. The capture shows this honestly. It belongs
  to the world-presentation pacing owner (Mac M2), not to T2.
- **Minor documentation issues (read).** The README says 206 intermediate-alpha samples, but its own
  `running/runtime.json` has 209. The runbook's lock command uses an absolute path in the user's home skill
  directory. Duplicate committed PNGs (the same initial, final and crop images under still, fixed, pan and stall
  folders) add about 10 MB to the 63 MB evidence folder.

## Item verdicts

- **M0.1 pass.** I traced the path: `FrameHost` `encode_contributed(..., Some(readback))` adds a graph Copy pass
  from `target.ids.output` in the same encoder and submission, and `map_buffer_on_submit` maps it. There is no
  endpoint redraw, second scene render, private queue submit or drawable use. I ran the real-Metal RGBA/BGRA
  odd-width correspondence test, which compares against an independent copy of the presenter output, and it
  passed. The disabled path is a `None` branch with no graph Copy pass, as the test shows. The endpoint semantics
  are unchanged (still equals the endpoint hash). Pre/post route checkpoints are identical, which shows submitted
  history is not perturbed. The Mac M1 loss tests pass in the Check. The tests `expect` a device, so they fail
  without hardware. Weak tests are listed in F5.
- **M0.2 pass.** I ran the loss/replacement, abandon, stale-session, saturation and resize tests, the writer
  failure and deadline tests and the Python refusal and hash tests in the Check. My slow-writer run gave an
  explicit incomplete result with bounded 6/32 peaks while ticks and rendering continued. Decoded PNG and archive
  hashes are checked by the packaging. The slot-retention-on-error issue is in F5.
- **M0.3 fail.** See F1.
- **M0.4 fail.** See F2 and F3. Everything else in the item checked out: locked console and preflight `false` in
  every run, adapter, backend, recipe and hash provenance, ordinary `SurfaceOutput` admission, paused-world fresh
  submissions at tick 0, controls through `apply_file` and `Game::apply`, complete coverage and finalized
  manifests, and fixed and endpoint parity.
- **M0.5 pass.** On run 2, every entry in the inspection list holds (below). Run 1's overflow was refused as
  inconclusive, as the item requires. That reliability problem is charged to M0.3.
- **M0.6 pass.** Intermediate alpha is present and monotone within endpoint pairs, the history epoch is constant,
  there is no reset or smear in native crops, and the holds are static. The missing presentation lead is charged
  to M0.4.
- **M0.7 pass.** Both stalls hold for their logged elapsed durations in the records and the PTS, then recover.
  Fixed mode refuses stall scenarios. Ordinary schedules have empty `diagnostic_actions` in every one of my pan,
  running, off and fixed runs.
- **M0.8 fail.** See F4.
- **M0.9 pass.** I ran the runbook's build, cook and capture commands, and they reproduce the cases. It states
  capture modes, provenance, limits, the 16 GiB allowance, free-space preflight, retention, the M2.2, M2.3 and
  M2.4 owners and the Chrome and simulator interim scope. It makes no swap, panel or photon claims. The minor
  issues are in F5.

## Inspection lists (written before viewing any frames)

**M0.5 pan:**

- P1: The initial hold is identical frames at yaw 0, and equals the still.
- P2: Every turn frame is a new state, with no repeated runs and no large submit gaps.
- P3: The rock and ridges translate consistently, and the sphere sweeps out of frame.
- P4: There is no cut or snap, so per-frame differences are bounded and have no spikes.
- P5: The final hold is identical frames at 36°, and equals the fixed-time final frame.
- P6: The world tick stays at 0, with fresh submission IDs.
- P7: Native mid-turn crops show no smear, ghosting or history artifact.

Results on my run 2:

- P1, P5 and P6: hold. These were checked by hash and by the records.
- P2: holds. There are 239 single-frame interior runs, and the largest gap is 12.3 ms.
- P3: holds. In my contact sheet, the sphere goes from x≈190 to the edge and leaves frame by 2.5 s, and the rock
  goes from x≈290 to x≈70 of 480. Both move monotonically left.
- P4: holds. The per-frame mean absolute difference, measured on the green channel with 1/8 subsampling, ranges
  from 1.26 to 1.90 across all 240 moving transitions, with no outlier.
- P7: holds. I checked native 900×500 crops of submissions 499 and 500: the sphere, rock and dune edges are clean
  and shift about 3 px per frame, with no trails.

**M0.6 running:**

- R1: The initial hold is static.
- R2: The movers advance between consecutive frames.
- R3: Alpha is in (0, 1) and monotone within an endpoint pair.
- R4: The history epoch is constant.
- R5: There are no jumps at endpoint boundaries.
- R6: The final hold is static.

Results: R1, R3, R4 and R6 hold. R2 holds in the contact sheet and native crops of 456 and 457 (lower grid
descends, bob visible). R5 holds except for one alpha-clamp hold-then-jump at submission 459/460, which is an
engine pacing observation (F5).

**M0.7 stalls:**

- S1: The render gap equals the logged delay in the images and PTS, then motion resumes.
- S2: Fresh renders continue at 120/s with identical pixels for the logged delay, then motion recovers.
- S3: Ordinary runs have no diagnostic actions.

All three hold, with the figures given above.

## Proposed item hardening (not applied; definitions unchanged)

- M0.3 or M0.5: require N≥5 consecutive complete integrated pans (or probes) with recorded writer rates. A
  single passing run should not close a throughput item.
- M0.4: require per-frame generations and the presentation lead to be read from the engine and recorded, not
  supplied by the driver.

## Retained reviewer scratch

`tmp/t2-review-m0/` (git-ignored, 92 MB) holds my manifests, runtime records, previews, contact sheets, crops and
diff profiles. Raw spools, PNG directories and archives were deleted after analysis. It is kept through the
builder's rework. Delete it after the next verdict.

FAIL M0: 3 fail, 0 cant_tell, 0 outside the items
