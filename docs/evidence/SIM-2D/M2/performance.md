# SIM-2D M2 cost investigation

Engine timings exclude browser startup and resource queue waits. Chrome's `utc`
and `load_before` are sampled before acquiring the lock; `load_after` is sampled
when the invocation returns. Those brackets can span a queue wait and are coarse
shared-host context, not an isolated measure of contention during each timed call.

## Second-review integrated Mac measurements

The [current Mac record](costs-mac-metal.json) has two complete release runs at
`768636def207df168be1bfb0feb90c25c12f860a`, source key
`0f6f30f9d3b2c2b2199641758aabbdead4065825cebce6a9818ff3b71217d8de`.
Its [source/completeness validator passes](cost-validation-mac-metal.json).
The earlier campaign is preserved with its original provenance in Git at
`768636def:docs/evidence/SIM-2D/M2/costs-mac-metal.json`.
No source check was relaxed and no old measurement was relabeled.

| Crowd bodies | M1 tick / index, ms | Repeat 1 tick / index, ms | Repeat 2 tick / index, ms |
|---|---:|---:|---:|
| 2,000 | 0.523417 / 0.026042 | 0.715708 / 0.090292 | 0.731709 / 0.089833 |
| 5,000 | 1.232208 / 0.075708 | 1.671208 / 0.240041 | 1.645791 / 0.237500 |
| 10,000 | 8.068417 / 0.165792 | 9.800459 / 0.482333 | 9.753500 / 0.476833 |

Four M1 regression flags remain: serial Crowd at 2,000 and 5,000 in both repeats,
33.6–39.8% slower at identical neighbour counts. Index time adds 64–65 and
162–164 microseconds at those sizes. The query, predicate, ordering and steering
code is unchanged from the earlier investigation below; its eight stable radix
passes, conservative bounds and shared predicates explain additional work without
assigning causal percentages. The historical CPU samples are not profiles of this
fresh executable. The clock additions that invalidated the old source key are
included in this campaign; no isolated timing effect is attributed to them.

Every forced-parallel row is faster than its matching serial row. At 10,000 bodies,
serial tick medians span 9.754–10.054 ms and parallel medians 1.981–2.254 ms.
Eight-tick replay spans 79.333–82.300 ms serial and 15.280–38.792 ms parallel.
The 38.792 ms Mixed replay in repeat two remains recorded; these one-shot replay
measurements do not establish a stable crossover or isolate contention.
One-minute load brackets span 7.18–16.45. No row trips the clock or chronological
quarter-regression flag. The four scaling flags remain serial 5,000-body rows at
the table-capacity step discussed below. Warmed build and every query measure zero
allocations. All times and counts remain observations, never pass thresholds.

## Second-review integrated Chrome measurements

The [current Chrome record](costs-chrome-webgpu.json) has two complete release runs
at `768636def`, with the same source key as the current Mac campaign.
Its [source/completeness validator passes](cost-validation-chrome-webgpu.json).
These are real isolated HeadlessChrome 157 runs of the freshly built Wasm, not
native substitutes. Both original clean-build identities and load brackets remain
in the records. The previous Chrome campaign remains in Git at
`768636def:docs/evidence/SIM-2D/M2/costs-chrome-webgpu.json`.

| Crowd bodies | M1 tick / index, ms | Repeat 1 tick / index, ms | Repeat 2 tick / index, ms |
|---|---:|---:|---:|
| 2,000 | 0.570 / 0.035 | 1.005 / 0.100 | 1.035 / 0.105 |
| 5,000 | 1.355 / 0.100 | 2.260 / 0.265 | 2.280 / 0.265 |
| 10,000 | 8.580 / 0.205 | 14.400 / 0.535 | 13.905 / 0.520 |

All six Crowd rows trip the M1 quarter-regression flag at identical neighbour
counts: 62.1–81.6% slower. The separate index increases are 65–330 microseconds.
As on the Mac, query, steering, predicate and ordering code is unchanged from the
historical investigation below. Eight-pass canonical ordering, conservative bounds
and shared predicates add work; the retained historical profiles locate those paths
without assigning causal shares or claiming to profile this fresh executable.
Both repeats agree within 3.6% for Crowd. Mixed tick medians span 1.005–1.055,
2.300–2.445 and 14.255–14.765 ms; it has no same-neighbour-count M1 comparator.

