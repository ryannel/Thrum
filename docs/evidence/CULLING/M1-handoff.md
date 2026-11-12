# CULLING M1 handoff for the orchestrate upgrade

The owner stopped all Thrum work on 12 November 2026 for an orchestrate upgrade and lead replacement.
Resume only after the next lead runs the upgraded script's `unblock` and then `run` for CULLING.
The active repair addresses review round 1 MF1, the mixed-mesh running-host lifecycle proof for M1.4.
[Review](M1-review.md), retained evidence (`M1.md`, section "Review round 1 repair — MF1 host lifecycle", on branch `build/culling`).

## Committed state

The worktree is `/Users/ryannel/Workspace/thrum/.claude/worktrees/culling`, branch `build/culling`.
The implementation and evidence are committed through `1043e93b00dcb799778d7dbc9b0be97e0957b4af`.
The initial upgrade handoff is committed at `9da1e40c`. This session changes only this handoff;
there was no uncommitted implementation work on entry.

- `f7cbef2a` added the actual mixed-field running-host fixture, the MF1 plan work and provisional evidence.
- `f7533d443b30a88196b2c41b0934d37515dafea4` changed the fixture to the game's daylight/exposure policy and added the nonblack geometry guard. This is the qualifying source revision.
- `a4857d3c` retained the full named-target pass, source identity, exact-baseline records, 18 losslessly verified PNGs and capture hashes. M1.4 became `pass` with reachable evidence.
- `1043e93b` set M1 to `ready_for_review` once all five items had status `pass` and evidence. It corrected an unnecessary dependency between the state update and my background finishing job.

M1 is locally `ready_for_review`, all five items are `pass`, and reviewer-owned verdicts, review links,
round count and findings remain intact. Only M1's state and item status/evidence changed in `DONE.json`.
The ownership mask against the reviewed file passed. The first two MF1 plan work items are checked.
The final qualification/integration work item remains unchecked.

The repair has not been integrated. At stop, cached `origin/main` was
`3b070bc2abf1ee095cec6dc4969453510be15872`, and `1043e93b` was not its ancestor. No `MERGED` result
exists for this repair. Earlier M1.2 production work passed the independent review, including all
1,930 ticks. The review reopened only M1.4.

## Confirmed proof and rejected approaches

A fresh stop-only session independently reread the local logs and result records on 12 November.
Both completed GPU jobs requested by the lead passed: `exact.log` ends with `PASS exact baselines`
and `focused-daylight.log` ends with the focused lifecycle test passing (one passed, zero failed,
12.25 seconds). The separate clean mutation-control run also passed (14.14 seconds). These are
existing results, not tests rerun during the stop. The focused command was:

```sh
python3 "$LOOP" lock gpu -- cargo test --locked --offline -p thrum-render --test visibility_buckets mixed_field_host_surface_resize_and_device_publication_keep_occlusion_hidden
```

The focused log explicitly records Direct equality on replacement frames 0–3, cold Occlusion
22,215 draws and renewed hiding at 1,145 draws. The full named-target and exact verifier records
below identify the committed qualifying source; no new correctness or visual inspection is claimed
by this stop-only session.

The named command `cargo test --locked --offline -p thrum-render --test visibility_buckets` passed
all 17 tests on Metal, with 12.44 seconds of test execution. The new witness calls
`FrameHost::resize_surface`, `device_prepare_inputs().prepare` and `publish_device` on existing
Frustum/Occlusion hosts with the game's full 100,513-instance mixed field and real surface presenters.
Four frames before resize, four after resize and four after replacement all match a separately
constructed Direct host byte for byte. Authority advances from tick 1 to 61 before publication.
Occlusion renews hiding from 21,930 Frustum draws to 3,186 before/after resize, and from 22,215 to
1,145 after replacement. The cold first replacement frame also matches Direct.
Named-target log (`docs/evidence/CULLING/M1-lifecycle/named-target.log` on branch `build/culling`), source identity (`docs/evidence/CULLING/M1-lifecycle/qualification-identity.json` on branch `build/culling`).

The full `python3 scripts/exact_baselines.py verify` command passed in 901.52 seconds of execution.
It verified 38 native comparisons, four surface comparisons and four Chrome sets of 481 records.
Its covered-content key is
`8e972d6a6af51a7c346cc367e5c8ad8e2b7074fe2c374131c0e787d80d5ef8f7`.
The later evidence/state commits change documentation only.
Record (`docs/evidence/CULLING/M1-lifecycle/exact-record.json` on branch `build/culling`), proof (`docs/evidence/CULLING/M1-lifecycle/exact-proof.json` on branch `build/culling`),
completed stage results (`docs/evidence/CULLING/M1-lifecycle/completed-stages.json` on branch `build/culling`).

Native-size inspection covered the original surface, resized surface, cold replacement frame and
seeded replacement frame. The images show lit props, walls and shadows. All 18 full-target captures
match the inspected focused-run captures and their PNG conversions decode back to identical RGB.
The captured output comes from the runtime presenter's GPU output textures at fixed frames.
Images and hashes (`docs/evidence/CULLING/M1-lifecycle/images.json` on branch `build/culling`), capture correspondence (`docs/evidence/CULLING/M1-lifecycle/inspected-capture-correspondence.json` on branch `build/culling`).

The first fixture used default diagnostic lighting and rendered dark geometry. Image inspection
rejected that evidence, and the committed qualifying fixture uses the game's actual daylight policy.
An initial setup using independently created wgpu Instances failed the replacement-device precondition.
The passing fixture requests distinct devices on one Instance, with generations 71 and 72.
The older target-reallocation/fresh-device loop remains insufficient for MF1 by itself.

