# Post-Apple-tile Mac M3 proof, 14 November 2026

Both items pass builder inspection on Apple M3 Max/Metal. The release executable is
`5ee819346927e2e48def9c0e4ddf701a6d9617cfc873640242ca40033b48f1ea`, built from post-APPLE-TILE-M3
source `d9ca8c13`. All 538 build inputs still match the verified merged assembly `145cadb2`;
all 490 capture source hashes agree. Every arm uses that executable, freshly cooked valley,
noon preset, Portable recipe and 1920×1080 extent. Observer/evidence edits change no engine input.
[Build map](M3/build.json.gz), [source audit](M3/sitting/source-audit.json.gz),
[source merge and full-check pass](M3/source-merge.json), [exact baselines](M3/exact-baselines.json.gz).
Independent M3 review belongs to the lead.

**M3.1.** The real-time paused-world pan verifies all 481 originals, with zero skipped opportunities,
ring/writer omissions or exact sweep repeats. Initial/final holds last 1.004219/1.008156 seconds around
the complete two-second 0°–36° semantic-input sweep. The largest image gap is 12.318958 ms.
All four attributed launcher/driver read-only Screen Recording preflights return denied. Console state
is unlocked before/after and is witness metadata only.
[Analysis](M3/analysis.json.gz), [launcher](M3/motion/launcher.json),
[complete source manifest](M3/motion/manifest.json.gz), [1× movie](M3/motion/preview-1x.mp4),
[native frames/crops](M3/motion/selected/selected.json).

Native sources/crops, complete native-size 1× replay and sampled sequences preserve coherent rock/ridge
placement, attached shadows, the sphere's sweep/exit and yaw-consistent settled endpoints. No cut, snap or
reversal was observed. Ground E identifies the fixture only. Browser readback and decoder overhead is
recorded separately: the latest source replay reports 34 dropped video frames, ON/untraced 12 and ON/traced 2.
These are viewer observations, not capture omissions or panel cadence. All source originals verify.
[Playback reports and samples](M3/review/playback-motion-clean/playback.json.gz),
[ON/untraced](M3/review/playback-on-untraced/sample-strip.png),
[ON/traced](M3/review/playback-on-traced/sample-strip.png).
Complete window movies: [ON/untraced](M3/presented/on-untraced/preview-1x.mp4),
[ON/traced](M3/presented/on-traced/preview-1x.mp4).

**M3.2.** The real-window channel/row oracle verifies 843 pattern originals; 834 valley originals also verify.
Both replacements retire submissions 357–359 with three readbacks pending, change surface lease 2→3 and
presenter 1→2, preserve identical boundary pixels and resume presentation. Stale-image recycling leaves
the fresh outstanding count 1→1. Actual-window screenshots show intact output before/after replacement.
The OFF probe presents 850 frames with no images/spool or enabled tap. Capture follows ordinary present,
consumes no additional drawable lease, and OFF constructs no tap/writer. The merged real-Metal regressions pass.
[Window inspection](M3/window-inspection.json), [actual window](M3/window/valley-replacement/window-6s.png).

Exactly one OFF and one ON recording use the accepted minimal/Deferred configuration. Both traces cover the
whole marked holds/pan and join all 490 marked submissions, with zero unmatched marked rows. Conversion error
is 6.496 ns OFF and 7.012 ns ON. ON has one unmatched startup row outside the marked interval. Each trace
identifies the game PID/window and sole built-in 3456×2234/120 Hz display; kernel display tables remain empty.
[OFF witness](M3/presented/off-traced/witness.json.gz), [ON witness](M3/presented/on-traced/witness.json.gz),
[table counts](M3/presented/off-traced/row-counts.json), [exhausted commission](M3/recording-commission.json.gz).

| Marked OFF drawable pair | Refresh periods | Yaw | Game present-call gap |
| --- | ---: | ---: | ---: |
| 542→543 | 2.000000 | 10.450° | 7.872458 ms |
| 543→544 | 2.000000 | 10.600° | 8.422084 ms |

Both destinations have ordinary host render durations (1.298/1.384 ms) and GPU query sums (3.619/4.085 ms).
Later neighboring host duration reaches 17.181 ms, consistent with presentation backpressure; its cause is
not isolated. All 253 strict over-period OFF intervals and 238 ON intervals retain time/yaw, calls, neighboring
GPU queries and host load. OFF/ON-traced/ON-untraced median host durations are 1.299/1.404/0.962 ms; named GPU
sums are 4.017/3.796/3.859 ms. Shared load and run order confound causal overhead percentages.
ON/traced has three two-refresh intervals under capture and a 19.920 ms pan source gap. Both ON source sequences
are complete, monotonic and repeat-free. Startup gaps of 228.446/62.024 ms precede the marked interval.
[Complete interval investigations and observer costs](M3/cadence.json.gz).

The [fixed sitting](M3/sitting_driver.py) holds `gpu` through qualification and final checked integration.
Callback targets, opportunities, present calls, GPU queries, engine image times, movie PTS and OS-reported
presented times remain separate. Cadence comes only from OFF. Kernel swaps, scan-out, photons, input-to-photon,
physical iPhone behavior and continuous physical-panel viewing remain unmeasured. Compressed originals,
binary, cooked store and trace bundles remain locally under `tmp/mac-m3` through review; they have no declared
durable backup. [Master hashes](M3/retained-masters.json) and committed exports preserve provenance.
PNG expansions and owned Chrome profiles are reconstructible and reclaimed. No further trace recording is allowed
without a scoped re-plan. Inspect retained exports with the unchanged `scripts/mac_m2_presented.py`; expand XML
gzip files into owned scratch and retain their accompanying arm tables. Do not rerun the recording command.
