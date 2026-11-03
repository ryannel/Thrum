# Held native tool hash fast path

3 November 2026. This is an **unready macOS ARM64 development fixture** at
exact local code commit `3365696f52c69efd91d73bddd7d39959e6f29f20`.
It changes no ready `BuildRevision` or whole-engine contract.

The native A/B fixture previously hashed its selected Cargo, Rustc, Clang,
LibreSSL config and Xcode support files before and after each Cargo job. With
the four sealed-root guard held and live, it now omits only the three exact
Cargo/Rustc/Clang file hashes. The guard's cold open already inventories the
selected roots, its warm reuse check validates all four live leases, and the
Rust side watches guard death during Cargo. The unheld path keeps every file
hash. LibreSSL config and Xcode support files remain outside the held roots
and are still checked before and after each build. A focused test changes a
held file, then an unheld support file: the guarded check skips only the
former; the unheld fallback rejects both. Independent review found no
remaining issue after a non-macOS mutable-binding correction, and the normal
`RUSTC_WRAPPER= ./dev check` passed.

The three selected executable sizes on this host were 32,182,376 B for Cargo,
413,480 B for Rustc and 124,520,368 B for Clang: **157,116,224 B per
pre/post check**, four checks across A/B, or 628,464,896 B of repeated file
hash input removed from this fixture. This is an accounting of logical input
bytes in the removed calls, not a measured physical I/O, CPU or wall-time gain.
Cold selection and the outer four-root post-build inventories still read the
selected roots, and complete warm edit-to-ready cost remains unmeasured.

Two normal exact-commit A/B runs passed. A produced five successful compiles
and three probes, retained and reloaded a runnable candidate; invalid B had
three successful compiles, one expected failed game compile and three probes;
materialized lockfile drift was refused and all four roots re-inventoried
unchanged before owned cleanup. The first successful run reported 4/1 ms held
source-key checks, 14.073 s guard cold open, 32.102 s A/B process and
13.051 s post-build inventory. The second reported 3/1 ms, 12.920 s,
30.194 s and 10.712 s respectively. An exact-commit guard-loss run after a
pending compile-kind wrapper receipt returned an interrupted unready report,
one pending receipt, no candidate and unchanged roots; its admitted process
took 21.967 s and its post-build inventory 10.763 s. These uncontrolled
observations do not establish a speedup or CPU/RSS bound.

An earlier run of the same fast-path code failed **after** both compiler and
receipt audits, when the retained A executable exited unsuccessfully on its
later rerun. Its assertion did not retain exit details, so the cause is
unknown; the owner preserved scratch and its failure log for inspection. The
fixture now captures exit status, stdout and stderr on that assertion. Two
subsequent complete A/B runs passed, as did 1,000 launches of an existing
non-pinned local `target/debug/headless` idle and 1,000 with eight CPU
workers. The direct launches are a weaker diagnostic; they did not reproduce or
explain the original failure. After reviewing the log and confirming the job
had stopped, the failed-run owner and four root locks were acquired
exclusively and only that owned scratch was removed. Retained-A rerun
stability remains an open release gate.

This optimization relies on the first development profile's trusted local
user, immutable sealed roots, held guard and selected no-detach compiler
family. It does not cover installed support/host-runtime bytes, metadata
preparation before the guard starts, detached compiler children, official
origin review or durable ready publication. `buildReady:false` remains.
