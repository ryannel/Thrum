# MacOS native compiler job containment

3 November 2026. Scoped T2 research for the first CPU-only ARM64 headless
development build. The governing design is
[T2 prepared native environment](../T2-PREPARED-NATIVE-ENVIRONMENT.md); this
note does not select a release-ready process backend.

## Observed boundary

The current opt-in supervisor starts sandboxed Cargo in a new POSIX session,
observes the unreaped leader, drains bounded diagnostics and signals its
original process group before releasing the leader PID. Same-group descendants
are stopped in the local fixture. An intentionally spawned descendant can call
`setsid`, leave that group and survive its signal; the fixture kills that
escaped child separately. An escaped child with a retained pipe makes the
collector refuse, but a child that closes the pipes is not reliably visible
through the existing leader/group observation. Therefore the current
supervisor is a bounded **candidate**, not a lifetime guarantee for all
descendants.

This is a process-model limit, not a timeout tuning problem. Apple's
[`setsid(2)` manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setsid.2.html)
defines a new session and process group; its
[`intro(2)` manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/intro.2.html)
describes child reparenting after the creator exits. Apple's
[App Sandbox guidance](https://developer.apple.com/documentation/security/discovering-and-diagnosing-app-sandbox-violations)
says a directly launched helper inherits the launcher sandbox capabilities.
That is useful for file/network restriction but does not establish a shared
killable lifetime after a child changes session. Apple's
[launchd job guidance](https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingLaunchdJobs.html)
discourages daemonizing managed processes; it does not by itself promise
transitive containment of arbitrary detached grandchildren.
[Apple's current background-process guidance](https://developer.apple.com/documentation/appkit/managing-ongoing-background-processes-in-your-mac)
recommends XPC services when a Mac app needs the system to track helper
relationships and terminate helpers with the app. That guidance does not
specify a termination boundary across `setsid`, reparenting and closed pipes
for an unprivileged command-line compiler adapter. Treating it as one would be
an inference requiring a detached-child qualification, not a documented
guarantee.

### Foundation `Process` candidate

Apple's [`Process.terminate()` documentation](https://developer.apple.com/documentation/foundation/process/terminate%28%29)
describes sending a termination signal to the task and its subtasks. This was
worth testing, but that sentence does not specify a stable job identity across
`setsid` and reparenting. A local Swift 6.4.2 probe on macOS 26.6.2 launched a
Python task through Foundation `Process`; that task launched a sleeping child
with `start_new_session=True`, closed all three standard streams and published
its PID. After `Process.terminate()` and `waitUntilExit()`, two fresh-marker
direct-child runs still showed the detached child alive in `ps` state `Ss`. A
second fixture let an intermediate child exit first, reparenting the detached
grandchild while the Foundation-managed guardian remained alive. The
grandchild also survived `terminate()`. A POSIX `kill(parent, SIGTERM)` control
likewise left the detached child alive. Each fixture explicitly killed its
sleeping child during cleanup. These are local behavior observations, not a
claim about every Foundation implementation; they rule out treating this API
as the required containment proof for the selected host.

## First-profile implications

The selected [release trust policy](../T2-NATIVE-RELEASE-TRUST.md) reviews exact
compiler and dependency inputs. The first native build also refuses build
scripts and proc macros, so admitted Rust rules are compiler input rather than
intentionally executed build-time code. This narrows the expected process
family but does not turn the current process-group cleanup into an OS job
container. A compiler/tool failure, a compromised selected tool or an
unexpected helper could still escape; output publication must refuse and
owned scratch cannot be assumed safe to delete merely because Cargo exited.
The [append-only held tool-root candidate](../../evidence/i2a/T2-HELD-TOOL-ROOT-CANDIDATE-2026-11-03.md)
keeps already sealed selected pathnames stable after an owner exits; it does
not make materialized source/output scratch safe to remove or prove all
compiler children have stopped.

The choices for a ready profile are:

1. First identify a supported macOS primitive or API that provides a
   transitive build-job identity and teardown operation across `setsid`,
   reparenting and closed pipes. The cited Apple guidance does **not**
   establish that such a primitive is available to this unprivileged adapter.
   If a candidate exists, qualify it on the selected macOS release against
   detached descendants, then measure cold/warm launch cost and diagnostic
   memory. A launchd or XPC name alone is insufficient; its tested lifecycle
   semantics must provide the guarantee.
2. Explicitly scope the first development profile to a reviewed compiler
   process family and assume that family never detaches a child from its build
   group. A detached child is then a trusted-tool fault rather than an
   adversarial input. Best-effort group cleanup and quarantined scratch after
   a *detected* fault do not cover a closed-pipe child on a nominal success;
   publication under this option relies on that compiler-family assumption,
   not a proof of process containment. This weaker operational contract
   requires a deliberate design decision before calling the adapter ready.
3. Use a disposable virtual machine or similarly isolated execution domain
   when untrusted build-time processes must be supported. That changes the
   cost and platform model and is outside the selected initial profile.

For now, keep option 1 as the **readiness target pending feasibility** and
option 2 as a reviewable fallback, not a silent promotion. The next proof
should run successful, failed, timed-out and interrupted compiler jobs with
retained receipt audits; then test same-group and new-session descendants with
both open and closed pipes against any selected job backend. Under option 1, a
success must leave no live owned process and no writable output claimant
before candidate publication or scratch removal. Under option 2, success
cannot establish that fact; the explicit trusted-tool assumption carries the
residual risk. In either case, a refused build must keep the previous
candidate verifiable.
No per-frame work belongs to this boundary; measured overhead is edit-to-ready
latency, launch/teardown tail and compiler-job RSS.
