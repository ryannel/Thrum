# T2-CAPTURE M0 independent milestone review (second claim)

Reviewer: Claude Opus 5.5 (claude-opus-5-5), independent of the Codex builder, 9 November 2026.
Claimed commit: `6be1b11b` (rework of the first claim `770bb4ef`, whose review FAILED M0.3, M0.4 and M0.8; that
review is in this file at `2f089669`). Target: mac-metal, Apple M3 Max, console locked in every run
(`console_locked` and `console_locked_after_capture` true), Screen Recording preflight `false`, no grant requested.

**Verdict: FAIL M0: 2 fail, 0 cant_tell, 0 outside the items.**
M0.3 and M0.8 fail. M0.1, M0.2, M0.4, M0.5, M0.6, M0.7 and M0.9 pass.

## What I ran (verified by running)

All GPU and capture work ran serially under `agent_loop.py lock gpu`, from the clean worktree at `6be1b11b`.

- `./dev check`: passed (4 min 25 s, no failing test). It includes the six real-Metal `submitted_capture` tests
  (now asserting copy-bracket, device 7→8 and presenter 1→3 generations and a 7 ms consumed lead), the odd-width
  readback test and the creation-site scanners.
- Rebuilt `target/release/v1-game`: SHA-256 `0e77c57d…dccbe97`, identical to the binary in the builder's
  `integrated/` manifests; build source digest `05009ccd…` matches. Fresh cook in `tmp/t2-review2-m0/valley`.
- Writer probe `scripts/t2_writer_probe.py <dir> --uncached`: 10 consecutive runs.
- Runbook capture command: 12 cooked pans, 5 input stalls, running, render stall, writer-slow, still, fixed,
  two capture-off arms, and the refused `--case running --mode fixed`. 20 capture-on sessions expected complete.
- Legacy route (`moving-v1.json`, proof overlay) and noon endpoint with the claimed binary, compared with the first
  review's outputs from the base binary.

Host conditions: load-1 between 2.0 and 5.1 during the series; the data volume was 94–97% full (35–55 GB free),
which is how this Mac is normally used. Nothing else of mine ran during captures.

### Results

| Case | Result |
| --- | --- |
| Writer probe ×10 | 10/10 complete, no missing IDs. Service rate 109.4, 199.3, 287.3, 248.3, 261.4, 278.6, **124.2**, 266.0, 409.6, 314.1 records/s. Queue peak 53 (run 1), **85 of 96** (run 7), otherwise 2–16. Run 7 drain 803 ms |
| Pan ×12 | 12/12 complete; 480–481 opportunities = submissions = images; 0 ring/writer omissions, 0 skipped; CPU lease peaks 2–18/96; writer 2.3–3.8 GB/s |
| Input stall ×5 | **run 1 overflowed**: 121 ring omissions, 360/481 images, 799.5 ms image gap, CPU leases 96/96, writer 0.58 GB/s; disk0 280–705 MB/s throughout the interval (`tmp` iostat). Correctly refused (exit 1, `complete: false`). Runs 2–5 complete |
| Running | complete 481/481; ticks 0–119; history epoch 0; 114 strict intermediate alphas; lead 0–8.32 ms (median 5.89) |
| Render stall | complete 451/451; logged delay 251.2 ms; image gap 259.8 ms; 30 skipped opportunities then normal motion |
| Writer slow | expected incomplete (exit 1): 163 images, 318 ring omissions, ticks 0–119 continue, peaks 6/96, drain 14.01 s |
| Still / fixed | still = fixed initial = every pan's initial hash; fixed final = every pan's final hash; all 481 fixed hashes identical to the builder's and the first claim's fixed runs |
| Fixed running | refused: "running-world and diagnostic controls require actual elapsed time" |
| Off ×2 | 480/481 submitted, no images, no tap usage |
| Legacy parity | `route.jsonl`, all five checkpoint PPMs and the noon endpoint (`d7d3d2fc…`) byte-identical to the base binary's |

Integrated capture-on sessions expected complete: **19 of 20 complete, 1 overflow.**

## Findings, worst first

### F1. Throughput still does not hold reliably on this Mac (M0.3 fails; verified)

The writer is disk-bound, and the disk does not always deliver the required ~1 GB/s. The 96-buffer (0.8 s) reserve
absorbs short dips but not a dip lasting the interval:

- Input stall run 1 (`tmp/t2-review2-m0/input-stall/`): iostat shows disk0 at 280–705 MB/s for the whole four
  seconds. The CPU pool filled (96/96), the GPU ring then overflowed (121 omissions) and the session was refused.
  This is an ordinary scenario, not a stress case.
