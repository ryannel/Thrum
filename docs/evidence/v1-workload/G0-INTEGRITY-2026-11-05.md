# G0 integrity and fixed baselines — 5 November 2026

Target: Apple M3 Max (Metal), verified by adapter identity in `tmp/g0-full-baseline-metal.log`. Reproduce with `RUSTC_WRAPPER= ./dev baseline-inventory`. It runs 1,000 CPU samples per S1/S2 variant and 600 offscreen frames per frame variant. Future rows with no consumer print `unsupported:<reason>`; no timing was invented for them. These baselines do not qualify G2S, G3b or G3t.

| Row, current recipe | Target p95 | p50 / p95 / p99 | Allocations |
| --- | ---: | ---: | ---: |
| S1 10k, four peers, shuffled, Damage + SetHealth | ≤25 µs | 71.6 / 74.3 / 76.7 µs | 0 |
| S1 100k, same | ≤300 µs | 700.8 / 772.3 / 995.3 µs | 0 |
| S2 10k, rewind8 + corrected peer input + branch recreation + 8 publications | ≤100 µs | 1,301.1 / 1,378.0 / 1,455.6 µs | 0 |
| S1 warmed 200-spawn burst, 10k live | zero allocations | 122.6 / 139.7 / 200.9 µs | 0 |
| G3b 10k/1080p offscreen host CPU minus tick | ≤300 µs for real frame | 574.7 / 1,385.3 / 2,481.6 µs | per-thread count pending |
| G3t 1080p offscreen GPU pass sum | ≤0.30 ms including present | 0.471 / 0.493 / 0.510 ms | n/a |

The S2 baseline uses the current branch-recreation path because retained-record amendment does not exist. It publishes all eight replayed ticks; the gate requires one final pair and a complete-state equality oracle. The frame rows use a device texture, not a drawable or presentation timestamps. A second 120 Hz request row submitted 600/600 offscreen callbacks; it does not prove a 120 Hz display interval. Its host-minus-tick CPU p50/p95/p99 was 372.0/398.0/452.9 µs. The first offscreen frame row had a large CPU tail under concurrent load; a prior run in `tmp/g0-full-baseline-final.log` measured 359.5/436.1/543.3 µs. Neither is a presentation result. Paired, interleaved S1 extra-command/damage rows are in the raw run; their ratio cannot rescue the failed absolute budget.

`./dev check` on native Metal passed with no `SKIP` or ignored test (`tmp/g0-check-all-device-timed.log`): all 23 previously ignored device tests, six host GPU tests and the depth/shadow regressions ran normally. Each requires an adapter; the focused host regression failed without Metal (`tmp/g0-no-metal-negative.log`). The iOS normal dependency tree contains no SQLite or authoring crate; final shipping links for other targets remain open. Observed check wall times were 29.41 s before cleanup, 6.89 s after cleanup with device tests still ignored, and 15.23 s with all device tests active; cache state and concurrent load differed, so this is not a controlled speedup claim. The cleanup removed 18,356 source-crate lines, 2,226 cook lines, 430 mobile-shell lines and a 315-line dead game test. At least 3,584 lines of obvious host/page/history/projection machinery still live in the game crate.

Independent review found an unused non-Mac import, a broken historical probe instruction, unpaired sensitivity figures, misleading whole-frame labels, an inventory that could accept missing Divan rows, an overclaimed G5 link guard, ignored device tests and an unverified benchmark adapter. Each was corrected and the raw inventory rerun. Real presentation and latency, Metal traffic counters, simulator function, content, transport and four-peer rows remain unqualified; the inventory names each missing consumer. No iPhone timing is recorded.
