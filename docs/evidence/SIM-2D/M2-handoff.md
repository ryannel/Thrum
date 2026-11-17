# M2 pre-merge recovery snapshot, 17 November 2026

All target evidence for the Chrome queue repair is complete. The remaining action
is the canonical merge. Check the build script's `jobs` and `origin/main` before
starting one: a merge may already be queued or completed after this snapshot.
Leave M2 `in_progress`, and end with `DONE` only after the script prints `MERGED`.
The lead runs the merged item checks and starts review.

The repair batches all nine Chrome conformance requests into one browser and one
resource queue turn, after compilation. It omits unused returned timelines and
rejects outstanding debugger requests on disconnect. No engine or retained trace
changed. The successful real Chrome receipt is
[chrome-webgpu.json](M2/chrome-webgpu.json), measured at `69395002b`.
All eight routes and the full vector table pass. Its native/Wasm artifacts remain
byte-identical after integrating upstream core world-lane changes at `119bd20ab`.

Both complete cost campaigns at `119bd20ab` are retained and pass their source and
completeness validators. Their shared source key is
`a8df10ea68a72e0b109e9f934951d7cd1dcf2809395fbba59c195c0b44d8b8c3`.
The background jobs `20261117-010823-22751` (Mac) and `20261117-010823-22761`
(Chrome) exited zero. The [performance notes](M2/performance.md) retain every
tripwire, both repeats and the historical investigation. Warmed calls allocate
zero. Earlier interrupted measurements were not accepted or relabeled.

Eleven focused Python tests, JavaScript syntax and the documentation gate pass.
The full repository gate passed before rebase; canonical merge runs it on the
integrated work. The lead reported the other M2 checks passing. Their accepted
proofs and the [repair notes](M2/chrome-queue-repair.md) are linked from
[the milestone evidence](M2.md). Every builder result is `pass`, and M2 remains
`in_progress`. The module log has an eight-line entry for this repair.

The build script is
`/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.3.6/skills/orchestration/build/scripts/agent_loop.py`.
Use its `merge --worktree .` without a resource lock. A background merge can be
collected with `wait <job> --timeout 55 --lines 10`. The abandoned Chrome profile
from the first batch attempt has been removed; preserve other scratch and other
builders' worktrees. The repair notes distinguish the observed first-attempt
disconnect from the unproven payload-size explanation.
