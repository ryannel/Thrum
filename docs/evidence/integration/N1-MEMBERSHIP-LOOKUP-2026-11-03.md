# N1 four-peer membership lookup microprobe — 2026-11-03

## Decision tested

N1's 60 Hz extraction must map changed W1 IDs into each peer's partition.
The candidate has four peers, each with 128 interested IDs drawn from a
256-ID sparse u64 world. One, eight, 32 or 128 sorted changed IDs are looked
up in all four peers. The [retained Rust source](N1-MEMBERSHIP-LOOKUP-PROBE.rs)
compares sorted-row binary search, a linear sorted merge, `HashMap` lookup and
`BTreeMap` lookup. All methods produce the same checksum in each case.

Build: `rustup run 1.99.1 rustc -O -C target-cpu=native docs/evidence/integration/N1-MEMBERSHIP-LOOKUP-PROBE.rs -o tmp/n1-membership-probe/probe`.
Run each method/count in a fresh process, rotating method order across seven
passes. The [raw 112 samples](N1-MEMBERSHIP-LOOKUP-SAMPLES-2026-11-03.csv)
are retained. Tool: rustc 1.99.1; machine: MacBook Pro Mac15,11, Apple M3 Max,
arm64. Source SHA-256:
`0f528e11faab2af494bc2da8274eee7f71d08ec5c95b47705d14a7e7e22ea31c`.
For each changed count `1,8,32,128`, pass `r=0..6` invokes methods
`[binary,merge,hash,tree][(j+r)%4]` for `j=0..3`, each as
`tmp/n1-membership-probe/probe METHOD COUNT`; the CSV stores pass, method,
count, nanoseconds and checksum.

Median nanoseconds per **four-peer lookup pass** (seven process samples):

| Changed IDs | Sorted binary | Sorted merge | HashMap | BTreeMap |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 24.6 | 45.0 | 24.7 | 28.1 |
| 8 | 131.7 | 179.3 | 160.3 | 187.7 |
| 32 | 615.6 | 287.6 | 653.4 | 832.6 |
| 128 | 2121.2 | 567.8 | 2561.8 | 3196.5 |

The result supports a compact sorted table with sparse binary lookup and a
sorted merge when many IDs change. It does not select the dispatch threshold.
The protected fixed/structural W1 publication paths sort their changed IDs;
the ordinary path traverses its sorted `BTreeMap` change journal. No
table construction/update, W1 sorting, codec, journal, snapshot lease,
four-peer host callback, allocation or RSS
cost is included. The one-ID fixture intentionally targets a middle ID;
this prevents a first-row scan from making merge appear universally cheap.
No whole-path speedup or 0.5 ms/tick N1 service claim follows.
