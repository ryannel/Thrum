# Four-root held native build guard candidate

3 November 2026. **Unready macOS ARM64 development qualification.** The
governing [T2 environment design](../../design/T2-PREPARED-NATIVE-ENVIRONMENT.md)
requires one exact selected native input owner. This slice holds four sealed
roots together for the compiler part of one admitted A/B fixture; it does not
close release provenance or publish a ready build.

At exact code commit `b4a175610704e5e8290de6fb1c7874b9503f7ba9`, the
Python guard cold-opened and held the sealed private Cargo cache, Rust,
Xcode linker and SDK roots in one process. Rust compared the three tool-root
inventory digests to the actual `NativeSandboxTools` paths before launching
that guard. Each warm source-key request checked all four live leases. A
synthetic fixture also injected lost linker and SDK leases and required
refusal. The ordinary repository gate passed, and independent review found
no issue.

The exact-commit admitted A/B run passed. A produced five successful Rustc
compiles and three probes, retained and freshly reloaded an unready candidate,
and remained runnable after invalid B. B produced three successful compiles,
one expected failed game compile and three probes. A changed byte in B's
owned materialized lockfile was refused on a later held reuse request.
The cache's 2,923 files and both digest domains matched prior runs; all four
roots re-inventoried unchanged before owned scratch cleanup. The guard's
cold verified open took **11.757 s**; its two warm source-key checks took
**1 ms and 1 ms**. The admitted A/B test process took 31.450 s, and the outer
post-build four-root inventory took 9.313 s. The older outer qualifier also
cold-opened Rust/linker/SDK roots separately, adding 11.423 s of overlapping
verification work in this run.

At exact code commit `cd253a93aabdc5f9fc01f74e69ab8627752d647d`, the
outer qualifier stopped repeating those three cold opens. The guard remained
the one process holding all four leases through A/B, receipt reconciliation,
retained A verification and B drift refusal; the qualifier required an
explicit `held_native_roots=4` marker, then independently inventoried all four
roots after the guard closed. The second exact-commit run passed the same
outcomes and cleaned owned scratch. Guard cold open took **13.942 s**; warm
source-key checks took **1 ms and 2 ms**. The admitted A/B process took
35.776 s and post-build inventory 11.088 s. These are uncontrolled local
observations, not a measured end-to-end speedup. Removing the outer cold
opens removes duplicate work by construction; variable compiler, filesystem
and scheduling costs still dominate the observed totals.

This guard begins **after** private-cache metadata preparation and sealing.
The outer private append-only owner and selected immutable flags protect the
prepared tool roots during that earlier phase under the trusted-local-user
development policy, but this is not one held selection through metadata.
Installed Xcode support and host-runtime bytes are outside the four-root
selection. The exact official-origin release manifest has not been reviewed,
compiler helper/no-detach and complete read closure remain open. A later
section qualifies one observed guard-death path during a compiler job. The
runner reports `buildReady:false`; no `BuildRevision`, device or frame-time
claim follows.

## Bounded guard responses

At exact local source commit `3ad1005c7e8705dbe3a889c6134adaf29b1e309d`,
the Rust side of the guard protocol stopped using an unbounded line reader
and unbounded response channel. It retains at most 4,097 bytes to decide
whether a response exceeds the **4 KiB line limit**, and queues at most two
responses. A full queue makes the reader stop without blocking; shutdown
also refuses queued unsolicited output. A focused regression feeds four
responses without a consumer and requires the reader to finish within one
second. The normal `RUSTC_WRAPPER= ./dev check` passed, and independent
review found and then cleared a full-queue cleanup deadlock in the first
bounded-channel version.

The exact-commit macOS A/B qualification passed with the bounded protocol:
four selected roots remained held, A compiled and reloaded, invalid B failed
as expected, the changed materialized lockfile was refused, all four roots
re-inventoried unchanged, and owned scratch was cleaned. The two held
source-key checks reported **3 ms and 1 ms**. Guard cold open took 13.054 s,
the A/B process 34.592 s and outer post-build inventory 10.356 s. These are
uncontrolled local observations, not a protocol-overhead or end-to-end
speedup measurement. `buildReady:false` remains unchanged.

## Guard loss during an admitted build

At exact local source commit `f300d23e6a539ec28f546d3c8d6f72968f87a3ce`,
the Rust test fixture polls its owned guard and native Cargo supervisor while
the supervisor runs. A guard exit or unexpected response is a sticky build
fault. If the supervisor is still running, the fixture sends it SIGTERM and
waits for its bounded collector to stop and reap Cargo's process group. It
requires a completed unready supervisor report before normal fixture cleanup;
unknown child or cleanup state exits the fixture process and preserves its
compiler input/output and outer held-root scratch for investigation. A live
guard is checked again before retaining A. The poll sleeps 20 ms between
checks, so this is detection with bounded polling, not instantaneous revocation.

The opt-in `--fault-guard` qualification killed the real guard after a
**pending compile-kind Rustc wrapper receipt** appeared during the admitted A
build. The supervisor reported `interrupted`, retained one pending receipt,
and no A candidate was published. The A/B test failed as required. The outer
owner re-inventoried the private cache, Rust, linker and SDK roots unchanged,
then cleaned only its owned scratch. The qualifier requires a positive pending
count and the intended terminal panic, so an unrelated failure cannot pass.
The admitted test process took 24.319 s and the post-build four-root inventory
9.886 s on this uncontrolled macOS ARM64 run.

The normal exact-commit A/B run also passed: five A compiles and three probes,
retained runnable A, expected invalid B with three successful compiles and
three probes, materialized lockfile drift refusal, and unchanged four-root
inventories. Its two held source-key checks were 3 ms and 2 ms; guard cold
open was 12.129 s, A/B process 30.878 s and post-build inventory 9.614 s.
These runs do not establish a watcher speedup or CPU/RSS bound. A pending
wrapper receipt does not prove Rustc had started. Guard loss during candidate
staging, external SIGKILL of the supervisor, detached compiler descendants,
the reviewed helper family and durable ready publication remain open.
`buildReady:false` remains the only publication status.
