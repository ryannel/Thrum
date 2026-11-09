# T2-CAPTURE M0 independent milestone review (third claim)

Reviewer: Claude Opus 5.5 (claude-opus-5-5), independent of the Codex builder, 9 November 2026.
Claimed commit: `1fa0a23c` (rework of `6be1b11b`, whose second review at `c65a90e6` FAILED M0.3 and M0.8; the
first review is at `2f089669`). Target: mac-metal, Apple M3 Max. Console locked in every run
(`console_locked` and `console_locked_after_capture` true), Screen Recording preflight `false`, no grant requested.

**Verdict: PASS M0.** All nine items pass. No blocking defect outside the items. Advisory findings are listed below.

## What I ran (verified by running)

All GPU and capture work ran serially under `agent_loop.py lock gpu`, from the clean worktree at `1fa0a23c`.
Nothing else of mine ran during captures. Load-1 was 1.9 to 5.8 (other agents active); the data volume was 97% full
(about 34 GB free at the start, down to 18 GB during the series).

- `./dev check` (the profile's Check): passed in 4 min 26 s, no failing test. It includes the six real-Metal
  `submitted_capture` tests, the three writer tests (raw/compressed failure, round trip, changed record size) and
  the five Python capture tests (`test_t2_capture.py`, also rerun on their own).
- Rebuilt `target/release/v1-game`: SHA-256 `afbfd378…88a5f3`, identical to the builder's frozen binary; build
  source digest `8975eeb0…` and an empty dirty diff in every manifest. Fresh cook in my scratch store.
- **30 back-to-back ordinary sessions** (10 pan, 10 input stall, 10 running, rotating, `--defer`, uncapped), then
  `--finalize --verify-only` on every one: the independent `zstd` CLI plus XOR reconstruction matched every
  original writer SHA-256.
- Margin probes: pan and input stall with the sink capped at 150, 120, 100 and 80 MB/s; two pans each under 7 and
  14 busy-loop CPU hogs (14 = every core).
- Runbook cases with full packaging: still, fixed, refused `--case running --mode fixed`, render stall, running,
  pan, raw `writer-slow` (expected refusal) and compressed `writer-slow`.
- Cost arms: active pacing OFF/ON/ON/OFF twice, then sleep pacing OFF/ON/ON/OFF.

## Results

### M0.3 throughput and margin

| Run | Result |
| --- | --- |
| 30 ordinary back-to-back | **30/30 complete**, 14,425/14,425 images verified, zero ring/writer omissions, zero skipped opportunities. Max image gap 12.0–13.0 ms. Pixel-lease peaks 2/6 GPU and 1–2/96 CPU; encoded-buffer peak 1–13/64. Disk I/O time 0.36–0.76 s per 4 s session for 505–581 MB (pan/stall) and 0.03–0.05 s for 50 MB (running). Writer thread busy 1.94–2.02 s per session |
| Cap 150 MB/s | pan complete (CPU leases peak **67/96**, encoded 64/64); stall complete (1/96) |
| Cap 120 MB/s | **pan refused** (13 ring omissions, 96/96); stall complete (40/96) |
| Cap 100 MB/s | pan refused (28 omissions); stall complete (85/96) |
| Cap 80 MB/s | pan refused (46); stall refused (15) |
| 7 busy loops | 2/2 pans complete, CPU leases 1–2/96 |
| 14 busy loops (all cores) | 2/2 pans complete, CPU leases 33–35/96, one skipped opportunity (driver busy, not a capture omission) |

Every refusal was explicit (`complete: false`, exit 1, about 14 s bounded drain) and rendering continued.

**Measured margin.** Disk: the pan needs a sustained sink between 120 and 150 MB/s; in normal conditions the
sink was busy at most 19% of the interval (effective ~0.8–1.6 GB/s), roughly **7× headroom**. Against the worst
sustained dip the second review observed on this volume (280–705 MB/s for four seconds, which overflowed the raw
writer), headroom is about **1.9×**. CPU: the writer thread spends about 4.1 ms per 8.33 ms record (**~2×**), and
the pan still completed with every core saturated by other work. The margin is clear in normal conditions. It is
specific to this content: the pan writes ~580 MB per session, the running grid 50 MB (see A2).

### Frame exactness after the format change (verified)

- All **481 fixed-mode hashes are identical** to the fixed run from the second review, which used the old raw
  spool and a different binary (`0e77c57d…`). The still equals the fixed initial frame.
- All 11 of my complete real-time pans share one (initial, final) hash pair, equal to the old raw-spool pans and
  the fixed endpoints.
- Every frame in every complete session (normal, capped, stressed, cases) decodes through the independent
  `zstd` CLI to its original pre-encoding SHA-256; packaged PNGs round-trip to the same hashes.

### Other cases

| Case | Result |
| --- | --- |
| Still / fixed | complete; parity above |
| Fixed running | refused: "running-world and diagnostic controls require actual elapsed time" |
| Render stall | 450/450; delay 260.0 ms; image gap 267.6 ms; 31 skipped opportunities, then motion resumes |
| Input stall (n01) | delay 250.2 ms; 30 fresh contiguous submissions inside it, one distinct hash; max gap 12.2 ms |
| Running | 481/481; ticks 0–119; history epoch 0; 110 strict intermediate alphas among 235 interior samples, no alpha regressions; 125 alpha-one clamps and 7 pixel repeats (A5); lead 0–8.31 ms; preview PTS error 0.5 µs |
| Writer slow, raw | refused: 109/473 images, 364 ring omissions, peaks 6/96, ticks 0–119 continue, "raw writer finalization timeout" |
| Writer slow, compressed | completes (its 50 MB stream fits the encoded reserve), as the runbook says |
| Ordinary runs | `diagnostic_actions` empty in pan, running, still and fixed |

### M0.8 cost arms

| Arm | Step CPU median ms | Loop CPU median ms | E-core samples | Copy bracket median ms | Whole-child CPU s | RSS MB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| active OFF a0 / a3 / a4 / a7 | 0.507 / 0.524 / 0.525 / 0.523 | 0.509–0.528 | 0–1 of 480 | – | 4.75–5.40 | 85–86 |
| active ON a1 / a2 / a5 / a6 | 0.650 / 0.664 / 0.664 / 0.660 | 0.830–0.846 | 0–1 | 1.181–1.192 | 10.21–10.94 | 286–290 |
| sleep OFF s0 / s3 | **1.724 / 0.853** | 1.738 / 0.860 | **385 / 163** | – | 2.63 / 1.57 | 85–86 |
| sleep ON s1 / s2 | 0.658 / 0.681 | 0.831 / 0.854 | 0 | 1.182–1.183 | 7.24 / 6.58 | 285–286 |

This reproduces the builder's diagnosis. The noisy sleep-paced OFF baseline comes from the render owner landing on
efficiency cores (CPUs 0–3) when the process is otherwise idle; the ON arms stay on performance cores. With active
pacing, OFF spans 0.507–0.525 ms (3.5%), and capture adds 0.13–0.16 ms per step and 0.30–0.33 ms per loop, plus a
positive copy bracket on every image. Whole-child CPU roughly doubles (about 5 s of extra CPU per four-second
session, mostly hashing, XOR and Zstd on the writer and codec threads), and RSS rises by ~200 MB. The README states
these scopes without a native-cadence claim.

## Findings, worst first

No blocking findings.

### A1. The governing design still says no compression during the interval (advisory; read)

`docs/design/modules/T2-CAPTURE.md` step 4 (lines 60–62) says the writer "appends raw pixel records" and "No
compression runs during the measured interval"; lines 119, 129 and 177 still describe the raw spool and its ~3.98 GB
budget. The implementation now compresses live (XOR + Zstd on two workers), and the runbook and plan say so, but the
design was not updated. The change is the fix the second review invited, it is lossless, and M0.3's ask does not
forbid it, so I do not fail M0.3 on it. It is still drift between the design and the code, and the design's reason
for the rule (keeping work off the critical path during capture) now rests on the M0.8 measurements instead. The
design owner should update step 4 and the feasibility section. The owner may promote this finding.

### A2. Throughput depends on how well the content compresses (advisory; verified for the M0 workload only)

The pan writes ~580 MB and the grid 50 MB, against 3.99 GB raw. The pan already needs 120–150 MB/s sustained.
Full-screen high-frequency motion (sea, clouds, foliage, grain) will compress far worse, and later visual
milestones consume this workflow. `docs/CAPTURE.md` says so, and qualification covers this workload only. Each
consumer should rerun a back-to-back series on its own content before relying on capture.

### A3. A mid-stream disk error is reported as "encoded I/O worker stopped" (advisory; read, not reproduced)

In `crates/thrum-app/src/offscreen/writer.rs` lines 120–121, `result?` is evaluated before `io?`. When the I/O
worker's `write_all` fails during a session, `Output::dispatch` returns the generic "encoded I/O worker stopped",
and that error wins over the worker's real error. The session is still refused. Only the diagnosis is lost. The
existing test covers only a failure that surfaces at finish.

### A4. Capture changes the render thread's scheduling class (advisory; verified)

`capture.rs` defaults the render owner to user-interactive QoS (recorded 0x15→0x21) in both OFF and ON arms. The
windowed game does not do this. This is recorded and matched across arms, but it means a captured run is scheduled
differently from the ordinary app. Consumers comparing captured timing with windowed behaviour should know this.

### A5. Existing items carried forward (advisory)

- Running-world interpolation clamps on about half the interior samples (125/235, with 7 pixel repeats). This is
  world/presentation pacing, outside T2 (second review F2).
- Second-review F4 items are unchanged: a timer error still drops the image, and `MAPPABLE_PRIMARY_BUFFERS` and the
  `STORAGE` readback usage are still device-wide.
- The runbook's lock path `$HOME/.claude/skills/build/scripts/agent_loop.py` does not exist on this host now (the
  script lives in the orchestrate plugin cache). Its other commands ran as written.
