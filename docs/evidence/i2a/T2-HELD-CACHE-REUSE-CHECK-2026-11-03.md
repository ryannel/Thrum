# Held private-cache reuse check through a real A/B build

3 November 2026. An opt-in macOS 26.6.2 ARM64 qualification passed at exact
checkout commit `11e559ed9fec2b12772427c7f3cd85456adb5d81` with
`python3.11 scripts/qualify_held_macos_native_ab.py`. The full
`RUSTC_WRAPPER= ./dev check` passed at that commit: 113 Python tests (three
skipped), the Rust workspace check, Clippy and tests. A prior uncommitted
integration run passed before the response protocol was tightened; the
committed run is the recorded result.

The persistent Python guard cold-opens the sealed private cache and selected
Rust tool root under one process, checks the Cargo/Rustc paths, sizes and bytes
against the sealed cache manifest once, and keeps both shared leases open.
For each admitted A/B source, the Rust fixture sends a bounded request with
an increasing ID. The guard recomputes the dependency-selection key from the
materialized source, compares it with the exact sealed manifest and returns
that key with the same ID. Rust requires the exact selected digest and
canonical response shape. Neither warm request inventories the cache or
rehashes Cargo/Rustc. The real runner requires exactly two successful held
reuse observations and zero legacy full-rehash reuse observations.

The two held request round trips and source-key checks reported **2 ms and
1 ms** in this uncontrolled run. The private cache retained the same legacy
SHA-256
`58c57419b000f0dbb2037792932f0191abecceae9e14e77e4b384fb06811225b`
and separate tree SHA-256
`1b20de230cb247e419b476abf1c30688940572a8193e08be58c05e06e95e8ec1`
for 2,923 files/67,792,387 logical bytes. Cold cache preparation took
1,575 ms; guard cold open took 913 ms, now including Rust-root verification
and selected-tool byte checks. The A/B process took 24.166 seconds, and
post-build full inventory of the four selected roots took 10.837 seconds.

A again produced five successful compiles and three probes, with receipt-set
SHA-256 `f43755dbf39bb23eecbc8172ca668c4b3139aaf4b619c64cbfee1136b012ed25`.
The retained unready A candidate SHA-256 was
`76491e7426d52049f61c30a565f3143ded95b9d76a121e461fdc51d37a649394`;
fresh reload took 28 ms. Invalid B produced three successful compiles, one
expected failed game compile and three probes, with receipt-set SHA-256
`cb3fe535391826057b1f593866b14ab61c235df1e2a0f03f9e4aaf6539ebbf05`.
A remained runnable and the runner cleaned its owned scratch.

Six small macOS cache fixtures cover a code-only edit retaining the key,
dependency manifest and target changes refusing reuse, malformed selection,
wrong Cargo path/hash, lease closure, directory/manifest drift, partial
publication and two readers. A separate Rust response test refuses null,
wrong, stale, extra-field, reordered and malformed replies. The independent
review found and closed a prior response-acceptance gap that treated any
`reuseKey` member as success.

This is a **selection-check** cost observation, not a complete edit-to-ready
speedup. The A/B qualification still calls explicit cache and tool byte hashes
before/after the builds and runs full post-build inventories; cold preparation
is also outside a code-only warm edit. The guard's death during a compiler
job, interrupted requests and complete T2 owner/worker failure handling have
not been qualified. Origin review, Xcode support and host-runtime closure,
physical allocation, memory/RSS and representative timing tails remain open.
No ready `BuildRevision` is claimed; the runner reports `buildReady:false`.

## Held build scan removal

At exact local source commit `cc95a984d32b325ec855a8781804c273c749deb4`,
the opt-in admitted A/B qualification passed again. The Rust fixture now
skips its two explicit whole-cache `tree_hashes` calls and separate pre/post
Cargo/Rustc byte assertions only when its persistent sealed-cache/Rust-root guard
exists. Other private homes and non-macOS paths still take the full checks.
The runner requires `fixture_cache_full_rehashes=0`, exactly two held reuse
responses and no legacy reuse marker. It still independently inventories all
four selected roots after both jobs before deleting owned scratch. The
compiled build path still hashes selected compiler, linker and support files
before and after each Cargo job.

On this one host run, the cache held 2,923 files and 67,792,387 logical bytes
with the same legacy and tree digests shown above. Cold cache preparation took
1,604 ms and the combined guard cold open took 935 ms. The two source-key
checks again reported 2 ms and 1 ms. A produced five successful compiles and
three probes, retained an unready candidate with SHA-256
`dd0c6724c94db58534604b718fb029ce16ad5b2131d3492e1045471d6811a504`,
and reloaded it in 26 ms. Invalid B produced three successful compiles, one
expected failed game compile and three probes; A remained runnable. The
overall A/B qualification process took 25.448 seconds, including work other
than the guarded cache checks. The outer post-build four-root inventory took
11.690 seconds. Scratch was cleaned on success. `RUSTC_WRAPPER= ./dev check`
passed at the code commit; independent review found no issue.

The removed scans avoid repeated reads and allocations over the 67.8 MB
private cache in this candidate. The preceding run paid roughly 150–230 ms
for one full cache rehash; these uncontrolled end-to-end runs do not isolate
a speedup. The outer qualifier's full inventories are cold evidence checks,
not the warm edit path. The guard is still a cooperating child rather than
one reviewed T2 selection owner, and compiler/support/runtime closure,
guard-death during a job, warm edit-to-ready tails and RSS remain open.

## Real materialized lockfile drift

At exact local source commit `f9c167a16f4d7315d62c11fbf627d2532b353add`,
the opt-in admitted A/B run passed with an additional refusal after B's
compile and receipt checks. The test appended one newline to B's owned,
materialized `Cargo.lock`. That changed its raw lockfile digest and the
dependency-selection key; the persistent held guard returned the specific
`held-cache owner refused reuse` error. The qualifier required the
`real_materialized_lock_drift_refused=true` marker, exactly two earlier
successful 1 ms/1 ms reuse checks, and zero fixture whole-cache rehashes.
The guard closed normally, A's retained candidate still ran after invalid B,
all four roots re-inventoried unchanged, and owned scratch was cleaned. The
overall A/B process took 21.609 seconds; the outer post-build inventory took
11.096 seconds. The normal `RUSTC_WRAPPER= ./dev check` passed at the code
commit, and independent review found no issue.

This proves refusal on a real changed materialized dependency input between
jobs. It does not prove that a write during an active compiler is observed or
contained, and it does not turn the cooperating guard into one reviewed T2
release owner. The retained A candidate remains unready.
