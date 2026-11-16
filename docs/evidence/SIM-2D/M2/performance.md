# SIM-2D M2 cost investigation

## Refreshed Mac CPU measurements

The [current Mac record](costs-mac-metal.json) contains two complete release runs at
`d24658b33cd994512dbe0c36a3daa5b44eeb356c`, source key
`bdcda83e5678dc917bd65c8ff9d7fa5a665867de5186d38c7f7ca0e1825c0e44`.
The [source/completeness validator passes](cost-validation-mac-metal.json). All 24 rows
retain their load brackets, M1 comparators and second-repeat comparators. Warmed build
and all five query calls again observe zero allocations. No count or time gates success.

| Crowd bodies | M1 tick / index, ms | Refresh repeat 1 tick / index, ms | Refresh repeat 2 tick / index, ms |
|---|---:|---:|---:|
| 2,000 | 0.523417 / 0.026042 | 0.738042 / 0.090708 | 0.727167 / 0.090958 |
| 5,000 | 1.232208 / 0.075708 | 1.964041 / 0.242250 | 1.682000 / 0.239709 |
| 10,000 | 8.068417 / 0.165792 | 10.133542 / 0.493292 | 9.696083 / 0.477250 |

Five rows trip the M1 quarter-regression flag: serial Crowd at 2,000 and 5,000 in
both repeats, and 10,000 in repeat 1. Their accepted neighbour counts equal M1's.
Index increases are 65, 164–167 and 311–328 microseconds at the three sizes. The
eight-pass canonical ordering, shared shape predicate and conservative-bound work
described in the initial investigation below remain unchanged. That investigation
locates these paths, without assigning causal percentages to the tick increase.

The 10,000-body flag is 25.6% over M1 in repeat 1 and 20.2% in repeat 2. At 5,000,
the tick changes by 16.8% between repeats while the separate index measurement
changes by about 1%. First-repeat Crowd load brackets are 31.5–34.4 at one minute;
second-repeat brackets are 7.9–18.0. This supports contention as a contributor to
repeat variation, not a measurement of its causal share. Full-run one-minute loads
range from 7.88 to 42.23. These remain shared-host baselines.

The four scaling flags are serial 5,000-body rows of both traces in both repeats,
at the same 4,096-to-16,384 table-capacity step explained below. No row trips the
clock flag or the chronological quarter-slower flag. Parallel is retained whichever
way it performs: the first 2,000-body Mixed row costs 1.121 ms versus 0.797 ms serial;
the second costs 0.594 ms versus 0.731 ms serial. This does not establish a quiet-host crossover.

At 10,000 bodies, serial tick medians span 9.696–10.134 ms and forced-parallel
medians 2.267–2.468 ms. Eight-tick replay spans 81.073–110.682 ms serial and
20.945–62.386 ms parallel. The larger replay spread remains visible for SESSION.
This refresh binds the changed clock/presentation source; it does not isolate a
timing effect caused by those source changes.

## Refreshed Chrome Wasm measurements

The [current Chrome record](costs-chrome-webgpu.json) contains two complete release
runs at the same `d24658b33` commit and source key as the refreshed Mac record.
Its [source/completeness validator passes](cost-validation-chrome-webgpu.json).

| Crowd bodies | M1 tick / index, ms | Refresh repeat 1 tick / index, ms | Refresh repeat 2 tick / index, ms |
|---|---:|---:|---:|
| 2,000 | 0.570 / 0.035 | 1.010 / 0.105 | 1.010 / 0.100 |
| 5,000 | 1.355 / 0.100 | 2.310 / 0.270 | 2.225 / 0.265 |
| 10,000 | 8.580 / 0.205 | 14.810 / 0.540 | 14.005 / 0.530 |

All six Crowd rows trip the M1 quarter-regression flag at identical neighbour counts:
63.2–77.2% increases. The separate index increases are 65–335 microseconds. The
query/rule code and its canonical-ordering, shared-predicate and conservative-bound
paths are unchanged from the initial investigation below. Its historical samples
locate those paths; they are not samples of the refreshed executable and do not
assign causal shares to the increase. The two refreshed Crowd repeats agree within
5.5%. One-minute load brackets span 10.64–40.16; these remain shared-host baselines.

Mixed tick medians span 1.025–1.035, 2.355–2.360 and 14.285–14.430 ms at the three
sizes. Mixed has no same-neighbour-count M1 comparator. No row trips the clock or
chronological quarter-slower flag. Scaling flags remain at 5,000 bodies in both
Mixed repeats and the first Crowd repeat, at the table-capacity step described below.
At 10,000, ticks span 14.005–14.810 ms and eight-tick replay spans 117.090–119.520 ms.

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