- Probe run 1 averaged 109.4 records/s, below the 120 required, and completed only because the reserve absorbed a
  ~50-frame backlog. Probe run 7 reached 85 of 96 buffers (11 from overflow) when per-write times rose to 50–90 ms
  in the last second.

Together with the builder's ten clean runs, the probe passes every time, but the integrated workflow overflowed
1 in 20 here (the first review saw 1 in 5 pans), and two of ten probes show the margin is thin. The item requires a
writer that "sustains" the workload; on this machine the storage rate sometimes falls below it for seconds at a
time, so the bounded reserve cannot guarantee it. Every failure is refused honestly, so this is a reliability
defect, not fake evidence. I think it matters because later milestones consume this workflow in long series of
sessions, which is exactly when the disk is slowest; the owner may judge a measured retry rate acceptable instead.

Reproduce: run the runbook pan/input-stall commands back to back (about 15 sessions, ~60 GB written) on the 95%-full
data volume, then check `ring_omissions` and `system-iostat.txt`.

To pass: either reduce the bytes the disk must take (for example a fast lossless intermediate), add a measured
storage-throughput preflight that refuses before the interval, or have the runbook state and record the observed
refusal rate with an automatic rerun, with repeated integrated series showing the result.

### F2. Running-world interpolation clamps on half the samples now that the lead is applied (outside T2; verified, does not fail an item)

With the shared `display_target_lead` now consumed, 122 of 236 interior running samples clamp at alpha 1.0
(builder: 82 of 235; first claim with zero lead: 19–27). Six adjacent running samples in my capture are
pixel-identical repeats at the same tick. The capture records this honestly, and the windowed actor uses the same
policy, so this is an existing world-presentation pacing limit (interpolation cannot run ahead of the latest
committed tick by the lead). It belongs to the world/presentation owner (Mac M2 qualification), not to T2. M0.6
passes because the capture preserves what the engine presented.

### F3. Capture-off baseline is unstable and the inversion is not investigated (M0.8 fails; verified)

Step CPU median: my OFF arms 1.039 and 1.727 ms; ON pans 0.817–0.844 ms. Builder: OFF 1.008/1.364, ON
0.834/0.761. Whole-child CPU: my OFF arms 1.66 and 2.45 s, ON pans 2.41–2.50 s, so my second OFF arm equals ON.
The OFF baseline varies by 35–66% between matched arms and is consistently above ON. The rework README says the
lower ON value "is not a speedup" but does not explain why OFF is slower or so variable (plausibly CPU frequency
and scheduling in a lighter loop, which a profile or a frequency/QoS record would show). M0.8 asks to investigate
unexpected scaling, and the headline "+53%/+76% whole-child CPU" depends on that unstable baseline. Load, the copy
GPU bracket (my median 0.9–1.2 ms, positive on every image), and the CPU profiles are now present, so F4 of the
first review is otherwise fixed. I think this is minor; it fails on the unmet "investigate" clause.

To pass: explain the OFF/ON step-CPU inversion and the OFF-arm spread with evidence (for example repeated OFF/ON
pairs with CPU frequency or QoS, or profiles of both loops), and state the observer cost with its spread.

### F4. Lower-severity issues (do not fail items on their own)

- **Diagnostic timing gates image collection (read).** In `submitted_capture.rs` `try_submitted_image`, the slot's
  copy-timer collection runs before pixel collection in one closure; a timing error retires the slot and drops the
  image, making the session incomplete. Evidence completeness now depends on a diagnostic timer. The timer also adds
  one resolution `queue.submit` per completed capture frame.
- **Device-wide feature change for a diagnostic (read).** `capabilities.rs` now requests
  `MAPPABLE_PRIMARY_BUFFERS` on every device that offers it, and `OffscreenTarget::readback` adds `STORAGE` usage to
  every readback buffer when present, including non-capture endpoint readbacks. Harmless on Apple unified memory;
  wgpu warns the feature can be slow on discrete GPUs, so it should be scoped to capture timing.
- **Runbook overstates qualification (read).** `docs/CAPTURE.md` says the six/96 profile "is qualified by five
  consecutive full writer probes followed by five complete integrated pans" without stating the storage-rate limit
  or an expected refusal rate (see F1).
- **Evidence weight (read).** This commit adds 95 MB under `docs/evidence/render/t2-capture-m0/rework/` (the M0
  evidence folder is now 158 MB), including duplicated PNGs and previews across sessions. Worth trimming under the
  storage rules.
