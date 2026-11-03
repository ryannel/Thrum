# Native build supervisor candidate

3 November 2026. **Local macOS ARM64 test adapter; `buildReady:false`.** This
qualifies a bounded development-build control path in the [prepared native
environment](../../design/T2-PREPARED-NATIVE-ENVIRONMENT.md). It does not close
the compiler's selected reads or publish a target `BuildRevision`.

The earlier [Rust collector experiment](F4-BOUNDED-NATIVE-BUILD.md) was
withdrawn after review found unsafe interruption, unbounded cleanup joins and
inherited stdin. The candidate reuses the Python Cargo-metadata collector. It
closes child stdin, checks a fixed launching-owner PID at most every 100 ms,
bounds escaped-descendant pipe cleanup waits, and uses macOS
[`kqueue` process-exit events](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/kevent.2.html)
or Linux `waitid` to observe child exit without reaping. It sends SIGKILL to the owned
process group before releasing the leader PID. It refuses an inherited-pipe
descendant and stops a same-group descendant with closed pipes before success.
The local macOS kernel returns `EPERM` when only the unreaped exited leader
remains; this candidate tolerates it under an assumption that the selected
Cargo/Rustc/Clang family does not change privilege. That assumption and
same-group containment are not release-qualified.

For the sandboxed A/B fixture, Rust serializes its selected `sandbox-exec`
command, working directory, clean child environment, output root and owner
PID into an exclusive request in the fresh output tree. Python checks the
request and parent before launch and runs Cargo without a shell. It retains
at most 4 MiB stdout and 1 MiB stderr for a 120-second deadline, then writes
exclusive diagnostic files and a small hash/length/status report. Rust caps
and hashes those files independently, including the 4 KiB report, and accepts
only Cargo exit 0 or the first-profile compile failure 101 as completed. A
SIGTERM handler routes ordinary termination through collector cleanup.

The first fault-report extension catches that resulting `KeyboardInterrupt`
after collector cleanup and writes a bounded `interrupted` report instead of
leaving no supervisor record. A timeout or other collector error writes
`supervisorError`. Either report has `buildReady:false` and a diagnostic
`receiptInventory`: a sorted, domain-separated digest binds the observed
receipt names, lengths, byte hashes and parse states. The inventory streams at
most 64 entries and reads at most 1 MiB per receipt, one at a time. It counts
completed zero/nonzero, well-formed pending and unparsed files; an aliased or
otherwise unreadable set is reported as unavailable. It does not assert that
the directory contains every invocation, or publish the raw fault receipts.
Successful and invalid-code B verification keep their strict completed-receipt
rules.

Sixteen focused metadata-collector tests cover Ctrl-C cleanup, closed stdin,
parent disappearance, output limit, retained pipes, an escaped-session child
and a same-group child with closed pipes, plus a fast-exit observation race.
The escaped-session case returns in
a bounded interval but leaves that escaped process for fixture cleanup; it
does **not** prove containment. Seven routine supervisor protocol tests cover output
binding, wrong-parent/aliased-request refusal, unexpected Cargo status, caught
interruption and timeout inventories, aliased and 65th-entry refusal, and an
actual SIGTERM delivered to a supervisor process with a mocked collector. An
eighth opt-in macOS sandbox process test runs the actual Rust receipt wrapper
against a sleeping selected Python child. It exercises both SIGTERM and a
two-second test-only timeout through the real collector, observes same-group
child exit and verifies the wrapper's pending receipt in each case. The command-line
supervisor retains its 120-second deadline.

A second opt-in test launches real pinned Cargo with the actual Rust receipt
wrapper and a controlled compiler shim. The shim forwards Cargo's probes to
the pinned Rustc, then records its PID and sleeps when Cargo starts the crate
compile. After that marker, SIGTERM reaches the real supervisor. On this host,
the fault report recorded a pending compile receipt and `buildReady:false`,
and the same-group compiler child was gone. The run took 0.54 seconds. This
tests Cargo/wrapper/supervisor interruption as one path; the controlled shim
does not prove an interrupted production compiler's complete invocation or
read closure, or detached-child containment.

Run the opt-in process fixture from the repository root after building the
wrapper with the pinned toolchain:

```sh
RUSTC_WRAPPER= rustup run 1.99.1 cargo build --locked --offline -p xtask --bin rustc_receipt
THRUM_TEST_NATIVE_SUPERVISOR_PROCESS=1 \
  THRUM_TEST_RUSTC_RECEIPT_WRAPPER="$PWD/target/debug/rustc_receipt" \
  python3.11 -m unittest discover -s scripts -p 'test_run_bounded_native_build.py'
```

The actual wrapper test took 2.24 seconds locally, including its deliberately
two-second timeout. This is fault-path test latency, not compile overhead or
an edit-to-ready tail.
The real sandboxed admitted A/B build passed: five compile receipts and three
probes reconciled, invalid B failed and A remained executable. A's fresh
Cargo step took 3,671 and 3,470 ms in two uncontrolled runs. Neither is an
overhead or tail-performance comparison.

An escaped session can still outlive the group; a complete release adapter
needs a retained job identity or equivalent containment. Python and its
script are not yet bound to reviewed release inputs. Host SIGKILL, complete
interrupted rustc invocation/child audit, compiler/system reads and retained
target publication remain unqualified. Compiler-job RSS has no hard process
memory ceiling.
