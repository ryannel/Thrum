# Mac M2 current-path qualification, 11–12 November

M2 is claimed locally and all five items pass with mac-metal evidence, but it is **not integrated**. The
pre-rebase mandated checks both passed. Landing then rebased the claim and failed in `cooked_material_gpu`;
the focused eight-test reproduction passes. The console subsequently locked, and work stopped as requested.
[Integration stop and remaining work](integration-stop-20261112.json),
[passing focused diagnostic](integration-diagnostic-20261112.log.gz). A fresh full landing Check is still required.
All attended capture/playback proof is complete; independent lead review remains owed.

## Build and attended setup

All arms use M2.1's release executable SHA-256
`aa1591522962933b55ec1f347cc538880f5eb14b7ea227bf4adcb7f00b585507`, unchanged noon preset, cooked root,
same recipe, 1920×1080 engine output and normal AppKit slider sweep on this M3 Max. M3 owns post-APPLE-TILE
requalification. [Identity](identity.json), [M2.1 motion and M2.3 real-window proof](../mac-m2-20261110/README.md).

The attended setup answers are in [ACCEPTED.json](../../../plans/PLATFORM-RUNTIME-MAC/RECORD.json),
`milestones.M2.owner_answers`: M2.2 at `2026-11-12T06:57:44Z` and M2.5 at `2026-11-12T06:57:53Z` each say
"The screen is unlocked if you need it". Those are the owner's recorded words. Per-arm panel witnesses separately
show the console unlocked and built-in display active/awake. No security setting or permission was changed.

The first 12 November request acquired GPU but refused the missing `target/release/v1-game` before recording.
Commit `10287802` matches all 406 recorded M2.1 build-source hashes; rebuilding it at the original source path
recovered the exact executable hash. Current sources were restored. The executable is now pinned outside
`target/` at `tmp/mac-m2/recover-pinned-20261112/v1-game`. [Recovery](binary-recovery.json),
[reproduction](recover-pinned-binary.py), [build log](binary-recovery-build.log.gz).

## Full-interval presented-time witnesses: M2.5

<a id="resumed-attended-sitting-full-interval-off-witness"></a>

Both successful traces use Blank + Metal Application, Display and Points of Interest, Deferred mode with no
ring window. GPU covers each recording/export batch. Both identify only built-in display 1, 3456×2234 at 120 Hz,
with actual game PID/window retained. [OFF records](attempt-3-off/window.txt), [ON records](resumed-on-traced/window.txt).

| Arm | Whole submissions joined | All OS rows joined | Conversion error bound | Collection + exports |
| --- | ---: | ---: | ---: | ---: |
| OFF traced | 491 / 491 | 846 | 8.5 ns | 81.590 s |
| ON traced, under capture | 488 / 488 | 843 | 9.496 ns | 76.232 s |

[OFF witness](attempt-3-off/witness.json.gz) and [ON witness](resumed-on-traced/witness.json.gz) both cover the
full Whole markers, validate conversion, and have no unmatched sample affecting Whole. OFF explicitly reports
its two terminal unmatched submissions outside Whole. ON has no unmatched rows. Handler retention spans both
markers; same-process marker offset widths are 625 / 708 ns. TOCs, signposts, display identity and all raw tables
are retained beside each witness. Raw/derived display-swap tables remain empty: [OFF counts](attempt-3-off/row-counts.json),
[ON counts](resumed-on-traced/row-counts.json). These empty tables are not the cadence witness.

Four of six granted trace recordings are used: two historical failed configurations, qualified OFF, qualified ON.
The missing-binary and earlier disk refusals consumed none. [Commission](trace-commission.json), [runbook](RUNBOOK.md).
Each recording stays within 16 s, finalization within 75 s, and every export within 30 s.

## Matched content and observer comparison: M2.2

Run order is OFF traced, ON untraced on 11 November, then ON traced on 12 November. OFF has no images or spool;
M2.3's disabled direct-path proof applies to this identical binary. ON retains all 850 / 845 originals with zero
omissions. [Untraced source joins](resumed-on-untraced/window-manifest.json.gz),
[traced source joins](resumed-on-traced/window-manifest.json.gz),
[untraced retention verification](resumed-on-untraced/retention-verified.json),
[traced retention verification](resumed-on-traced/retention-verified.json).

[Timing analysis](timing-review.json), reproduced by [check-timing.py](check-timing.py), keeps marker brackets
separate from submitted endpoints. The nominal schedule is `clamp((elapsed-4)/2)*36`; AppKit polls its thresholds.

| Arm | Whole begin → first changing submission | Last yaw 0 → first yaw 36 submission | First yaw 36 → Whole end |
| --- | ---: | ---: | ---: |
| OFF traced | 1.002453 s | 2.002694 s | 1.104379 s |
| ON untraced | 1.001945 s | 2.000333 s | 1.098420 s |
| ON traced | 1.001944 s | 2.009644 s | 1.097723 s |

