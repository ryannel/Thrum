# Mac M2 attended diagnostics, 11 November

**M2.2 and M2.4 remain open. The two-attempt diagnostic commission is exhausted.** Both Game Performance recordings
finalized and exported readable data, but both raw swap/VSync tables and derived displayed-surfaces-interval have
zero rows. Display identity and the four same-process pan markers are present. A scoped re-plan must address the
missing display-event source before further diagnostic recording. [Result](result.json) and [durable journal](trace-commission.json).

| Arm | Engine presentations | Original images | Raw swaps / VSync | Derived swaps | Clock-offset bound |
| --- | ---: | ---: | ---: | ---: | ---: |
| ON untraced | 851 | 851 | Untraced | Untraced | Host clock |
| ON Game Performance, OS logs excluded | 848 | 848 | 0 / 0 | 0 | 0.666 µs |
| OFF Game Performance, OS logs included | 846 | 0 | 0 / 0 | 0 | 0.333 µs |

The same release binary `aa1591522962933b55ec1f347cc538880f5eb14b7ea227bf4adcb7f00b585507` matches the completed
M2.1/M2.3 proofs. All three arms use the same cooked noon store, preset, engine recipe, 1920×1080 extent and normal
AppKit slider target/action pan. The ON trace identifies PID 49285/window 45393, and OFF PID 74068/window 45482,
on the M3 Max built-in 3456×2234 display reporting 120 Hz. The OFF arm marks capture disabled in every surface row,
retains normal frame markers, produces zero images and creates no spool. The 10 November proof establishes the
underlying disabled-resource and direct-present behavior. The assembly predates later engine changes on main;
Mac M3 owns the planned post-APPLE-TILE requalification.
[First invocation](invocation.json), [first window](window.txt) and [OFF invocation/window records](off-traced/window.txt).

The first changed hypothesis excluded ordinary OS-log messages to reduce finalization work while preserving
Game Performance display events. It saved a readable trace, but its raw and derived display-event tables were
empty. The second hypothesis restored OS logs, retaining the absence of a desktop recorder and using capture OFF.
It also saved a trace with empty display-event tables. That observation rejects OS-log exclusion as a sufficient
explanation. The missing event source remains undiagnosed. A post-run read-only `DevToolsSecurity -status` reports Developer mode disabled. That policy observation leaves the actual recording authorization and its effect on display-event collection unverified. [Authorization observation](authorization-status.json). Collection, finalization and the three standard exports
took 70.185 seconds for ON and 63.357 seconds for OFF. Each trace's recorded interval was below the 16-second cap,
and each entire collection/export interval was below 75 seconds. Both process groups exited normally. The finite
record/finalization/export commands and deadlines are retained.
[First log/timing](process.log), [OFF log](off-traced/process.log) and [OFF timing](off-traced/process.timing.json).

The first trace's raw exports are [TOC](toc.xml.gz), [derived swaps](swaps.xml.gz) and
[raw display tables](raw-displays.xml.gz). The second has its own [TOC](off-traced/toc.xml.gz),
[derived swaps](off-traced/swaps.xml.gz) and [raw display tables](off-traced/raw-displays.xml.gz).
Each raw export contains one device-display-info row and zero display-surface-swap, display-surface-queue and
VSync rows. No app/window swap stream or numeric Swap Time is available for attribution or interval measurement.
The PID/window logs and display identity alone leave the cadence criterion unmet.

All four MacM2Whole/MacM2Pan boundaries resolve to the correct game PID in each trace. Identical duplicate schema
projections are deduplicated. Each media-before/media-after bracket constrains the same-process trace-clock offset;
all four bounds intersect consistently. Their widths are 0.666 and 0.333 microseconds. These calibrations establish
the clock joins while the absent swap streams leave cadence unqualified.
[First signposts/calibration](clock-calibration.json) and [OFF calibration](off-traced/clock-calibration.json).
The [analysis script](analyse-traces.py) resolves XML references, counts table rows and retains all anchor values
in [analysis.json](analysis.json). It reads the complete local roots under `tmp/mac-m2/final-window-20261110`.

