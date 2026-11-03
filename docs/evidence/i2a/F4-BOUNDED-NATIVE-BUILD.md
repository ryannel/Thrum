# Rejected native build collector experiment

3 November 2026. **The collector was withdrawn; no process-control gate was
checked off.** This records why the next [prepared native environment](../../design/T2-PREPARED-NATIVE-ENVIRONMENT.md)
slice must own interruption as well as output and elapsed-time bounds.

A temporary Rust test-adapter collector placed Cargo in a new Unix process
group, read both pipes concurrently with 4 MiB stdout and 1 MiB stderr caps,
and polled a 120-second deadline. A local sandboxed A/B compile passed. Small
fixtures also refused overproducing stdout/stderr, a sleeping child and a
background child retaining pipes.

Read-only review found three defects beyond those fixtures. Terminal Ctrl-C
would stop the test owner but not Cargo's separate process group, leaving the
compiler running. After a best-effort group kill, blocking pipe-reader joins
could still hang if a descendant escaped into another session while retaining
the pipe. The replacement also inherited terminal stdin, whereas the earlier
`Command::output` used closed stdin. These are lifecycle and input-boundary
errors, so the candidate was reverted before a commit. Its successful A/B
run establishes no accepted build-control qualification.

The next supervisor candidate must close stdin, forward or handle normal
interruption, bound retained output and cleanup waits, and explicitly test an
escaped descendant holding a pipe. The existing Python
`prepare_headless_cargo_home.py` collector already has bounded output and
process-group cleanup for Cargo metadata, but its present 120-second wait and
signal behavior must be reviewed before reuse for compilation. A complete
release adapter must additionally select the supervisor and all of its
runtime inputs. Compiler RSS has no hard process ceiling.
