# SIM-2D M3 resource repair continuation

Pre-integration checkpoint: all nine builder results are now `pass`, with fresh clean Mac,
Chrome and simulator receipts at `f64eade89`; all 26 rows independently match the
unchanged retained folds/checkpoints. The simulator retry job has finished successfully
for M3.7; its only failure is the expected pre-merge M3.9 `NOT_ON_MAIN` query.
The repository gate in job `20261117-162103-57552` finished with its 3,600-second timeout
late in V1 GPU tests. Its logs and receipt are retained in M3-resource; all resource
refusal tests passed and no assertion failure was reported. The complete gate must
pass in canonical integration. Finish the evidence/log and merge canonically;
no checks are to be skipped. The work was not merged at this checkpoint.
The earlier account below is historical and its remaining-items list is superseded.

The repair code is committed at `f64eade89b3547715f4bb5d7176c89c709f27d06`.
`TickGame::profile_resources` exposes the complete staged aggregate through
`StateEncode`; runtime preflight refuses non-finite resources inside the existing
tentative-write abort boundary. Crowd supplies its staged resources, with a fieldwise
finite check. Non-profile games retain their default opt-out. The guide documents it.
New public-API tests cover nested NaN/both infinities, queued/direct edits, serial/forced
Rayon merges, unchanged endpoint/digest/resources/publication/prior delta and finite retry.
Focused native tests and Clippy passed. No retained trace or image was changed.

First collect these background jobs with the supplied build script's `wait` command:

- `20261117-162103-57552`: full `check --module SIM-2D --milestone M3 --timeout 3600 --json`.
  Guard, controls, Mac and Chrome passed; simulator failed its lock limit; repository gate
  was still progressing through Python tests at 17:14 UTC. This job will exit nonzero
  for the simulator failure and expected M3.9 `NOT_ON_MAIN`, even if the gate passes.
- `20261117-181457-76652`: `caffeinate -i` around the same check with `--items-only`.
  It reuses passing checks and retries the simulator unchanged, with normal per-invocation
  simulator locks. Idle sleep is only a hypothesis; it is not a diagnosed cause.

The first simulator run was killed on its single `10000 36000` Crowd invocation after
holding the simulator lock for 30 minutes. The host load was about 40 at retry launch.
Do not shorten traces, change digests, remove locks or raise limits without direction.
The stale `tmp/sim2d/run-ios-sim.json` is from the old M3 build and must not be counted.
The fresh retry should replace it only after a complete success.

[Repair evidence](M3-resource-repair.md) links the clean native/Chrome receipts, guard
outputs, control and guide recording. Guide recording matched all 603 retained Crowd data
rows exactly. Fifteen conformance Python tests passed; engine crate crowd-name search
had no matches. Resource-specific rejection was executed natively, not in Chrome/simulator.

Remaining: obtain a passing simulator receipt and repository gate, finish the evidence,
set all nine M3 `builder_result` fields to pass with target evidence, and leave status
`in_progress`. Only builder fields may change; preserve reviewer/planner fields.
M3.9 uses `main-branch check exact-images`; do not run a new image campaign.
Commit the evidence and use `agent_loop.py merge --worktree .` until it reports `MERGED`.
The lead starts review after its main-branch checks; do not start a reviewer here.
Work has not been merged. This handoff records ongoing work, not a blocker verdict.