- **Lead is a synthetic target (read).** The offscreen lead targets the next 120 Hz opportunity, not a measured
  display target; `docs/CAPTURE.md` labels this correctly.

## Item verdicts

- **M0.1 pass.** Path unchanged from the first review except the added dependency-ordered marker passes around
  the copy, inside the same encoder and submission; no redraw, extra scene render or game queue. Real-Metal
  correspondence, disabled zero-allocation and loss tests pass in the Check. Disabled capture creates no timer or
  marker resources. Legacy route/endpoint parity re-verified byte-identical. Minor scope issues in F4.
- **M0.2 pass.** Loss/replacement, abandon, stale, saturation, resize and writer-failure tests pass in the Check.
  Writer-slow refuses explicitly with bounded 6/96 peaks while ticks 0–119 continue. The first review's
  reserved-slot-on-error issue is fixed (slot retired, CPU lease returned). My input-stall overflow was also refused
  correctly, with rendering continuing.
- **M0.3 fail.** See F1.
- **M0.4 pass.** Locked console and preflight `false` in every run; binary/source/recipe pins match. Device and
  presenter generations now come from `FrameHost` at submission (tests prove changes); lead is consumed through
  `Offscreen::step_for` with the windowed actor's `display_target_lead`, stamped per image and varying 0–8.3 ms.
  Paused-world pans give fresh contiguous submission IDs at tick 0 through the ordinary presenter. Still/fixed/pan
  parity and legacy parity hold. First-review F2 and F3 are fixed.
- **M0.5 pass.** Pan 1 against my list (below), and 12/12 pans complete with identical endpoints.
- **M0.6 pass.** See the list below; F2 is an engine observation, not a capture defect.
- **M0.7 pass.** Render stall: 251.2 ms delay, 259.8 ms gap, then recovery. Input stall (four complete runs):
  257–258 ms delay, 32 identical fresh submissions spanning 259–260 ms, then motion. Ordinary runs have empty
  `diagnostic_actions`; fixed mode refuses running and stall scenarios.
- **M0.8 fail.** See F3.
- **M0.9 pass.** I ran the runbook's build, cook and capture commands and they reproduce every case. It states
  modes, provenance, lead and generation scope, copy-bracket scope, profile arms, 16 GiB allowance, free-space
  preflight, retention and the Mac M2 owners, and makes no swap/panel/photon claims. The overstatement is charged
  to F1/F4.

## Inspection lists (written before viewing any frame)

**M0.5 pan** (my pan 1; native PNGs, per-frame differences at 480×270, contact sheet, native crops of 499/500):

- P1 initial hold ≥1 s identical at yaw 0, equal to still/fixed: holds (121 frames, 1.01 s; hash equal).
- P2 every turn submission a new state, no gap >17 ms: holds (239 single-frame runs; max gap 11.85 ms).
- P3 rock and ridges translate together, sphere exits: holds (contact sheet, sphere leaves frame by ~2.5 s).
- P4 no cut/snap/reversal: holds (moving-frame difference 0.63–1.18, max/median 1.25; accepted yaw monotone).
- P5 final hold ≥1 s at 36°, equal to fixed final: holds (120 frames; hash equal).
- P6 tick 0, contiguous IDs, lead recorded: holds.
- P7 native mid-turn crops without smear/ghosting: holds (about 4 px shift, clean edges).

I judged motion from frames, hashes and per-frame differences; I did not watch the MP4 at 1x.

**M0.6 running:** R1 static initial hold: holds (1.03 s). R2 movers advance: holds (native crops 496/497).
R3 alpha in (0,1) monotone within an endpoint pair: holds (114 strict, no regressions). R4 history epoch constant:
holds. R5 no endpoint jumps beyond clamps: 122 clamps and 6 pixel repeats (F2, engine). R6 static final hold: holds.
R7 lead plausible: holds (4.6–8.3 ms while running, zero only on the primed frame).

**M0.7 stalls:** S1, S2 and S3 hold with the figures above.

## Proposed item hardening (not applied; definitions unchanged)

- M0.3: qualify throughput on a repeated integrated series (for example 20 back-to-back sessions on the normal
  data volume), recording the refusal rate and disk rate, not only on isolated probes.
- M0.8: require at least two OFF/ON pairs with the spread stated, and an explanation of any OFF > ON inversion.

## Reviewer scratch

`tmp/t2-review2-m0/` (git-ignored) holds my manifests, runtime records, iostat logs and crops; bulk raw, PNG and
archive files were deleted after analysis. The first review's `tmp/t2-review-m0/` was deleted. Delete mine after
the next verdict.

FAIL M0: 2 fail, 0 cant_tell, 0 outside the items
