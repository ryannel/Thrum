# Admitted A/B build under held macOS tool roots

3 November 2026. One opt-in macOS 26.6.2 ARM64 run from checkout commit
`61e69375e8600f031822397710b0e3b98a372b9d` passed with
[`qualify_held_macos_native_ab.py`](../../../scripts/qualify_held_macos_native_ab.py).
It is a local **unready** integration of the
[sealed-root candidate](T2-HELD-TOOL-ROOT-CANDIDATE-2026-11-03.md) and the
existing F3/F4 admitted A/B build fixture.

The runner inventoried and sealed fresh local Rust 1.99.1, Xcode 27.2 linker
and macOS SDK roots under one private owner. It opened a shared cold-verified
lease for each root and held all three while the existing opt-in Rust test
prepared its private Cargo home, compiled A, reconciled receipts, published
and reloaded A's unready output candidate, admitted invalid B, checked B's
failed compile, and reran retained A. The test released its F3 source-store
owner before its fresh verifier child reopened the accepted source, then
reacquired the owner before B admission. This is the necessary single-owner
restart boundary; the earlier attempt correctly failed with `OwnerBusy`
before that fixture correction. The outer runner used a bounded process-group
collector and explicit ARM64 wrapper target. After the test, it rehashed all
three held roots, closed their shared leases while retaining the owner-wide
exclusive lock, acquired per-root exclusive cleanup locks, then removed only
its owned probe directory.

The exact selected local root SHA-256 digests were Rust
`00d56ad367ace4cdbae132ce52f66a0cb49ef7f7b432dc1fd493421b947ae368`,
linker
`cb8dbb37004fbbc31eb5de9401c5c57f3019831d9c440add9675ed1e9b9d8c3d`,
and SDK
`e92b280633607b7d71d0807c13d18d33255665bbb5fbf89a37aec9920bb247f0`.
The A receipt audit reported five successful compiles and three probes with
set SHA-256 `9d90d5f7a0ba50deb427baf0ed316ffd497355dd3a3b6630b85404f8ac96fb4c`.
The retained A candidate SHA-256 was
`54efa9245e3bcd2c720c24e8142cebbc90fb9dc04e435dda05aa80dbad1c1519`.
B reported three successful compiles, one expected failed `i1_mobile_game`
compile and three probes with receipt-set SHA-256
`635a279da4154d21d99b23eb608921795637731787e9e30735417f8ac3a6bdf4`.
A's retained reload took 26 ms in the test's uncontrolled timing.

| Stage | Rust | Linker | SDK |
| --- | ---: | ---: | ---: |
| Installed-source full inventory | 0.600 s | 5.879 s | 5.714 s |
| Fresh clone, seal, fsync, rehash and sidecar | 1.105 s | 14.544 s | 22.934 s |
| Shared cold open with full seal/byte verification | 0.471 s | 5.339 s | 7.108 s |

The admitted A/B test process took 22.536 seconds; post-build full inventory
of all three held roots took 10.707 seconds. These are one-host, uncontrolled
integration timings. They do not isolate build speed, memory/RSS, physical
allocation, concurrency gains or edit-to-ready tails. Warm builds should hold
the already verified shared leases instead of repeating cold inventories.

The first attempt ran inside a tool session that denied `sandbox-exec` itself;
it stopped before compilation and preserved its sealed roots. A resumed run
outside that restriction reached A's fresh verifier child and exposed the
source-store owner-lifetime bug above. After the fix, a resumed A/B run passed,
then the final fresh committed run above passed. Each failure left the prior
candidate unready and the owned sealed scratch intact; the successful runs
cleaned their scratch after verification. The runner is opt-in; ordinary
`./dev check` does not run this full native build.

This proof holds only the three tool roots. Its private dependency cache,
selected Xcode support and host-runtime reads are not part of one reviewed
retained selection; the local root digests have not been approved in a release
manifest. The user's first-profile compiler-family no-detach assumption is
selected but its exact release helper set is not yet reviewed. An outer
qualification timeout can kill the test's supervisor before that supervisor
cleans its separately sessioned inner Cargo, so the runner preserves failed
scratch for diagnosis instead of claiming complete nested-job teardown.
No complete compiler/system-read closure, durable ready publication or
`BuildRevision` is claimed; `buildReady:false` remains mandatory.

## Pinned Cargo cache bookkeeping probe

At committed checkout `09a244bf6612168ba5af7e425c16b2ae86ca25b9`,
the same opt-in command passed again on macOS 26.6.2 ARM64. The fixture now
asserts that both `.global-cache` and `.package-cache-mutate` are absent from
the freshly prepared private Cargo home before A. The compiler sandbox denies
all `CARGO_HOME` writes, and the existing final file-byte comparison found
the cache unchanged after successful A and expected compile-failed B. A
produced five successful compiles and three probes; B produced three
successful compiles, one failed `i1_mobile_game` compile and three probes.
All three held tool-root inventories were unchanged after the build, and the
runner cleaned its owned scratch on success. The admitted A/B test process
took 20.351 seconds and the three-root post-build inventory 10.577 seconds;
these uncontrolled values are integration observations, not cache-speed data.
`RUSTC_WRAPPER= ./dev check` also passed at the assertion change.

This establishes that the pinned Cargo version tolerated **absent** cache
bookkeeping files when its private home was read-only to the compiler for
this A/B fixture. The home itself was not sealed or held by a cache lease;
metadata reuse from a sealed home, drift refusal and the warm fast path remain
open. The runner still reports `buildReady:false`.
