# E1 pinned constant-light cook cost probe — 3 November 2026

This bounded probe tests whether complete manifest authentication and three linear selection scans create consequential cost when a small selected constant-light closure lives in a larger legal declared tree. It called production public APIs at the time of measurement; those APIs were removed from the active tree during the code-quality reset and remain in Git history. It did not select a universal budget.

## Identity and method

- Apple M3 Max, MacBook Pro Mac15,11, 14 CPU cores (10 performance + 4 efficiency), 36 GB; macOS Darwin 25.6.0, arm64. Live desktop, no CPU affinity, power/thermal control or exclusive-machine claim.
- Repository HEAD at build: `9e088154da316fb377078399e6bfcc6b316f4cf5`; unrelated working-tree changes were preserved. The owning API source hashes below remained unchanged before/after measurement.
- Rust 1.99.1 `58cf8f9c3`, aarch64-apple-darwin, LLVM 23.1.1; `cargo build --offline --release`; exact dependency lockfile retained. `RUSTC_WRAPPER=` bypasses a sandbox-incompatible sccache wrapper.
- Standalone scratch Cargo package depends on production `thrum-source` and `thrum-cook` path crates. macOS `getrusage(RUSAGE_SELF)` measures user/system CPU and process lifetime high-water RSS; `Instant` measures elapsed time. Unsafe FFI is measurement-only harness code.
- Five fresh release processes per fixture, interleaved small then large. Each opens the same already admitted fixture store, recovers A/B via durable outcomes, times pin A then cook A, keeps A held while timing pin B then cook B, then measures repeated cooks with both pins held. It closes B, times repeated `pin_accepted(B)` calls with each close excluded from the pin timer, then times the complete pin+cook+close path while A remains held. Every process verifies A is still 100000 lux and B is 200000 lux; A is cooked again after B activity.
- Small repeated groups use 1000 calls; large groups use 100 calls. Each table value is a median of five per-process averages, with the range across those five averages. First-call rows are five individual fresh-process observations. CPU is user+system; raw JSON retains each separately.
- RSS is the process high-water mark, not a phase allocation total, current resident memory, or an isolated incremental pin charge. Phase timing excludes fixture generation/admission, store open, root recovery, metadata reporting and initial setup except where named. Full-path rows include pin authentication, four copies, typed parse, cook, product drop and durable pin close.
- Fresh process does not mean cold filesystem cache: stores were generated/admitted first, OS cache was not evicted, and SQLite/open initialization has its own separately retained row. Fixed A-before-B ordering permits warmup effects; it does not establish an A/B performance difference.

## Fixtures

The six-file small tree is derived from the production `constant_light_source.rs` test fixture: project, selected schema-2 scene, authored environment, light identity, empty registry history, and unrelated note. A/B differ in authored lux and unrelated note bytes.

The large tree adds 4093 legal, unique, unselected `thrum.light.identity` records with sorted `records/aNNNN.record.json` paths. It has 4096 project-declared records and 4099 manifest entries. Fillers occur before the selected environment/light and selected scene, exposing nearly three full manifest scans. This is a legal count/path stress fixture, not representative artistic content. All candidates pass the real staging/registry validation and durable `admit` path.

| Quantity | Small | Large |
| --- | ---: | ---: |
| Manifest entries | 6 | 4099 |
| Canonical manifest bytes | 1350 | 918185 |
| Selected bytes copied per cook | 1299 | 115903 |
| Project bytes copied per cook | 357 | 114961 |
| Identity entries visited per cook (scene + environment + light) | 5 + 3 + 4 = 12 | 4098 + 4096 + 4097 = 12291 |
| Identity scan visits / total manifest entries | 2.0000× | 2.9985× |

These visit counts reproduce the three production `.find` traversals from the authenticated manifest outside the timed region; they are operation counts, not instrumented per-scan timing. Project lookup and four copy lookups are separate binary searches. Every cook copies only four selected files, but the selected project contains all declared paths, so copied bytes and project-path parsing also grow. The larger cook delta cannot be attributed to manifest scanning alone.

## Results

Elapsed and CPU are microseconds per API call / complete path. Brackets contain observed min–max across five fresh processes.

