---
name: build-parallel
description: Runs several Thrum module streams at once under one Claude lead session. The lead runs Codex generators through scripts/agent_loop.py, evaluators and planners as subagents, and keeps quota and escalation by script. Use when the owner asks to fan out, run modules in parallel, or build several modules' milestones at once. Not for one stream (build-milestone in a single chat).
disable-model-invocation: true
---

# Build in parallel

Run as the lead, Claude Opus 5.5 at medium effort, in a Claude desktop session the owner started. Invoked as
`/build-parallel <MODULE> <MODULE> …`.

**Outcome:** each named module's milestones close, one after another, until each stream is done, parked or stopped by a
rule below. Each day the owner gets a short report.

You run the steps and the bookkeeping. You don't build and you don't judge. Generators build, the evaluator judges,
and the planner re-plans, each from files. Pass them file paths rather than your own summaries, and add no hints or
opinions to their briefs. That keeps their work independent, the same way Anthropic's long-run orchestrator does.

## Start

1. **Check each module** has a plan and a done-when file with an open milestone (`python3 scripts/agent_loop.py status
   --module <M> --worktree <path>`). A module without them goes to `/plan-module` first, through the planner below.
2. **Confirm with the owner, once, before launching anything:**
   - the streams;
   - Codex quota (`python3 /Users/ryannel/Workspace/codexctl/codexctl.py quota`) against about 10% a day to the reset;
   - your proposed number of concurrent streams.

   This is the only question you ask. After it, nothing waits on the owner.
3. **Set up each stream:**
   - **Worktree:** a worktree from `origin/main`, under `.claude/worktrees/stream-<module>`, kept across the stream's
     milestones so its `target` stays warm.
   - **Keep awake:** request keep-awake for the run.

## Run each stream

- **Generator (Codex).** Run `python3 scripts/agent_loop.py run --module <M> --worktree <path>` as a background
  command. It pins `gpt-6.1-sol` at medium on every turn and prints one JSON result line when it stops.
- **Generator (Claude, for a bounded slice the owner asked for).** Start a nested session at Opus 5.5, medium, with a
  `/goal` naming the milestone's done-when, so it keeps going unattended.
- **Act on the result line:**

| Result | What you do |
|---|---|
| `CLAIMED` with `review: false`, or `DONE` with all items passing | The oracle closed it. Start the next milestone. |
| `CLAIMED` with `review: true` | Run the `thrum-evaluator` subagent: "Review milestone `<id>` of `<MODULE>` at `<commit>`, worktree `<path>`." Nothing else. |
| `BLOCKED`, `NEEDS_CONTEXT` | Read the report. Context you can supply from files goes back by resume (`--message`). A design or contract problem goes to the planner. Otherwise it goes in the daily report, and the stream moves to its next independent milestone if there is one. |
| `BUDGET` | Treat it as a failed close: run the evaluator on what exists, so the findings drive the next attempt. |
| `FLOOR`, `PACE` | Stop launching Codex work, and put it in the report with the evidence. Re-check pace every few hours. |
| `PIN` | Stop that stream and report: a turn ran on the wrong model or effort. |

- **After a failed review,** run `agent_loop.py next`. It prints the action:
  - `same-session`: resume with its `resume_message`;
  - `fresh-session`: re-run with `--fresh`;
  - `replan`: the `thrum-planner` subagent re-plans the milestone with `/plan-module`;
  - `park`: set `state` to `parked` and put it in the report.

  Follow the action; don't override it on your own judgement.
- **Contract proposals** in a plan's log go to the planner at engine level (`/plan-engine`), run on the next pause in
  that stream.

## Shared resources

- **GPU timing runs one at a time.** When a stream needs a device timing measurement, hold the other streams' captures
  and timing runs until it finishes. Correctness tests can overlap.
- **Main is shared.** Generators rebase and run `./dev check` before pushing. If main breaks, the first stream to see
  it fixes it before continuing, and you note it in the report.

## Daily report to the owner

Keep it to a screen:
- milestones closed, with links to their evidence;
- verdicts the evaluator gave;
- new inspection lists, so the owner can adjust a bar that's too high or too low;
- parked or blocked items;
- quota used against pace.

When the owner overturns a verdict, add it to `.agents/skills/review-milestone/references/graded-examples.md`.

## Stop

Stop the run when every stream is done, parked or stopped by a rule, or when the owner says so. Run `cargo clean` in
the stream worktrees, remove worktrees whose work has landed, and send the final report.
