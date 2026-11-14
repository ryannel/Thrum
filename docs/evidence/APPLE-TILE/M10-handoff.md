# APPLE-TILE M10 continuation

This is a resumable checkpoint while target proofs wait for the shared GPU. M10
remains in_progress and every item remains open. Three integrated Metal lifecycle
tests pass, but no complete item, repository gate or merge is claimed. Continue the
existing worktree, not a new implementation. The latest code checkpoint is
`fe809c7f`; later commits retain continuation and proof records. `57e70e7c`
restricts the Metal-only device-test module to macOS. `fe809c7f` ends the native
qualification hold once both backgrounding and lease revocation are observed,
so the harness can observe cancellation while still backgrounded. Ordinary
eight-second stall witnesses retain their existing behavior. Simulator-target
Clippy and the rebuilt app pass after that change; actual simulator function is
still pending.

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
Three Metal lifecycle tests pass at `d29d2667`: held two-slot history, exact
replacement output/failure/loss reset, and old-generation retention until completion
delivery. [Selected actual output](M10/lifecycle-d29d2667.txt) records the command
and executable hash. The separate coordinator test stopped on capture accounting,
expecting zero omissions despite its three deliberate discarded candidates. It had
not reached native/twin pixel comparison. `342151fc` requires exactly those three
omissions, with no pixel-metric change; its rerun is queued. The simulator harness's
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

- The foreground `metal_native` command completed successfully: three actual
  Metal tests passed. Its retained result is linked above.
- Background job `20261114-062934-64565` exited 101 on the capture-accounting
  assertion described above, before simulator or T2 capture. Do not treat it as a
  pixel failure or as a passing proof.
- Background job `20261114-072259-82489` waits for the GPU, then reruns the corrected
  coordinator test, simulator native background race, and
  `scripts/apple_tile_m10.py`. Output goes to `tmp/g3t-m10/simulator` and
  `tmp/g3t-m10/diagnostic`; inspect `manifest.json`/`report.json` and the job log.
- Background job `20261114-063318-66523` waits for the GPU, then runs
  `scripts/exact_baselines.py verify` and the official M10 check, with the full
  repository gate. It requires a clean tree. Both jobs survive turn interruption.
- Release builds have finished. The latest simulator build job is
  `20261114-080040-90655`; the latest Mac game build is
  `20261114-064231-69171`. The shared input store is
  `tmp/g7-showcase/runs/moving-20261114-033128/store`, validated by the proof runner.

The original long Mac presentation holder exited. At the latest observation the
GPU holder was the Apple-presentation motion proof, child PID 88998 under lock
owner PID 66291; it was actively running, not diagnosed stuck. There
are other queued builders ahead of the corrected M10 diagnostic job. If a queued proof fails,
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
and selected evidence were preserved. This session removed reconstructible debug
build outputs with `cargo clean --profile dev` (4.9 GiB); target tests rebuild
them normally. It also released 7,156,258,376 bytes of superseded M3 resume bulk
sources after confirming M3 is done, the documented through-review expiry, 256
retained artifact hashes, and the newer integrated source set still retained in
`tmp/apple-tile-m3-integrated/captures`. Only README differs from the old hash
inventory and is the current committed integration/history explanation. The
[release receipt](M10/m3-expired-bulk-release.json.gz) inventories deleted bytes;
expired raw source bytes are not claimed recoverable from hashes. Selected M3
frames/previews/records and the registered baselines remain intact. Retain selected
M10 source spools through review, and preserve
failed proof artifacts while they are needed for diagnosis. No Instruments run,
temporary Python environment, OS grant change or capability/performance promotion
was made.