Per-arm load, iostat interval CPU/disk samples and process snapshots are retained. The one-minute load changed
3.52→19.51 during ON/traced and 11.76→6.34 during OFF/traced. These sampler intervals include startup, finalization
and export as well as the pan. Process CPU snapshots are lifetime percentages. The GPU lock covered each complete
recording/export batch. Other builders' GPU holders delayed acquisition, and their CPU work could continue under
the lock. These are shared-machine observations. OS-log tracing options differ between the two traced arms.
[ON load](load.json), [OFF load](off-traced/load.json) and [per-arm CPU/GPU comparison](analysis.json).

Across all samples including warm-up, render-method elapsed medians were 0.907 / 1.267 / 1.213 ms for untraced ON /
traced ON / traced OFF. Reported gpu_present_ns medians were 1.136 / 1.126 / 1.125 ms, with zero missing GPU-query
sample rows. These comparisons retain host-load, tracing-option and run-order confounds. They support no isolated
causal percentage for capture overhead. Host method elapsed durations include waits. GPU query intervals,
callback targets, accepted input, engine submission, map observation and movie PTS keep their own timestamps.
Compositor Swap Time, physical panel output and input-to-photon timing remain unmeasured.

Both ON source archives independently decoded and matched every original RGBA hash, with all 851/848 image-to-
submission/input joins and zero omissions. The native frames and crops preserve coherent rock/ridge placement,
attached shadows, the sphere's authored leftward sweep and exit, and settled 0°/36° endpoints. The full-sequence
navigation sheets retain the three-second warm-up, initial hold, pan and final hold. No exact repeated image bytes
occur inside either sweep. Largest source-image gaps were 17.438/27.879 ms, both during warm-up. The traced sweep
has local gaps of 15.715 and 13.088 ms, recorded with their source IDs/yaws in its manifest. These are source-image
observations. Ground E supplies scene/framing context. Timed movie inspection was not completed: the review request was cancelled while still waiting for the shared GPU lock, before acquisition. Source-image inspection is complete.
[Untraced selected sources](on-untraced-current/selected.json), [traced selected sources](on-traced/selected.json),
[untraced 1× movie](on-untraced-current/pan-1x.mp4) and [traced 1× movie](on-traced/pan-1x.mp4).

The panel stayed unlocked, active and awake throughout both recordings and exports. The read-only guard samples
every 0.5 seconds with a two-second I/O Registry timeout; failure stops the collector's owned process group.
Earlier, the remaining diagnostic was stopped while still queued when the panel slept, consuming no attempt.
The owner then woke it and the lead held the display awake for the completed continuation. Bounded temporary
caffeinate assertions were also used during the earlier queue. [ON panel witness](panel-witness.jsonl.gz),
[OFF witness](off-traced/panel-witness.jsonl.gz) and [result history](result.json).

Complete local trace bundles are `tmp/mac-m2/final-window-20261110/on-traced/pan.trace` and `off-traced/pan.trace`.
Lossless ON archives, expanded originals, hashes, input/submission tables and previews remain under the same root,
with the fresh baseline in `on-untraced-current`. Exact executed collector sources are retained by SHA-256 in
collectors/. Selected evidence lives here in Git. Ignored local archives have no durable backup. The
[runbook](../mac-m2-20261110/RUNBOOK.md) records the tested commands and exhausted allowance. The CPU test-binary build, six Python refusal controls, documentation check and full `./dev check` passed.
The full check ran under the GPU lock; its [compressed log](check.log.gz) and hash are recorded in
[verification state](verification.json). Timed playback remains unverified. Integration uses the required land command, whose own check reruns under the GPU lock. Independent lead review remains owed.
