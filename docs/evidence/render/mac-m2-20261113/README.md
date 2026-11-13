# Mac M2 current-build qualification, 13 November 2026

All five items pass builder inspection on Apple M3 Max/Metal. The release was built after merging
`7a791b20dcd2c70b2537661b123c0d13978bd9f9`. Executable SHA-256 is
`688f378f96d33c61f5f21b70582610ff9673c63e7f3c5af6b5eb859e6b4f8422`. Every arm uses that executable,
cooked store, noon preset, portable recipe and 1920×1080 extent. All 466 capture source hashes agree with the
511-file build map. Independent milestone review belongs to the lead.
[Build](build.json.gz), [source manifest](motion/manifest.json.gz), [window identity](identity.json).

The owner's M2.2/M2.5 feedback at `2026-11-12T06:57:44Z`/`2026-11-12T06:57:53Z` says exactly
"The screen is unlocked if you need it". This sitting independently witnesses an unlocked, active, awake
built-in panel, kept awake with `caffeinate -d -i`. The separate window observer already had permission.
[Human feedback record](../../../plans/PLATFORM-RUNTIME-MAC/ACCEPTED.json),
[panel observations](presented/off-traced/panel-witness.jsonl.gz).

**M2.1.** All 480 originals verify, with zero ring/writer omissions. All four attributed launcher/driver
Screen Recording preflights return denied. Initial/final holds are 1.003472/1.008173 s around the elapsed-time
two-second sweep. One skipped opportunity produces a 22.935667 ms gap during the initial stationary hold.
The sweep has no exact repeated images. Native sources/crops, sampled sequences and complete 1× playback
preserve rock/ridge placement, attached shadows, sphere sweep/exit and yaw-consistent endpoints, with no
observed cut, snap or reversal. Ground E supplies framing context.
[Analysis](analysis.json), [launcher](motion/launcher.json), [movie](motion/preview-1x.mp4),
[native selections](motion/selected.json), [playback](playback-motion.json).

**M2.3.** All 844 pattern originals equal the independent channel/row oracle. All 833 valley originals verify.
Both replacements retire IDs 357–359 with three readbacks pending, change lease 2→3 and presenter 1→2, preserve
identical boundary pixels and resume presentation. Stale recycling retains outstanding count 1→1. Actual-window
screenshots show intact output before/after. The observer follows ordinary present consuming the drawable.
The OFF probe has 851 presentations, zero images/spool and disabled tap throughout. GPU-locked `./dev check`
passes correspondence, disabled-allocation, stale-generation and loss regressions.
[Window proof](analysis.json), [acceptance](window-acceptance.json), [full check](check.stdout.gz).

**M2.4/M2.5.** The witness reproduces all three known misses and the 15.470792 ms game gap, while refusing
historical truncated coverage. Fourteen tests pass. Fresh Deferred OFF/ON traces retain 841/845 joined rows,
each with all 488 marked submissions, zero unmatched marked rows, complete Whole coverage and validated
7.996 ns conversion error. Each identifies one built-in main display at 3456×2234/120 Hz. Kernel display tables
remain empty. These are recordings five/six, exhausting the allowance.
[Tests](tool-tests.log), [OFF witness](presented/off-traced/witness.json.gz),
[ON witness](presented/on-traced/witness.json.gz), [table counts](presented/off-traced/row-counts.json),
[counter](recording-commission.json).

**M2.2.** Matched ON/untraced and ON/traced verify all 851/846 originals without omissions or sweep repeats.
Complete 1× playback and native-source inspection retain coherent motion. ON/traced has a 23.815875 ms source
gap at 569→570 beside a 17.042709 ms host render duration. Its browser playback reports six decoder drops.
AppKit marker polling jitter, complete holds and control/submission joins are retained.
[Motion analysis](analysis.json), [untraced movie](presented/on-untraced/preview-1x.mp4),
[traced movie](presented/on-traced/preview-1x.mp4), [pan strip](presented/on-traced/pan-strip.png).

| OFF drawable pair | Refresh periods | Game present-call gap |
| --- | ---: | ---: |
| 365→366 | 2.000000 | 8.238958 ms |
| 546→547 | 2.499890 | 8.178458 ms |
| 547→548 | 1.500005 | 9.023125 ms |
| 602→603 | 2.000005 | 7.562292 ms |

All 238 strict over-period OFF intervals join time/yaw, present calls, opportunities, targets, neighboring named
GPU queries and run-wide load. Later host-duration spikes suggest presentation backpressure, with cause
unresolved. Median OFF/ON-traced/ON-untraced host durations are 1.275/1.311/0.822 ms, graph-query sums
3.946/3.936/3.980 ms. Run order/shared load constrain causal costs. Cadence comes from OFF. ON intervals are
under capture. Callback, GPU, image and movie clocks remain distinct. Kernel swaps, scan-out, photons,
input-to-photon and physical iPhone timing are unmeasured.
[Complete investigations](cadence.json.gz), [reproduction, inspection limits and retention](RUNBOOK.md).