| Phase | Small elapsed | Small CPU | Large elapsed | Large CPU |
| --- | ---: | ---: | ---: | ---: |
| First pin A | 13521.2 [4763.5–13946.2] | 415.0 [220.0–591.0] | 9733.9 [9633.6–13614.8] | 6080.0 [5938.0–9150.0] |
| First cook A | 230.3 [170.1–236.5] | 230.0 [171.0–237.0] | 907.8 [789.3–1310.5] | 908.0 [790.0–1311.0] |
| First pin B with A held | 4621.4 [3904.9–4934.9] | 166.0 [119.0–221.0] | 10677.3 [9754.8–12627.1] | 6452.0 [5634.0–8325.0] |
| First cook B with A held | 74.1 [63.6–76.9] | 74.0 [64.0–78.0] | 818.6 [753.3–1098.6] | 819.0 [754.0–1066.0] |
| Repeated cook A with A+B held | 36.6 [33.2–39.8] | 36.6 [33.2–39.7] | 740.4 [713.7–780.7] | 739.5 [709.5–779.1] |
| Repeated cook B with A+B held | 34.0 [32.6–35.0] | 33.8 [32.6–34.7] | 627.5 [620.4–635.1] | 627.0 [619.7–634.4] |
| Repeated pin B, close excluded | 4138.6 [4101.5–4272.7] | 137.7 [115.3–210.0] | 10172.4 [9635.5–10976.3] | 6261.9 [5717.1–6734.6] |
| Full B pin+cook+close, A held | 8485.6 [8112.0–8599.2] | 419.8 [241.9–535.7] | 15481.4 [14972.5–16129.2] | 7320.3 [7109.9–7727.4] |

Process peak RSS after complete measured sequences: small **4.281–4.359 MiB**, large **14.172–17.203 MiB**. After first A pin it is 3.719–3.812 MiB small and 8.672–8.719 MiB large; after B pin with A retained it is 3.812–3.922 MiB small and 12.156–12.875 MiB large. These high-water observations show overlap footprint but do not isolate allocator or individual manifest ownership.

## Interpretation and limits

The concrete scale risk is present: this legal larger tree makes repeated selected cook roughly 18–21 times the small fixture cost even though the lighting payload remains 160 bytes, and a new accepted pin separately authenticates the complete manifest plus every complete-file header. It does not load/copy all source file bodies during pinning. Large repeated B cooking remains about 0.63 ms CPU on this machine; repeated pinning costs about 6.26 ms CPU before close, so pin lifecycle/authentication deserves attention alongside the three scans if a future workload repeatedly opens roots.

Elapsed pin and full-path costs materially exceed CPU cost. Code inspection identifies durable pin-create and pin-release transactions; the probe has not traced fsync or attributed waiting to individual system calls. Do not call the elapsed–CPU gap a measured disk-only cost. Keeping an already authorized pin through a bounded batch changes the amortization, but no new lifecycle/API policy is proposed here.

The fixture does not measure 100000-manifest-entry limits, 16 MiB selected records, large opaque selected fields, large history/schema sets, concurrent admission, multiple threads, malformed/rejected input, process interruption, browser/mobile targets, cold storage, editor edit-to-ready publication, caching, or a renderer/runtime. It supplies no universal latency/RSS budget and no end-to-end engine qualification. No source optimization, production API change, or indexed alternative was evaluated.

## Reproduce and retained evidence

Extract the [retained harness and raw evidence](../ARCHIVE-ACCESS.md#archive-inventory) into `tmp/e1-pinned-cook-perf/`. Run from the repository root (the fixture store path must be absolute):

```sh
RUSTC_WRAPPER= CARGO_TARGET_DIR=tmp/e1-pinned-cook-perf/target rustup run 1.99.1 cargo build --offline --release --manifest-path tmp/e1-pinned-cook-perf/Cargo.toml
tmp/e1-pinned-cook-perf/target/release/e1-pinned-cook-perf init /Users/ryannel/Workspace/thrum/tmp/e1-pinned-cook-perf/small 3
tmp/e1-pinned-cook-perf/target/release/e1-pinned-cook-perf init /Users/ryannel/Workspace/thrum/tmp/e1-pinned-cook-perf/large 4096
python3 tmp/e1-pinned-cook-perf/run.py
python3 tmp/e1-pinned-cook-perf/summarize.py
```

`init` is for absent fixture databases; use new owned fixture directories, or remove only these owned generated fixture directories before regeneration. The retained harness and runner fully specify selected inputs, ordered measurements and raw samples. `raw.jsonl` contains 120 JSON rows for ten fresh measurement processes; `summary.txt` records all phase ranges including store open and pin+close container overhead. `init-small.jsonl` and `init-large.jsonl` record accepted revision identities. Generated fixture stores and build/cache outputs are excluded from the retained archive and can be regenerated.

SHA-256 identities:

- `crates/thrum-cook/src/constant_light_source.rs`: `f5d28f6f0384333c7fcedf69775505049cbaf9d0499ee16a26f787ca2c0f1493`
- `crates/thrum-source/src/sqlite_store.rs`: `79081b2c7db0032b8038db417a5c879f4082996c0efcdf7724c23000cf957f1c`
- `crates/thrum-source/src/project_v2_json.rs`: `d26f9bf32b955623b8fe0b5cc9f82670bab480eb3922279dcbc807403b7e6ea7`
- `tmp/e1-pinned-cook-perf/src/main.rs`: `a96ac8f6d10251edcfb09a5f0e932b2bcb5aecff8de361a6f846b2e94076390a`
- `tmp/e1-pinned-cook-perf/target/release/e1-pinned-cook-perf`: `b318fbf103b1ac485951f20c20b85cd1044dcefe3778bfb5fb49d74ddac418e6`
