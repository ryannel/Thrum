# SESSION M2 handoff

The implementation and evidence are committed at `357e4efc1ba3ca51a61e09bf956e62d8bc454ccf`. All twelve builder results are `pass` and M2 is `in_progress` in that commit. The local `./dev check` and `./dev check-docs` passed. Both 36,000-tick Mac correctness runs compared 1,200 digest checkpoints on each client. The lead controls, independent release cost runs and focused tests passed. The clean release rebuild matched the executable used for the retained runs. [Builder evidence](M2.md)

The remaining action is to confirm the merge. Orchestrate's durable job is `20261119-005058-84488`. The combined candidate at `3faf81079` contained SESSION and all its builder results. Its workspace check stopped when V1's `realtime_records_consumed_targets_including_expired_opportunity` submitted 31 frames where it expected 32. That test passed in the local workspace check. Orchestrate is retesting smaller groups, and the cause has yet to be isolated. The durable job can continue after this builder session's two-hour limit. [V1 capture test](../../../examples/v1-game/tests/capture_display_phase_gpu.rs)

Wait for the existing job from this worktree.

```sh
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.5.0/skills/orchestration/build/scripts/agent_loop.py wait 20261119-005058-84488 --worktree . --timeout 50
```

On `MERGED`, verify the twelve results and `in_progress` at the landed revision. The lead's post-merge checks own the status transition and the shared exact-image verification. If the job returns a failure, handle the specific failure it names before submitting another merge.