No row trips the clock or chronological quarter-regression flag. The three scaling
flags are 5,000-body Crowd in repeat one and Mixed in both repeats, at the same
table-capacity step explained below. At 10,000 bodies, tick medians span
13.905–14.765 ms and eight-tick replay spans 114.260–120.025 ms.
One-minute load brackets span 4.36–36.33, including long resource-queue waits.
Engine timers exclude those waits; the load samples cannot isolate contention
during a particular call. These remain shared-host baselines, not quiet-machine
qualifications or timing budgets. Neither campaign changed a trace or its length.

## Previous Mac refresh (historical)

The previous Mac record (retained in Git at the path above) contains two complete release runs at
`119bd20ab17438e0bab5a5c5bfb52b695e835b74`, source key
`a8df10ea68a72e0b109e9f934951d7cd1dcf2809395fbba59c195c0b44d8b8c3`.
Its source/completeness validator passed before later clock changes. All 24 rows
retain their load brackets, M1 comparators and second-repeat comparators. Warmed build
and all five query calls again observe zero allocations. No count or time gates success.

| Crowd bodies | M1 tick / index, ms | Refresh repeat 1 tick / index, ms | Refresh repeat 2 tick / index, ms |
|---|---:|---:|---:|
| 2,000 | 0.523417 / 0.026042 | 0.747584 / 0.091209 | 0.718209 / 0.089458 |
| 5,000 | 1.232208 / 0.075708 | 1.675750 / 0.242000 | 1.627875 / 0.236750 |
| 10,000 | 8.068417 / 0.165792 | 9.852000 / 0.479458 | 9.640208 / 0.475709 |

Four rows trip the M1 quarter-regression flag: serial Crowd at 2,000 and 5,000 in
both repeats, by 32.1–42.8%. Their accepted neighbour counts equal M1's.
Index increases are 63–65, 161–166 and 310–314 microseconds at the three sizes. The
eight-pass canonical ordering, shared shape predicate and conservative-bound work
described in the initial investigation below remain unchanged. That investigation
locates these paths, without assigning causal percentages to the tick increase.

The 10,000-body ticks are 22.1% and 19.5% above M1, below the quarter tripwire.
Crowd serial medians agree within 4.1% between repeats. Full-run one-minute load
brackets range from 6.03 to 22.14. These remain shared-host baselines, with no
measurement of contention's causal share. The native executable is byte-identical
to the previous refresh at `d24658b33`; the harness and integrated source manifest
have new identities. Earlier records keep their original provenance in Git.

The four scaling flags are serial 5,000-body rows of both traces in both repeats,
at the same 4,096-to-16,384 table-capacity step explained below. No row trips the
clock flag or the chronological quarter-slower flag. Every current forced-parallel
row is faster than its matching serial row. Mixed at 2,000 is 0.360–0.372 ms
parallel versus 0.717–0.743 ms serial. The previous `d24658b33` refresh recorded
1.121 ms parallel versus 0.797 ms serial for this row in repeat one, and 0.594 ms
versus 0.731 ms in repeat two. Those source-tagged measurements remain in Git at
`a00d6c1e3` in this record's path;
the current improvement does not establish a quiet-host crossover.

At 10,000 bodies, serial tick medians span 9.640–9.878 ms and forced-parallel
medians 1.952–2.107 ms. Eight-tick replay spans 80.279–83.075 ms serial and
15.733–17.240 ms parallel. All full-trace index, digest, neighbour and replay
measurements remain in the record. This refresh binds the Chrome queue repair
and upstream core world-lane source; it does not isolate a timing effect from them.

## Previous Chrome refresh (historical)