The ON/untraced marker sweep is 1.992085 s; this is retained rather than rounded to two seconds. Both ON sources
have fixed tick 2, stable device/presenter/lease generations, monotonic yaw, and no repeated image bytes or source
gap over 16 ms during changing yaw. Maximum source gaps anywhere are 17.018 / 24.196 ms, both in settled/warmup
output. Movie PTS preserve host source timing within 0.5 µs; they do not measure panel cadence.

Native full frames and crops show coherent rock/ridge placement, attached shadows, authored sphere exit and
settled yaw-consistent endpoints. [Untraced native view](resumed-on-untraced/native-yaw18.png) and
[crops](resumed-on-untraced/content-contact.png); [traced native view](resumed-on-traced/native-yaw18.png) and
[crops](resumed-on-traced/content-contact.png), with [row identities](resumed-on-traced/selected-content.json)
and [native crop geometry](resumed-on-traced/contact-geometry.json).
The complete [untraced 1× movie](resumed-on-untraced/preview-1x.mp4) and
[traced 1× movie](resumed-on-traced/preview-1x.mp4) remain for review. Fresh [native 1× self-review](content-review-20261112.json) passed the worded list. Both full movies
reached ended=true at rate 1; close pan replays supplement native-source inspection. Chrome reported 7 / 17
decoder drops on full playback (18 cumulative after the traced excerpt). These are player losses, not engine
omissions or OFF cadence. [GPU hold and panel records](playback-20261112/invocation.json).

| Observed median, ms | OFF traced | ON untraced | ON traced |
| --- | ---: | ---: | ---: |
| Host render elapsed, including waits | 3.221 | 1.079 | 1.297 |
| Host encode elapsed | 0.738 | 0.537 | 0.747 |
| GPU scene query | 2.698 | 2.709 | 2.688 |

[Complete comparison](observer-comparison.json) retains every joined sample, missing-sample lists, GPU present/
contribution queries, distributions, raw median deltas, miss neighbors and load. [analyse-arms.py](analyse-arms.py)
reproduces per-arm costs. All marked host/GPU scene samples join. ON traced minus untraced encode median is
+0.210 ms; ON traced minus OFF traced is +0.009 ms. Host render changes are not a capture speedup: waits, run order
and host load confound causality. One-minute load is 4.72→5.76, 7.29→7.31 and 3.71→2.88 respectively; traced load
sampling includes exports, while untraced sampling covers the game. No isolated overhead percentage is claimed.

Cadence is claimed only from capture OFF. [Every strict OFF over-period interval](resumed-off-over-period-investigations.json.gz)
retains drawable, time, yaw, both game records and load references: 236 overlap Whole; non-miss excess above the
nominal 120 Hz grid is at most 5.501 µs. Two whole missed refreshes are 547→548 (16.665250 ms, 8.315917 ms game gap)
and 548→549 (16.666667 ms, 12.193708 ms game gap). The first has ordinary host/GPU neighbors; the second correlates
with 4.612 ms host render. GPU scene neighbors remain 2.6–2.7 ms. This does not isolate a cause.

ON has four two-refresh intervals **under capture**: 451→452 and 455→456 at yaw 0, 494→495 at yaw 3.10, and
634→635 at yaw 24.25. Game-call gaps stay 8.276–8.398 ms. The preceding sample at 455 has 8.176 ms host render;
635 has 3.093 ms GPU scene work within this run's 3.428 ms maximum. Full neighbor investigations are in the
comparison, and all 246 strict marked ON over-period intervals remain in its witness. None qualifies OFF cadence.

## Reusable command, checks and limits

[mac_m2_presented.py](../../../../scripts/mac_m2_presented.py) filters the game PID's
`ca-client-presented-handler.at-time`; it never substitutes callback time or `delta`. Same-process signposts and
mach timebase 125/3 validate `presented_media_ns = (raw_at_time + marker_offset_ns) * 3 / 125`.
The [fresh known-answer rerun](resume-known-answer.json) finds OFF 526→527, OFF 544→545 with the 15.470792 ms
game gap, and ON 558→559, and refuses both truncated historical intervals with expected exit 2.
[Fourteen targeted tests](resume-20261112-tests.log) pass. The [mandated milestone check](module-check-20261112.json) passes both M2.3 under GPU and the repository Check.

Callback target, actual opportunity, game post-present record, GPU query/completion, engine image time, movie
PTS and OS-reported presented time are separate measurements. Acquisition has no separate absolute timestamp.
Kernel swaps, scan-out, photons, input-to-photon and physical iPhone timing are unmeasured. Engine images exclude
native controls and compositor output. Source/native-image review uses the worded motion criteria; Ground E is
scene/framing context only. No earlier cadence verdict is reused for this build.

<a id="remaining-m22-blocker-shared-disk-headroom"></a>

The [11 November checkpoint](HISTORY.md) retains failed attempts, the asleep-panel stop and the resolved disk
blocker. Complete source archives and traces stay under `tmp/mac-m2/` through lead review. Selected records and
movies are in Git; local bulk originals have no durable backup. The exact binary has proven source reconstruction.
