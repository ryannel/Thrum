# APPLE-TILE M10 continuation

This is a resumable checkpoint while target proofs wait for the shared GPU. M10
remains in_progress and every item remains open. No integrated target proof,
repository gate or merge is claimed. Continue the existing worktree, not a new
implementation. The latest implementation checkpoint is `a9330a7e`; this file's
commit adds only this continuation record.

The small same-submit fixture passed on the Apple M3 Max at `cdede389`; its
[retained result](M10/fixture-cdede389.txt) permits the route integration now in
the worktree. The implementation adds a dedicated whole Metal encoder between
wgpu encoders in one coordinator submission, initialized output/ping-pong history,
generation/completion retention, RAII discard invalidation, a prepared WGSL twin,
and five omission/stale/reset controls. Ordinary scenes leave the witness Off.
Diagnostic phase restarts after reset, and T2 resets the diagnostic after its
variable-duration warmup. Absolute warmup/submission serials remain recorded;
paired fixed samples use reset-relative sample indices and numerical exposure.

The native lifecycle tests cover exact SDR/scene/output/history parity, cuts,
healthy discard, background cancellation, acquire refusal, held two-slot history,
failure/loss, replacement parity and old-generation resource retirement. These
tests compile, but their Metal execution is pending. The simulator harness's
`--native-witness` mode holds an encoded frame, backgrounds the app, waits for
refusal while still suspended, checks native/empty-submit/present quiescence,
then checks reset and world-preserving resume. Simulator execution is pending.

Verified before this checkpoint: scoped native Clippy, `./dev check-web`,
`./dev check-mobile-core`, `./dev check-docs`, two graph-native CPU tests, 13 iOS
lease/input/world-policy CPU tests, and 11 capture-validator tests pass. The
simulator-target Clippy check and release simulator app build pass after
`a9330a7e`. Mac release game build passes; later edits affect only tests, the
simulator observer and proof scripts. These are compile/unit results, not target
acceptance. Use the pinned toolchain bin directory on PATH for cross-target
Clippy, as the repository launchers do; a direct Cargo subcommand otherwise found
a driver with the wrong sysroot despite the installed simulator target.

Existing work to resume, using the configured `agent_loop.py`:

- A foreground GPU-lock command runs `cargo test -p thrum-render metal_native
  -- --nocapture`. At this checkpoint it is waiting for the lock, not executing.
  If the turn is interrupted, that foreground command will stop; rerun it under
  the shared GPU lock and retain its output.
- Background job `20261114-062934-64565` waits for the GPU, then runs the exact
  coordinator test, simulator native background race, and
  `scripts/apple_tile_m10.py`. Output goes to `tmp/g3t-m10/simulator` and
  `tmp/g3t-m10/diagnostic`; inspect `manifest.json`/`report.json` and the job log.
- Background job `20261114-063318-66523` waits for the GPU, then runs
  `scripts/exact_baselines.py verify` and the official M10 check, with the full
  repository gate. It requires a clean tree. Both jobs survive turn interruption.
- Release builds have finished. The latest simulator build job is
  `20261114-065550-71424`; the latest Mac game build is
  `20261114-064231-69171`. The shared input store is
  `tmp/g7-showcase/runs/moving-20261114-033128/store`, validated by the proof runner.

The lock holder at the checkpoint is the Mac presentation builder's bounded
`tmp/mac-m3/sitting_driver.py` job, PID 64485, under lock owner PID 37853. It is
making progress through its final repository check; do not stop another builder's
work. There are other queued builders ahead of M10. If a queued proof fails,
diagnose its saved result and preserve the failure; never widen exact equality or
replace target proof with unit tests. A native copying/ordering mechanism failure
returns to boundary design review.

Finish target runs, inspect the selected full-resolution images and realtime
sequences, retain compact provenance/results in `docs/evidence/APPLE-TILE/M10`,
and mark only actually proved items pass. Then integrate through
`agent_loop.py merge --worktree .`, inspect upstream changes and rerun affected
proofs. Leave M10 in_progress. D10.5 must stay open: this background session cannot
obtain the owner's unlocked active Mac session and Screen Recording permission
for the OS-window observer. Do not automate grants. The requested final outcome
after every other item is completed is `BLOCKED needs_access`, naming that setup.

Scratch ownership is recorded in `tmp/g3t-m10/OWNERSHIP.md`. Earlier M3 captures
and evidence were preserved. This session removed only reconstructible debug
build outputs with `cargo clean --profile dev` (4.9 GiB); target tests rebuild
them normally. Retain selected M10 source spools through review, and preserve
failed proof artifacts while they are needed for diagnosis. No Instruments run,
temporary Python environment, OS grant change or capability/performance promotion
was made.
