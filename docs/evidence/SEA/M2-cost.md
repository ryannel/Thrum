# SEA M2 cost on this Mac

Measured 14 November 2026 on source `bd36ca70edf90e7cb216d4029e9fc7c28875d593`,
Apple M3 Max / Metal, Mac15,11, 14 CPU cores, 36 GiB shared memory. The GPU lock
was held. The machine's load averages were 4.41 / 3.62 / 3.43; `pmset -g therm`
reported no recorded thermal or performance warning. The [complete target record](M2-initial-ambient.txt.gz)
contains the adapter, limits, source, load, every resource and both frame samples.

Workload: wind 20 m/s, direction 0.41, fetch 1,000 km, no swell; 192 long components,
three 256-square cascades, eight 128-square cache levels, choppiness 0.1 and 4,096
queries. These are real per-pass timestamps in one frame encoder at display time
86,400.00625 seconds, after the first-minute frame. No surface is drawn in M2.

| Simulate pass | GPU milliseconds in that frame |
| --- | ---: |
| sea long waves | 1.797667 |
| sea cascade evolve | 0.084750 |
| sea cascade rows | 0.311375 |
| sea cascade columns | 0.352208 |
| sea cascade unpack | 0.086000 |
| sea moments mip 1 | 0.016583 |
| sea moments mip 2 | 0.006292 |
| sea moments mip 3 | 0.004000 |
| sea moments mip 4 | 0.004667 |
| sea moments mip 5 | 0.003625 |
| sea moments mip 6 | 0.003875 |
| sea moments mip 7 | 0.003625 |
| sea moments mip 8 | 0.003875 |

These are individual frame samples, not medians or panel cadence. In the retained
first-minute frame, the final mip's stage timestamps are equal, reporting zero;
that does not mean zero work. The table uses the complete nonzero 24-hour sample.
An attempted extra observer also encountered an equal stage sample and was removed;
no replacement counter or inferred zero-cost claim enters this record.

| GPU resource | Descriptor bytes |
| --- | ---: |
| sea parameters | 176 |
| sea spectrum / final transform | 17,301,504 |
| sea long components | 6,144 |
| sea query points | 32,768 |
| sea transform A | 17,301,504 |
| sea transform B | 17,301,504 |
| sea cascades | 9,437,184 |
| sea moments base | 6,291,456 |
| sea long field | 6,488,064 |
| sea moments mip 1 | 1,572,864 |
| sea moments mip 2 | 393,216 |
| sea moments mip 3 | 98,304 |
| sea moments mip 4 | 24,576 |
| sea moments mip 5 | 6,144 |
| sea moments mip 6 | 1,536 |
| sea moments mip 7 | 384 |
| sea moments mip 8 | 96 |
| sea upload staging | 6,330,544 |
| sea mip 0 parameters through sea mip 7 parameters, eight separate buffers | 16 each, 128 total |
| **All 26 allocations** | **82,588,096 (78.762 MiB)** |

These are requested buffer bytes, including staging. There are no sea textures.
Driver allocation rounding, compiled pipeline memory and diagnostic readbacks are
outside that total. Upload reuses a 6,330,544-byte CPU arena and copies that many
bytes per frame; the larger spectrum allocation becomes the final transform output.

Compared with the [design's estimates](../../plans/SEA/DESIGN.md#cost-account):
the cascades and mips total 0.881 ms against about 0.4 ms. This implementation
transforms eleven separate complex channels, including all displacement rates and
differentials, rather than packing pairs of real channels. The long-wave pass is
1.798 ms against 0.2–0.3 ms: its world-point height oracle requires repeated
horizontal-inversion component sweeps, plus the final differential/velocity sweep
and the query batch. The estimate counted roughly one field evaluation.

Memory is 78.762 MiB against about 15 MiB: the three unpacked transform worksets
alone are 49.5 MiB, with 9 MiB of cascade products, 8 MiB of moments and about
6 MiB each for long products and staging. Separate mip allocations give each graph
resource one producer. These explain the estimate overruns; they are measurements
to carry into WATERS' integrated cost account, with no promoted acceleration or
full-frame performance claim.