- The 1024-record session cap refuses intervals longer than ~8.5 s at 120 Hz; this is documented.

## Item verdicts

- **M0.1 pass.** The renderer tap is unchanged since the second review. The real-Metal correspondence, odd-width,
  disabled-zero-allocation, loss and history tests pass in the Check. Frames remain exact after the new format.
- **M0.2 pass.** Writer failure, byte-limit, changed-size and round-trip tests cover both encodings, and the Python
  decoder refuses a wrong hash, truncation and extra data. Raw slow-writer saturation refuses with bounded 6/96
  peaks while ticks continue. Every capped overflow refused, with a bounded drain.
- **M0.3 pass.** 30/30 back-to-back sessions complete with clear margin (disk about 7× normal and about 1.9× the
  worst observed dip; writer CPU about 2×, and it holds with every core loaded). Every original hash was verified.
  The writer is bounded (96 pixel leases plus 64 × 4 MiB encoded), and PNG/movie packaging is deferred. See A1 and A2.
- **M0.4 pass.** Locked console and preflight `false` in every run. One frozen binary, source digest and recipe. The
  engine stamps device and presenter generations and the consumed lead (0–8.3 ms). Paused pans give fresh
  contiguous IDs at tick 0. Still, fixed and pan parity hold, and so does parity across formats.