The previous Chrome record, preserved in Git at
`768636def:docs/evidence/SIM-2D/M2/costs-chrome-webgpu.json`, contains two complete release
runs at `119bd20ab`, with the same source key as the refreshed Mac record.
Its source/completeness validator passed before the later clock additions.
The isolated browser identifies itself as HeadlessChrome 157. The Wasm artifact
is byte-identical to the previous refresh; no measurements were relabeled.

| Crowd bodies | M1 tick / index, ms | Refresh repeat 1 tick / index, ms | Refresh repeat 2 tick / index, ms |
|---|---:|---:|---:|
| 2,000 | 0.570 / 0.035 | 0.995 / 0.100 | 0.990 / 0.100 |
| 5,000 | 1.355 / 0.100 | 2.235 / 0.265 | 2.155 / 0.260 |
| 10,000 | 8.580 / 0.205 | 13.875 / 0.525 | 13.855 / 0.515 |

All six Crowd rows trip the M1 quarter-regression flag at identical neighbour counts:
59.0–74.6% increases. The separate index increases are 65–320 microseconds. The
query/rule code and its canonical-ordering, shared-predicate and conservative-bound
paths are unchanged from the initial investigation below. Its historical samples
locate those paths; they are not samples of the refreshed executable and do not
assign causal shares to the increase. The two refreshed Crowd repeats agree within
3.8%. One-minute load brackets span 5.42–11.73, with the queue-sampling limits
noted above. These remain shared-host baselines.

Mixed tick medians span 1.005–1.045, 2.270–2.280 and 14.015–14.190 ms at the three
sizes. Mixed has no same-neighbour-count M1 comparator. No row trips the clock or
chronological quarter-slower flag. Scaling flags occur at 5,000 bodies in both
traces' first repeat, at the table-capacity step described below. The second-repeat
timings fall below that flag while neighbour counts remain unchanged.
At 10,000, ticks span 13.855–14.190 ms and eight-tick replay spans 114.130–119.155 ms.

## Historical investigation

The sections below retain the initial measurements and investigation. The M2 review
accepted the explanation but found that later core clock/presentation changes invalidated
their source binding. The refreshed records above establish the integrated build's
cost. Initial records and samples below retain their original provenance.

## Initial Mac CPU measurements

[The initial record](costs-mac-metal-initial.json) contains two release runs at
`9d06e07c731bfbc7253a9ea04b6de2cd2ca9b189`, source key
`b2ae5a08c3585081315bcbecb8368247cf69afacd37868c310f0f8660cf9f613`.
Every row runs the full trace, records its load bracket, and retains its M1 and chronological
comparators. These are shared-host baselines; one-minute load brackets range from 8.97 to 21.43.
No time or allocation count gates success.

The four quarter-slower flags are the serial Crowd rows at 2,000 and 5,000 bodies in both repeats.
Their accepted neighbour counts equal M1's. Against M1's latest repeat:

| Bodies | M1 tick / index, ms | M2 repeat 1 tick / index, ms | M2 repeat 2 tick / index, ms |
|---|---:|---:|---:|
| 2,000 | 0.5234 / 0.0260 | 0.7363 / 0.0905 | 0.7110 / 0.0900 |
| 5,000 | 1.2322 / 0.0757 | 1.6590 / 0.2409 | 1.6134 / 0.2373 |

The serial medians agree within 4% between repeats, with the same flags in both.

The tick increases are 31–41%. The separate index measurement increases by 64–165 microseconds,
compared with total tick increases of 188–427 microseconds. M2 constructs a canonical wide-query
scan during every build with eight stable byte-counting passes and retained scratch. M1 instead
allocated and sorted matches when a wide query occurred. This deliberately moves work into the
linear build so warmed queries, including a wide one, can remain read-only and avoid scratch allocation.

The separate build measurement does not isolate the rest of the tick difference. M2 also gathers private
steering chunks, dispatches shared shape predicates, and computes conservative rounded bounds.
[A native sampling diagnostic](native-profile.json), with its [raw sample](native-sample.txt),
locates these paths in the real 2,000-body engine run: 752 of 990 history-step samples are in
steering, including 178 in the shared intersection predicate; 128 are in the in-tick index build.
This supports the hot-path explanation but does not isolate a causal percentage for each change.
The compiler folds the tiny-scale threshold into a constant; normal predicates do not divide to
construct it. The profiled row is observer-affected and never replaces the two unobserved repeats.

