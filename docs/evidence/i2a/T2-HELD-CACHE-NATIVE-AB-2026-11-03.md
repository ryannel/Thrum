# Admitted A/B build from a held sealed private Cargo cache

3 November 2026. One final opt-in macOS 26.6.2 ARM64 run passed at exact
checkout commit `ab4e6ba91f5c530d53f69ad397ed267aaf62556d` using
`python3.11 scripts/qualify_held_macos_native_ab.py`. The full
`RUSTC_WRAPPER= ./dev check` passed after the rebase onto the first source
capacity catalogue. An earlier uncommitted integration trial also passed;
the committed run below is the recorded checkpoint.

The existing test prepared a writable private Cargo home from the admitted A
source, completed both locked offline metadata passes and pruning, then cloned
and sealed it under the same private scratch owner as the three tool roots.
The new cache guard cold-opened the selection and held a shared root lease
through A compilation, receipt reconciliation, retained candidate publication
and fresh reload, invalid B compilation, and the final A execution and cache
comparison. The compiler used the sealed clone as `CARGO_HOME`; its sandbox
denied cache writes. Before A, `.global-cache` and `.package-cache-mutate`
were absent. The test continued to verify the complete dependency selection
key and rehash the selected cache before A and B. After the test guard exited,
the outer runner re-inventoried the cache and three tool roots, acquired each
exclusive lock, and cleaned only its owned scratch.

The cache's legacy `thrum.cargo-home.v1` SHA-256 was
`58c57419b000f0dbb2037792932f0191abecceae9e14e77e4b384fb06811225b`;
its separate directory-complete tree SHA-256 was
`1b20de230cb247e419b476abf1c30688940572a8193e08be58c05e06e95e8ec1`.
It held 2,923 files totaling 67,792,387 logical bytes. Cold cache
preparation, including source checks, clone, seal and rechecks, took 1,550 ms;
the guard's cold verified open took 363 ms. The admitted A/B test process took
24.444 seconds, and post-build full inventory of the four selected roots took
10.899 seconds. These are one-host uncontrolled integration observations;
they do not measure memory, physical allocation, a warm edit path or a
speedup.

A's receipt audit reported five successful compiles and three probes, with
receipt-set SHA-256
`78b6d97d4bb3d01b9d7b7a3db07890189fb3a4a8c7004ac14921b1f03e0f4c08`.
Its retained unready candidate SHA-256 was
`ba41e290314d59d9e511c306ca5d2216e200be4d95c8aaf5d14f94f9268ef3e8`;
fresh reload took 27 ms. Invalid B produced three successful compiles, one
expected failed `i1_mobile_game` compile and three probes, with receipt-set
SHA-256
`7d2ee3441357c5d6b8016fb6aa413877da1fb8f65365f84179c233bfc93e8319`.
A remained runnable. The runner reported `buildReady:false`.

This proves real Cargo consumption from a sealed cache with a cooperating
guard-held lease. It does not yet give the T2 owner a complete reviewed
tool/dependency/support/runtime selection. The full cache and executable
rehashes are still on the per-edit path; no warm fast-path performance claim
follows. Synthetic extra-directory and manifest-drift refusal remain in the
[candidate fixtures](T2-HELD-CARGO-CACHE-CANDIDATE-2026-11-03.md), not this
real compiler run. The selected compiler-family no-detach assumption and
the outer-timeout nested-job gap described in the
[tool-root A/B evidence](T2-HELD-NATIVE-AB-2026-11-03.md) still apply. No
ready `BuildRevision` is claimed.