- **M0.5 pass.** Pan list below.
- **M0.6 pass.** Running list below; the clamps are A5.
- **M0.7 pass.** Render stall 260.0 ms with a 267.6 ms gap, then recovery. Input stall 250.2 ms with 30 fresh
  identical submissions, then motion. Ordinary schedules have no actions, and fixed mode refuses controls.
- **M0.8 pass.** The OFF/ON inversion is explained and reproduced (E-core placement). The stable active-pacing
  control gives the observer cost with its spread. Bytes, RSS, queue peaks, flush and deferred verification are
  recorded. The second review's "investigate" gap is closed.
- **M0.9 pass.** I ran the runbook's build, cook, capture, defer and finalize commands. It states modes, the live
  lossless format and its limits, the encoded reserve, the pacing and QoS controls, the 16 GiB allowance, retention
  and the Mac M2 owners. It makes no swap, panel or photon claims. The stale design text is A1.

## Inspection lists (written before viewing any frame)

**M0.5 pan** (`pan-full`: native PNGs, contact sheet, native 800×450 crops of 495/496):
- P1 initial hold ≥1 s identical, equal to still/fixed: holds (120 frames, 1.003 s; hash equal).
- P2 every turn submission new, gap ≤17 ms: holds (240 turn frames, no repeats; max gap 12.0 ms).
- P3 rock/ridges coherent, sphere exits: holds (contact sheet; sphere leaves frame by the third row).
- P4 no cut/snap/reversal: holds (accepted yaw monotone 0→36°).
- P5 final hold ≥1 s equal to fixed final: holds (120 frames; hash equal).
- P6 tick 0, contiguous IDs: holds.
- P7 native crops clean: holds (about 4 px shift, crisp rock and ridge edges, no row or channel corruption).

**M0.6 running** (`running-full`, frames 496/497): R1 initial hold: holds. R2 movers advance: holds (cube rotation
advances between adjacent frames, no trails). R3 alpha monotone within pairs: holds (0 regressions). R4 history
epoch 0: holds. R5 no jumps beyond the known clamps: clamps only (A5). R6 final hold: holds. R7 lead 0–8.31 ms: holds.

**M0.7 stalls** (render-stall frames 473/474): S1 holds (the camera advances by the elapsed stall across the gap).
S2 and S3 hold, with the figures above.

I judged motion from native frames, hashes and timing records; I did not watch the MP4 at 1x.

## Proposed item hardening (not applied; definitions unchanged)

- For any later visual milestone that uses capture: qualify a back-to-back series on that milestone's own content,
  recording the encoded bytes per session and the sink rate at which it refuses (A2).

## Reviewer scratch

`tmp/t2-review3-m0/` and the second review's `tmp/t2-review2-m0/` were deleted after this verdict was written.
The builder's `tmp/t2-capture-m0-r3/` (22 GB) is the builder's to close out now that the review is done.

PASS M0