The scaling flags are the serial 5,000-body rows of both traces, in both repeats. The table grows
from 4,096 to 16,384 cells while bodies grow 2.5 times and accepted neighbours about 2.2 times.
New index time grows about 2.6 times. Its fixed table scan and radix work are less amortized per
neighbour at this size. The 10,000-body rows and all parallel rows have no scaling flag. M1 already
recorded the same 5,000-body table-growth effect; no crowd or trace was thinned to change it.

At 10,000 bodies, serial tick medians are 9.706–9.935 ms and forced-parallel medians are
2.069–2.440 ms. Both traces and all sizes are retained whichever mode is faster. No Mac row trips
the 60 Hz clock flag, and no row is a quarter slower than the preceding repeat at comparable
neighbour count. These observations do not qualify a quiet-machine crossover or another device.

At 10,000 bodies, eight-tick replay is 79.8–108.0 ms serial and 16.8–18.1 ms forced-parallel.
The raw rows retain rewind and digest timings too, for SESSION's replay planning.

The counting allocator observes zero allocations in warmed build, circle, box, capsule, wide
circle and nearest-k calls at 10,000 bodies. First-use buffer growth is outside that scope. The
test and record check completeness, never a zero-allocation threshold.

## Initial Chrome Wasm measurements

[The initial Chrome record](costs-chrome-webgpu-initial.json) retains two full release runs on
the same source key. All six serial Crowd rows trip the M1 quarter-regression flag:

| Bodies | M1 tick / index, ms | M2 repeat 1 tick / index, ms | M2 repeat 2 tick / index, ms |
|---|---:|---:|---:|
| 2,000 | 0.570 / 0.035 | 1.030 / 0.100 | 1.035 / 0.100 |
| 5,000 | 1.355 / 0.100 | 2.290 / 0.265 | 2.240 / 0.260 |
| 10,000 | 8.580 / 0.205 | 13.930 / 0.525 | 13.920 / 0.520 |

Accepted Crowd neighbour counts equal M1's exactly. The tick increases are 62–82%,
while the separate index stage increases by 65–320 microseconds. Index preparation therefore
does not account for the whole increase. The production hot path also changes from M1's
circle-only predicate to the shared shape predicate and conservative rounded bounds, as
the native investigation above records. Chrome's two Crowd repeats agree within 2.2%,
so this persistent increase cannot be explained solely by one unusually slow repeat.
The measurements do not isolate each change's causal contribution. Shared-host one-minute
load brackets range from 4.65 to 27.59; these are not quiet-machine qualifications.

The [Chrome CPU diagnostic](chrome-profile.json), with its [raw CDP profile](chrome-cpu-profile.json),
observes the unchanged final Wasm artifact over the full 2,000-body cost run. Its 3,247 samples
include 1,364 self samples in steering queries, 480 in shared intersection predicates and 425
in index builds. This locates the changed paths in Chrome itself and supports the hot-path
explanation above; samples do not isolate a causal share of the M1-to-M2 increase. The observed
tick median is 1.025 ms, close to the two unobserved rows, and does not replace them.

Mixed tick medians are 1.000–1.055, 2.295–2.350 and 14.360–14.545 ms at the three sizes.
Mixed has no same-neighbour-count M1 comparator. No Chrome row trips the clock flag or
the chronological repeat-regression flag. Scaling flags occur at 5,000 bodies in both
Mixed repeats and Crowd's first repeat: the same table-capacity step described above.
The second Crowd repeat's per-neighbour cost decreases slightly at that step, and it is
retained with its false flag rather than rounded into a scaling claim.

At 10,000 bodies, Chrome's eight-tick replay is 113.865–124.020 ms across both traces
and repeats. Like the Mac replay rows, this is planning evidence for SESSION, not a
claim that replay plus rendering fits one frame.
