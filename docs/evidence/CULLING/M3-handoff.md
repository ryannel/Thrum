# CULLING M3 continuation

Recorded on 14 November while the checked merge waits for the shared GPU.
The purpose is to finish M3 with all ten builder items pass, merge the work,
and leave M3 `in_progress` for the lead's checks and independent review.

All ten items now have builder_result pass and link to [M3 evidence](M3.md).
The runtime source at `396132a1` passes all six literal target checks. The
current code checkpoint `e90704cb` passes the six checks again, every exact
baseline, renderer all-target Clippy and a complete workspace test run.
`76a7f877` adds the qualification and retention log. This handoff adds context.

The remaining action is the checked merge. Background job
`20261114-005317-95180` runs `python3 tmp/culling-m3/merge_final.py` and writes
`tmp/culling-m3/final-merge.log`. Its wrapper holds the shared GPU lock before
calling the authorised merge command. Reuse this active job and inspect its
result before starting another integration attempt.

```sh
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.2.0/skills/orchestration/build/scripts/agent_loop.py wait 20261114-005317-95180 --worktree .
```

A `MERGED` result supplies the checked integration and its upstream file list.
Requalify proofs affected by upstream changes as the build skill requires.
Then verify all ten builder fields and M3's `in_progress` status on main and
finish with DONE and evidence. The lead starts the reviewer. At the latest
inspection, the SEA gate held the GPU and was progressing through workspace
field tests. APPLE-TILE runs were also queued. This was ordinary resource
contention. CULLING's duplicate queued gate `20261114-004741-94278` was cancelled
by terminating only its child PID 94282, leaving the checked merge queued.

The item report on current code is `tmp/culling-m3/clean-items.json`, with six
passes. Its original logs are under the Git common directory's orchestrate
checks folder. The current exact proof is
`tmp/baseline-guard/verify-20261113T231400.193830Z/record.json`, passed with
content key `4d66f2e12eea360d6b95661088b20c6bbeddef88bc0a78448357cd9339fe9bd6`.
The committed target reports, cost rows, captures and earlier exact records
are linked from M3 evidence.

The completed T2 job `20261113-231534-65408` passes the full 1,930-tick mixed
field, with Occlusion halving Frustum on 1,918 ticks. The builder reviewed all
five fixed checkpoints per target, consecutive whole realtime frames and
six-frame native-size edge, ridge and cover crops on Mac and Chrome, plus
field entrance/exit readbacks. No culling discontinuity was observed. Shadow
stair steps and the authored cut remain visible. The ledger and M3.9 clause
audit distinguish implementation, correctness, inspected content and costs.

The repository gate exposed the cloud observer's obsolete shade signature.
Its test-only adapter is repaired and both focused Metal cloud tests pass.
Clippy's implicit-clone finding is corrected by an explicit String clone.
A later full gate on e90704cb failed the existing cooker watcher test at
`generation 2 not published`. All twenty isolated repetitions then passed.
The native notification stream starts before the initial cook, so inspection
of that startup order does not explain the intermittent failure. Its full
failure is recorded by `tmp/culling-m3/clean-all.json` and the linked Git-common
stdout/stderr. The merge still has to pass the full repository gate.

The original sparse native pixel failures remain individually unattributed
in [the investigation](M3-investigation.md). The selected implicit material
sampling order passes the strict current oracles and retained references.
The rejected explicit-gradient candidate remains rejected. Retained references
and pixel comparisons retain their original criteria.

The log records deliberate retention of `tmp/culling-m3/` (22 GiB), including
`capture-20261113-231534/` (7.2 GiB), through independent motion review and
failure-input retention checks. The worktree's `target/` is an owned 11 GiB
build tree. Finish ownership-scoped cleanup after the active integration and
proof consumers are done. The worktree remains the loop's continuation surface.