An optional isolated mutation campaign completed a clean lifecycle control with all 12 comparisons
passing in 14.14 seconds. Deliberately inserting `scene.meshes = None;` immediately after
`let mut scene = self.scene.clone();` in `crates/thrum-render/src/runtime/device.rs` then failed at
pixel equality, exit 101, in 10.17 seconds. Its original file SHA-256 was
`578482613cbcd54e25fdd803bd10db59f6f3fb0000cfd39436d4ba2675708ad7`, and its mutated SHA-256 was
`b8962fd265018137b80c08e8972f2d3b490308098514a2b20a2e1310837402a9`.
The local record and logs are under `tmp/culling-m1-lifecycle/recovery-mutants.json` and
`recovery-mutant-loses-mesh-selection.log`. These optional results have not been promoted to Git.

The mode-reset mutation and final restored clean run have no result. They are optional additional
coverage checks. They must not become a new acceptance requirement. The original two uncaptured
flythrough mismatches remain individually unattributed. The scoped mesh-caster invariance repair
and subsequent passing routes establish the current behavior. They do not identify those old pixels'
individual causes. The history and limits are recorded in the main evidence page.

## Background work stopped

At 2026-11-12 16:44 UTC, I stopped all remaining jobs owned by this repair and confirmed every recorded
process had exited. The local stop record is `tmp/culling-m1-lifecycle/upgrade-stop.json`.

- Qualification supervisor 68583 and aggregate-check child 75001 were stopped. The aggregate command was `agent_loop.py check --module CULLING --milestone M1 --worktree . --timeout 3600 --json`. Its active M1.2 release test, cargo 78947 and executable 79120, was terminated at the owner's request. The aggregate log is empty, and no aggregate pass or failure report was produced.
- Optional mutation supervisor 68634 and GPU-lock child 78801 were stopped. The second mutation was queued and its log is empty. The isolated snapshot under `tmp/culling-m1-lifecycle/recovery-mutant-source` still contains the mode-reset mutation in `runtime/device.rs`, with SHA-256 `7935606da161b0689a789f1f310ee2b0726f96ce2f861fe4827b4b575a949d01`. The production checkout is clean.
- Automatic finisher 77656 was stopped before it finalized metadata or started integration. There is no `finish.json` or `merge.log`. Its predecessor 76727 had already been stopped when the owner required the immediate state correction.

The fresh stop-only session checked the recorded PIDs with signal 0: 68583, 75001, 78947,
79120, 68634, 78801, 77656 and 76727 were all absent. Process-list enumeration is unavailable
in this sandbox, so this confirms the named jobs only; the earlier stop record reports no remaining
owned processes. No new background job was started.

The completed exact verifier and clean lifecycle run were read before stopping the remaining jobs.
The incomplete aggregate and mode-reset stages are interrupted work. No result from them is claimed.
The ordinary documentation check passed after the M1.4 evidence promotion. No new build or test was
started during this owner-requested stop turn.

## First actions after the lead unblocks the module

Read this handoff and MF1, inspect the actual branch tip and `DONE.json`, and confirm the stopped jobs
have stayed stopped. Resolve the upgraded `build-milestone` skill and its script before issuing commands.
At stop the script was
`/Users/ryannel/.codex/plugins/cache/ryannel-skills/orchestrate/2.0.0/skills/orchestration/build/scripts/agent_loop.py`.
The checked-in profile uses `Test command: ./dev check` and `Merge: push`. The next lead should use the
upgraded workflow and its equivalent checked integration command.

Do not restart the local `qualify.py`, `recovery-mutants.py` or `finish.py` blindly. They contain
session-specific paths, source hashes and output assumptions. The mutation script expects its snapshot
to be absent, and the existing snapshot is mutated. The local scratch scripts are conveniences.
The committed plan, review, done-when file and proof records remain the authorities.

## Required qualification and integration still to complete

Use the upgraded script path as `LOOP` below. These are the commands supported by 2.0 at stop.
The aggregate command manages the locks on its named item checks. Keep long jobs in the background
with logs and revalidate their actual processes before retrying.

```sh
python3 "$LOOP" check --module CULLING --milestone M1 --worktree . --timeout 3600 --json
```

This must produce a complete pass for M1.1–M1.5 and `./dev check`. It includes the interrupted command
below and M1.5's exact literal build/cook/native-matrix command from `docs/plans/CULLING/DONE.json`.

```sh
python3 "$LOOP" lock gpu -- env THRUM_FIELD_FULL_PATH=1 cargo test --locked --offline --release -p v1-game --test field_occlusion_gpu
```

Retain the complete aggregate result and actual source identity. Finish the last MF1 plan work item,
update the log/evidence and maturity scope to reflect completed qualification, and commit those updates.
The local draft helpers have not been executed against the final state. Verify any reused prose against
actual results, particularly the optional mutation assertions. Preserve reviewer-owned DONE fields.

Then run checked integration through the upgraded script. The 2.0 command was

```sh
python3 "$LOOP" lock gpu -- python3 "$LOOP" merge --worktree .
```

If a rebase brings in affected renderer/game/script changes, rerun the affected named proofs on the
actual rebased source. A new covered-content key also needs a fresh baseline proof under the GPU lock.
The existing full baseline pass can be reused while that covered key remains identical.

```sh
python3 "$LOOP" lock gpu -- python3 scripts/exact_baselines.py verify
```

After integration, inspect the actual `MERGED` result, fetch and verify HEAD is an ancestor of
`origin/main`, and read remote M1 state/items/evidence. End with DONE only after that completion audit.
The lead owns the independent review. The active goal was paused for the owner's upgrade request.
