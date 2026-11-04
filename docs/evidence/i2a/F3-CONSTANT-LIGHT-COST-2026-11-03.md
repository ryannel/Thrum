# F3 constant-light candidate cost — 3 November 2026

This review measures native **provisional candidate validation and manifest
revision after staging**, plus fresh-process CPU and RSS observations. It
extends the [constant-light closure proof](E1-CONSTANT-LIGHT-F3-2026-11-02.md)
under the [F3 source bounds](../../design/FOUNDATION-SPEC.md#first-engine-owned-environment-records--selected-next-f3-extension)
and [performance policy](../../design/AI-PERFORMANCE.md#the-cost-account-in-each-owning-design).
It does not qualify admission, edit-to-ready, cook, runtime, rendering or a device.

## Matched workloads and observations

All four workloads have **4,096 declared records, 4,099 staged files**, one
game descriptor, its two-field/one-version history, one game record and one
entry scene. Baseline reproduces the existing ignored probe's 4,095 tiny
schema-1 scenes and one game record. Selected replaces two scenes with one
constant-light source and light identity, and upgrades the entry scene to
schema 2. Unselected replaces all but the entry scene with **2,047 distinct
source/light pairs**, retaining the game record and selecting no environment.
Opaque uses Selected with 16 optional values, each an array of four
4,092-byte strings: **262,096 raw opaque bytes**, 80 opaque value nodes.
The per-value raw length is 16,381 bytes, below 16,384; this is a byte-heavy
case, not the 4,096-node or 16-level worst case.

Seven fresh processes per workload ran serially in rotating order, with no
cache flush or CPU affinity. Candidate elapsed time starts immediately before
`validate_staged_candidate` and ends after `provisional_manifest().revision()`.
Process CPU is `wait4` user+system time. RSS is macOS `wait4.ru_maxrss` in
bytes and includes fixture construction, staging, validation, manifest and
cleanup. Process CPU is **not** candidate-only CPU. Filesystem cache and other
processes are outside the RSS measurement; host scheduling was uncontrolled.

| Workload | Candidate ms min / median / max | Process CPU ms min / median / max | Process peak RSS B min / median / max |
| --- | --- | --- | --- |
| Baseline | 72.677 / 87.998 / 91.141 | 1,234.728 / 1,246.664 / 1,315.511 | 7,061,504 / 7,323,648 / 7,503,872 |
| Selected | 75.821 / 83.758 / 95.242 | 1,237.475 / 1,260.431 / 1,289.485 | 7,143,424 / 7,241,728 / 7,405,568 |
| Unselected | 80.608 / 84.774 / 98.346 | 1,294.414 / 1,329.611 / 1,337.830 | 8,011,776 / 8,208,384 / 8,486,912 |
| Opaque | 75.112 / 79.421 / 92.497 | 1,208.195 / 1,239.197 / 1,285.575 | 7,651,328 / 7,946,240 / 8,192,000 |

Timing ranges overlap. These observations establish neither a speedup nor a
one-pair regression. Many unselected declarations remain paid validation work;
they do not activate a Sun. No latency or RSS threshold was preselected, so
these samples cannot award a performance target or a full F3 memory gate.
Stage-capture median elapsed times were 581.779 / 567.237 / 583.538 / 575.847 ms;
whole-process median elapsed times were 1,281.474 / 1,298.891 / 1,360.269 /
1,274.004 ms, in table order. Neither includes real authoring or durable admission.

| Workload | Exact copied file bytes | Largest JSON file B | Largest environment B | Required edges |
| --- | --- | --- | --- | --- |
| Baseline | 488,821 | 127,266 | 0 | 1 |
| Selected | 489,465 | 127,276 | 589 | 3 |
| Unselected | 1,686,316 | 147,736 | 589 | 2,048 |
| Opaque | 752,041 | 263,165 | 263,165 | 3 |

## Scaling, repeated work and byte account

One ordinary selected environment adds fixed scalar/numeric validation and two
required links. This is per **candidate attempt**, with no per-frame or
per-light runtime work in this path. Every attempt rereads and rehashes all
declared registry/record JSON, even if one field changed. With R records,
S descriptors, F staged files and E edges, current membership/file lookup is
binary search, namespaces sort borrowed IDs, and typed closure uses binary
search: roughly input-byte work plus `R log F + R log R + E log R`, with
registry/history validation and manifest construction additional work.
There is no production all-pairs environment/light search. The linear-search
closure helper in `environment_json.rs` is test-only.

`registry_stage::validate_staged_records` reads one reusable record buffer,
then `environment_json::record_header` walks the entire JSON before the typed
parser walks it again. The dispatch traversal enforces complete-record node
and depth bounds; dropping it without equivalent checks is invalid. Opaque
values additionally undergo RawValue boundary scanning and `check_opaque`
validation, with per-object `BTreeSet` duplicate-key checks. The release parser
borrows raw opaque bytes and does not retain copies; copies in `extensions`
are `cfg(test)` only. Header top-level keys, IDs and types, and typed parser
keys/text, allocate owned strings; skipped header member strings are discarded.
Escaped strings may need serde scratch. No allocation-count claim was measured.

The full path also copies/hashes source files during staging, rereads/hashes
the working files for drift, and rereads/hashes staged JSON during validation.
`StagedTree::open` repeats a binary membership lookup after the validator's
lookup. Manifest creation clones paths and ID/type labels, checks path/UUID
uniqueness again and sorts entries; revision streams canonical bytes into
SHA-256 without a whole-manifest JSON buffer. These are concrete repeated costs,
not evidence that removing a check is safe or useful.

At the first profile's 4,096-record cap, one scene and one shared light permit
at most **4,094 environment sources**; distinct source/light pairs with the
scene and game used here permit 2,047. This review measures the latter, not
all maximum shapes. There are at most 256 game descriptors, 65,536 required
edges, 16 MiB/1,000,000 value nodes/64 container levels per JSON record,
and 100,000 tree entries including directories. A bound's Cartesian product
is not a promised admitted project: 4,096 × 16 MiB is 64 GiB, while staging's
temporary bucket is 2 GiB including projected allocation/metadata and existing
temporary usage. Source opaque bytes could approach 4,094 × 256 KiB
= **1,073,217,536 B** before other source and staging limits. None of that
combined byte/edge/history/tree peak was measured here.

On this arm64 build, `size_of` reports StagedFile 64 B, StagedScene 112 B,
StagedBuiltin 104 B, StagedGame 136 B and ManifestEntry 112 B. Thus these
fixtures have 262,336 B of stage-file element payload, baseline scene payload
458,640 B and manifest-entry payload 459,088 B. Manifest labels carry
4,099 × 40 = 163,960 B; the baseline simultaneous borrowed all-ID and scene-ID
elements carry 131,104 B. Maximum all-ID element count with 256 descriptors
is 4,354 × 16 = 69,664 B. These are modeled element payloads, **not allocator
capacity/heap peaks**, and exclude owned strings, hash tables and allocator
overhead. Staging uses a separate 4 MiB copy buffer which is dropped before
candidate validation. Candidate record storage retains a reusable buffer at
least the largest record length plus its one-byte drift sentinel; capacity
growth and serde scratch prevent treating that as a hard heap upper bound.

## Identity, reproduction and disposition

Host: Apple M3 Max MacBook Pro, 14 CPU cores, 36 GB RAM, macOS 26.6.2
build 25G83, arm64. Toolchain: Rust 1.99.1 (`58cf8f9c35b355199727627d07ae34a4e2846d67`,
LLVM 23.1.1), Cargo release defaults, no RUSTFLAGS, `RUSTC_WRAPPER=`.
The harness uses the normal release library, not a Rust unit-test executable.
Source is HEAD `05184410ccdee3494c9558822f98306c4081f277` plus the snapshotted
working source. Key SHA-256 identities:

```text
environment_json.rs 81c74636bedd08aaf7b2eeaf44d78fba779febc75446bca267298b792f695ced
registry_stage.rs   dce42689de21f6e4fdec479777d1b46da6fe750e27c64dddbcc39610f10e70c3
game_json.rs        59d7b88fb59b6a6d55cdb377da4eabab5f9ce52e6264e34822d3c00bf4bb12b7
harness main.rs     a74d64d657d07caf23654889b175d81617899a8e287a94d0c1bf9b85567e3336
harness Cargo.lock  d6b6cb729639be4a7e30146446a3d1b48e09546ce59d80e7465c54dec93f4804
release executable  c3074aa6d48272955967f0b1eec496639ed7c1fb76b2194459748fcc84ec6ebe
```

The retained [proof archive](../ARCHIVE-ACCESS.md#archive-inventory)
(SHA-256 `b5c38c257caaed97de0c6d3101ab64e6e7aae2a8bb9818c08321eceec0521cef`)
contains the owned local recipe: `src/main.rs` generates deterministic files,
`run.py` collects every sample and full source hashes in `results.json`, and
`source/` freezes the measured source. Extract it under `tmp/` to recreate
`tmp/f3-constant-light-performance/`, or use the existing local tree.
To reproduce:

```sh
RUSTC_WRAPPER= rustup run 1.99.1 cargo build --locked --offline --release \
  --manifest-path tmp/f3-constant-light-performance/Cargo.toml \
  --bin f3-constant-light-performance
python3 tmp/f3-constant-light-performance/run.py
```

Each workload's seven outputs have the same provisional revision, and assert
record, built-in, scene, game and required-edge counts. Scoped release tests
against the frozen library passed **11 environment-parser tests and 15
registry-stage tests**, with eight unrelated cost probes ignored. The harness
passes rustfmt and Python compile checks. No production code changed in this
review; document checks are a separate repository gate.

**Disposition:** retain the current bounded algorithm; no material algorithmic
performance bug was found in production constant-light closure. Reject any
promotion to maximum-source memory qualification or edit-to-ready performance.
Before that gate, select target budgets and jointly measure maximum history,
65,536 edges, full tree metadata, escaped strings, live prior/current candidates
and admission/cook consumers. Further optimization needs matched candidate-only
allocation/CPU attribution and unchanged rejection semantics. These local
samples justify no specialization or public restriction change.
