# PLATFORM-RUNTIME-WEB: log entries in full

Entries of [the module log](../../plans/PLATFORM-RUNTIME-WEB/LOG.md) that ran past a few lines, moved here unchanged on 11 November 2026 when the plans took the orchestrate 1.7.0 shape. The log keeps each one's opening and a link.

<a id="entry-1"></a>

## Entry 1: 2026-11-08

- 2026-11-08 Revised for the independent design review (REVISE).
  - H1: pause and resume are game-owned. The shell now drives them through a shell-side `WebWorld` trait, and the
    drain state stays in the game's `WorldDriver`. This changes no `thrum-render` API and touches neither native world.
  - H2: material reload is now refused while lost.
  - M1: attach is deferred to publication.
  - M2: `host_mut` is replaced by CPU-only mutators and gated GPU methods.
  - M3: the `rg` command is named with its file scope.
  - M4 and M5: the witness separates the camera control from the time control, reads both back from the game, compares
    presented screenshots as well as captures, and adds repeat controls.
  - M6: the report fields are listed.
  - M7: the seam now covers the host wrapper, xtask, the dependency text and a native test route for the clock policy.
  - L1–L7: corrected.
  - The material and attach rows change today's behaviour, so they land first with witnesses on today's facade.
