# EFFECT-GRAPH M2 handoff

The owner stopped work on 12 November 2026 for the orchestrate upgrade. The goal remains active.
Read the upgraded build-milestone skill and project instructions before resuming.

At shutdown, branch `build/effect-graph` was **16 commits ahead of main and unmerged**, with work tip
`2e9ccbda663ce6a10fe712c413c813c0d8f142b4`. Its base is
`90ed21a0b9256d175f3106134910a0462a2d4282`, APPLE-PRESENTATION M2. This handoff adds one documentation
commit to that branch. No merge or push followed the stop request. The working source was already committed.

M2 currently has state `in_progress` and all eight items `open`. The checked work boxes in PLAN and the promoted
M2 proof describe an earlier qualification. They must be reconciled with final evidence before claiming review.
In particular, `docs/evidence/EFFECT-GRAPH/M2-proof.md` and its linked machine records still describe the
pre-rebase source `41357d2f` / checked commit `e1d8fe4b`. They are historical evidence, not the final combined build.
The lead starts review. Do not start a reviewer or M3.

## Implemented work

The source at `2e9ccbda` contains the ordered public contribution list, declared effect resources and named
compute passes, cadence validation, source-clock frames, tentative/submitted state and wrapping anchors, and
independently committed history sides. The external V1 diagnostic is opt-in through `THRUM_EFFECT_GRAPH=1`.
It exercises two differently cadenced histories, a consumer, a time-sliced volume, a mipped array and a cube.
The public seam and exact scripted oracle run on both native recipes and in Chrome, including device destruction
and replacement from a fresh adapter on the same executor instance. Every row reads every diagnostic texel.
The scripted stall drops exactly 76 ticks beyond the catch-up bound. The realtime route stops its source at
270, changes cadence at 300 and pans the camera across the 32 m anchor boundary around 346.

The initial merge attempt stopped on conflicts after APPLE-PRESENTATION M2 landed. The rebase is complete.
The combined code preserves its temporal inputs and submission-order history, includes both temporal and effect
browser cases, and excludes both kinds of auxiliary pass from the fixed timing sequence while preserving raw names.
No other module's exact references were refreshed.

Two subsequent integration repairs are committed.

- `da925675a1c85bd146b24214e5995802321a02c3` adds the diagnostic's third opaque output required by the new
  public surface. It writes reactive coverage 1 and the invalid-motion sentinel 65504. Correct diagnostic motion
  belongs to M3. Chrome exposed the missing output on the first rebase run.
- `2e9ccbda663ce6a10fe712c413c813c0d8f142b4` moves effect-pass recording, with its body unchanged, into
  `synthetic/contributed.rs`. The combined `synthetic.rs` had exceeded the 2,500-line code cap by four lines.
  It now has 2,493 code lines. Repository policy and all-target Clippy passed after the move.

## Confirmed evidence and limits

A complete six-item automated check passed before the rebase. The separately GPU-locked `./dev check` also
passed then. Its evidence is committed in `docs/evidence/EFFECT-GRAPH/M2/`. The original realtime proof includes
whole frames, 12 consecutive native-size crops per event, movie, exact identities and executor timings.

On `da925675`, all six immutable item commands passed again. See
`tmp/effect-graph-m2/combined-check.json`. Its exact verifier passed 38 native matrix comparisons, four native
surface comparisons and 481 records in each of four Chrome sets. Its record is
`tmp/baseline-guard/verify-20261112T174335.188241Z/record.json`, with covered-content key
`613126b57c3906df293989bce8fba1aa090c45936c0f1e6fbb1e0a2ffebaa0eb`.
The subsequent repository gate stopped at the size backstop before its GPU tests. That is recorded in
`tmp/effect-graph-m2/dev-check-combined.log` and repaired by `2e9ccbda`.

On final source `2e9ccbda`, repository policy and native all-target Clippy passed. Logs are
`tmp/effect-graph-m2/repo-check-split.log` and `tmp/effect-graph-m2/clippy-split.log`.
Chrome's final state run passed in `tmp/v1-browser-1794506650440/run.json`, with hashes, hardware identity and
exact state/recovery results in that directory. Native state and cadence records were regenerated under
`tmp/effect-graph-m2/{state,cadence}-{portable,apple-tile}.txt` and their aggregate `*-record.txt` files.
The final combined cadence pipeline count is **17**, unchanged across cadence changes. Earlier promoted
records have 15 from the older engine. The final milestone checker was stopped during M2.8, so its aggregate
`tmp/effect-graph-m2/final-check.json` is empty. Do not claim a completed final checker from those partial records.

Two complete capture pairs remain in scratch after the rebase, at `trail-rebased` / `without-effect-rebased`
(source `a7bb0cd5`) and `trail-combined` / `without-effect-combined` (source `da925675`), beneath
`tmp/effect-graph-m2/`. Both corrected-source capture arms completed. They have not been promoted as final proof.
The `a7bb0cd5` images were inspected through whole frames and consecutive native-size stop/cadence/wrap crops.
The trail persisted, faded, stayed on the ground and showed no transition pop. The later `da925675` capture
has not received that full inspection. No final-source `2e9ccbda` capture was started.
Descriptor bytes remain 23,840. Regenerate costs from the final capture and retain its load records.
Capture verifies the lossless source against every original writer RGBA hash with an independent zstd decoder.

