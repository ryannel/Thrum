# SIM-2D M2 cost investigation

## Mac CPU

[The complete record](costs-mac-metal.json) contains two release runs at
`9d06e07c731bfbc7253a9ea04b6de2cd2ca9b189`, source key
`b2ae5a08c3585081315bcbecb8368247cf69afacd37868c310f0f8660cf9f613`.
Every row runs the full trace, records its load bracket, and retains its M1 and chronological
comparators. These are shared-host baselines. No time or allocation count gates success.

The four quarter-slower flags are the serial Crowd rows at 2,000 and 5,000 bodies in both repeats.
Their accepted neighbour counts equal M1's. Against M1's latest repeat:

| Bodies | M1 tick / index, ms | M2 repeat 1 tick / index, ms | M2 repeat 2 tick / index, ms |
|---|---:|---:|---:|
| 2,000 | 0.5234 / 0.0260 | 0.7363 / 0.0905 | 0.7110 / 0.0900 |
| 5,000 | 1.2322 / 0.0757 | 1.6590 / 0.2409 | 1.6134 / 0.2373 |

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
