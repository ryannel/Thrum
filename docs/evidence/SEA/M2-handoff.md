# SEA M2 recovery note

Finish SEA M2 with every item passed from its target and the work merged through
`agent_loop.py merge --worktree .`. Leave M2 `in_progress`; the lead's script owns
the later status and review. This note preserves a long-running qualification,
not a request to restart the implementation.

The implementation and evidence checkpoint is `4372f847`, rebased by the integration
script. `47f8337d` adds the resumed qualification log. No implementation changed
in this session. M2.1–M2.7 and M2.9 have `builder_result: pass`; only M2.8 is open.
The [M2 record](M2.md), its four selected Metal records, [cost record](M2-cost.md)
and [CPU loop record](M2-loop.txt) hold the proofs.

All three Metal integration targets and three library tests passed at `4b1fbfa8`
under the GPU lock in job `20261114-051511-21883` (exit 0). The rebase changed
none of the sea crates, their renderer dependency or workspace dependencies.
The ambient integration target passed again at `4372f847`; its fresh record is
`tmp/sea-m2/ambient.txt` (written at 07:13 local time on 14 November).

The complete scripted check is still running as background job
`20261114-054946-45357`, process 45366. It runs `check --module SEA --milestone M2
--worktree . --timeout 3600 --json`. Its output appears only when it finishes.
Wait for that job rather than launch a duplicate:

```sh
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.2.2/skills/orchestration/build/scripts/agent_loop.py wait 20261114-054946-45357 --worktree .
```

Its log is in the common Git directory, at
`orchestrate/jobs/20261114-054946-45357.log`. The shared GPU queue is long and has
been advancing. At 07:39 local time WATERS' verifier held it, with process 65983
as the lock parent and 85789 running the verifier. No access request was refused.
These process identities are observations, not grounds to stop another builder.

The first baseline job, `20261114-053227-31529`, exited 1 at `4372f847`. The
[retained attempt](M2-baseline-attempt.json) records an incomplete native
route-motion interval: 153 readback-ring omissions, zero writer omissions, and
Metal backend. The verifier stopped before completing all comparisons. The cause
of the omissions is unconfirmed. No baseline or other module's test was edited.
All 66 diagnostic metadata files are additionally preserved in
`tmp/sea-m2/baseline-first-failure.json.gz`; disposable pixels were removed.

The integration attempt rebased successfully, then refused its repository gate
because an exact-baseline proof was absent. It did not print `MERGED` and did not
push SEA. Documentation checks pass. The active scripted check includes another
baseline attempt and the full repository gate. Finish any actual failures, retain
the successful baseline record, update M2.8 and the final log, and merge. Recheck
only proofs affected by any upstream files the merge brings in.

GPU commands use the shared `lock gpu -- <command>` wrapper and need automatic
sandbox escalation for Metal/Chrome. The user authorizes verified pushes through
the integration script and the lead starts the reviewer. Keep changes in this
worktree. The baseline verifier needs a clean tree and at least 16 GiB free.
This session removed only its private release/incremental build caches and its
failed capture pixels. Keep the remaining diagnostic scratch until the baseline
failure is resolved; clean owned scratch and build output at final closeout.