The expectation that the final recording-method relocation preserves rendering is supported by its unchanged
body, compilation and Clippy. A completed final exact verifier, final realtime inspection and final full gate
are still required to make the final-source claim. No final-source exact pass, ready-for-review state or merge
is claimed.

## Jobs stopped for the upgrade

All these owned jobs and descendants were terminated with SIGTERM and confirmed absent afterward.
The process inventory is also recorded locally in `tmp/effect-graph-m2/upgrade-stop.json`.

- PID 51941 was the final six-item checker. It reached exact verification, PID 52355, and native matrix
  PID 52430. The native build, cooker build, web build and cook exited 0. The first native matrix was still
  running, with no completed comparisons. Its interrupted record is
  `tmp/baseline-guard/verify-20261112T180420.336408Z/record.json`, where `passed` is false.
  This interruption is not an image-mismatch finding. No final-content proof was published.
- PID 52686 was waiting for the GPU lock to run `run-capture-2e9ccbda.sh`. Neither
  `trail-2e9ccbda` nor `without-effect-2e9ccbda` was created.
- PID 52307 was waiting for the final checker before starting `./dev check`. It never started the gate.
  `tmp/effect-graph-m2/dev-check-final-source.log` is empty.

Earlier obsolete runs also remain for diagnosis. The first rebased Chrome failure is in
`tmp/v1-browser-1794505095069/run.json` and `chrome.log`. It reports the missing third fragment output,
followed by an invalid command buffer and a zero seed readback. That output-contract defect is fixed in
`da925675`, whose Chrome run passes. An attempted `run-capture-final.sh` refused an already existing scratch
output directory before capture. The commit-specific capture script avoids that collision. Native release
builds can print a nonfatal rust-objcopy/libLLVM stripping warning. The resulting binaries ran successfully.

## Resume work and commands

First confirm this branch, the handoff tip, current main and that the stopped jobs remain absent. Read the
upgraded skill. Retain other modules' changes if a later rebase is needed. Run repository policy before long
GPU jobs so structural failures are caught early. Keep source fixed throughout each exact proof.
Use this worktree only. GPU lock contention is normal. Do not refresh another module's references.

The commands below use the script path configured for this session. Adapt it if the upgraded skill changes
that path. `tmp/effect-graph-m2/pinned.sh` selects the rustup 1.99.1 toolchain, prepends its bin directory and
clears `RUSTC_WRAPPER`. The Homebrew toolchain mixed incompatible cached metadata with rustup earlier, and
sccache failed in this environment. Use the pinned wrapper or the toolchain environment set by `./dev`.

```sh
L=/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.0.0/skills/orchestration/build/scripts/agent_loop.py
python3 scripts/repo_check.py
sh tmp/effect-graph-m2/pinned.sh python3 "$L" check --module EFFECT-GRAPH --milestone M2 --worktree . --items-only --timeout 3600 --json > tmp/effect-graph-m2/resumed-check.json 2> tmp/effect-graph-m2/resumed-check.stderr
python3 "$L" lock gpu --worktree . -- sh tmp/effect-graph-m2/run-capture-2e9ccbda.sh > tmp/effect-graph-m2/resumed-capture.log 2>&1
python3 "$L" lock gpu --worktree . -- ./dev check > tmp/effect-graph-m2/resumed-dev-check.log 2>&1
```

Run long jobs in the background and inspect their logs. The item checker takes each declared GPU lock itself.
Run the separately locked repository gate after the item checks publish a successful exact proof.
The capture script builds the native release game, then captures both arms in T2 `realtime` mode using the
valley and `examples/v1-game/routes/effect-state.json`. Its commit-specific directories are currently absent.
Choose fresh output names if they exist when resuming.

After both final captures complete, generate their selected whole frames, consecutive native-size crops,
1x movie and costs. `tmp/effect-graph-m2/review_capture.py` accepts capture directory and selected-output directory.
`tmp/effect-graph-m2/summarize_costs.py` currently points at the original capture pair. Make an owned scratch
copy with the final pair's paths and a fresh costs filename. Inspect the final whole sequence and each event
at native scale. Promote the final selected evidence and machine records into the existing M2 evidence folder.
Update the proof's source identity, counts, named costs and exact content key. Preserve the failed-run history
where it explains the integration repair. Do not mix earlier pipeline counts or capture identities into the final claim.

Once all eight items have final target evidence and the full gate passes, set their statuses to pass and M2 to
`ready_for_review`, reconcile the proof and log, and commit. Only builder-owned status/evidence/state fields
in DONE may change. M3 remains untouched. Integrate with the required script, under the GPU lock.

```sh
python3 "$L" lock gpu --worktree . -- python3 "$L" merge --worktree .
```

If its rebase changes relevant source or it refuses a stale exact proof, resolve the actual change and rerun the
affected proof and verifier. Do not bypass the gate. Verify the reported merged commit is an ancestor of
`origin/main` before declaring DONE. The goal may only complete after all items pass, M2 is ready for review
and the work is merged.

Scratch bulk is local and reconstructible. OWNER.txt records retention and expiry. Keep the selected final
lossless source archives through lead review. Older superseded captures and owned build products can be cleaned
under the storage rules after the required evidence is retained. No cleanup or further build ran after the
owner's stop request. The worktree remains for the next lead.
