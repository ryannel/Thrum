# Delivery lead handoff, 12 Nov 2026

The outgoing lead (a Claude Opus 5.5 session overseeing Thrum through orchestrate 2.0.0) wrote this when the owner
stopped all Thrum work for the orchestrate 2.1.0 upgrade. Nothing was running when it was merged. The live milestone
list is [STATUS.md](../plans/STATUS.md); this file adds what that page can't show.

## Where each module stands

- **PLATFORM-RUNTIME-MAC M2**: Blocked (`needs_context`, the upgrade stop). [Handoff](PLATFORM-RUNTIME-MAC/M2-handoff.md).
  The last review failed because the motion and cadence recordings were made on an old binary. M2.1–M2.3 need an
  attended re-record of the current build on the built-in ProMotion display, with the owner at an unlocked Mac and no
  other GPU work running. The branch `build/mac-m2` has only the handoff commit beyond main.
- **APPLE-TILE M3**: Ready for Review at `4159a2b769d771fdd4d74db6a953262d1d6de7c3` (worktree `apple-tile-m0`). The
  last two reviews ended `could_not_judge` on D3.5 (the console was locked) or timed out on GPU contention. It needs one
  visible review (`ORCHESTRATE_VISIBLE=1 review --module APPLE-TILE --milestone M3 --commit 4159a2b7...`) with the owner
  at the Mac and builders paused. CREATOR M2 and PLATFORM-RUNTIME-MAC both wait on it.
- **APPLE-PRESENTATION M2**: Ready for Review on main at `90ed21a0`, with all seven items passing. The builder merged it,
  but the run was paused before the script recorded a claim, so the record file may not show one. Next, run the
  review on that commit (Astra, medium). M3 then unblocks ATMOSPHERE M6. No handoff was needed.
- **CULLING M1**: In Progress on main. The repair for the failed review item M1.4 is finished and set ready for review on
  `build/culling` at `1043e93b`, but it is **not merged**. The full milestone check was interrupted by the stop and
  has to run again before the merge. [Handoff](CULLING/M1-handoff.md) (the builder paused its own goal, so the lead
  copied this handoff to main by hand). The milestone is not marked Blocked, so `run --fresh` carries on without `unblock`.
- **EFFECT-GRAPH M2**: Blocked (`needs_context`, the upgrade stop). [Handoff](EFFECT-GRAPH/M2-handoff.md). The work
  is on `build/effect-graph`, 17 commits ahead of main and **not merged**. It is rebased onto APPLE-PRESENTATION M2,
  and its final source is `2e9ccbda`. The items are still marked open, and the proof page describes a commit from
  before the rebase. Requalify on the final source, then merge and claim.

Modules not listed are Done (PLATFORM-RUNTIME-IOS, T2-CAPTURE, COOKED-DRAW, BASELINE-GUARD) or not planned yet
(CONTENT-LOOP, SHOWCASE, SEA, CLOUDS and the swarm-game modules 16–21). CREATOR waits on APPLE-TILE M3, and ATMOSPHERE
M6 waits on APPLE-PRESENTATION M3.

PLATFORM-RUNTIME-MAC M2 and EFFECT-GRAPH M2 are recorded Blocked with reason `needs_context`. That block is not a real
blocker. To carry one on, run `unblock --module X --milestone Mn`, then `run --fresh` (see "Running Thrum" for why
fresh). The new builder's prompt names the handoff.

## Waiting on the owner

These are the open questions, each listed once:

1. **When the sea and cloud port starts.** ATMOSPHERE M5 and EFFECT-GRAPH are building readiness ahead of that call
   ([port decision](../design/decisions/SEA-AND-CLOUD-PORT.md)).
2. **The Broadsides licence**, which the port needs before any code moves.
3. **Codex `danger-full-access`.** With `workspace-write`, browser (Chrome WebGPU) items need an out-of-sandbox
   request that auto-review can refuse. Eighteen items need a browser. The setting is the owner's call, because it
   hands the agent the machine.
4. **Swarm-game decisions.** The brief, [NEXT-GOAL-SWARM-GAME.md](../design/NEXT-GOAL-SWARM-GAME.md), records
   defaults the owner hasn't confirmed: 2–4 player co-op, top-down 2D sprites, and V1's device targets.
5. **The plan assumptions** listed under "Waiting on you" in STATUS.md (Mac ProMotion witness, the CREATOR profiler
   shape, the APPLE-PRESENTATION Chrome rows and ΔE bound, ATMOSPHERE's deferrals, CULLING and EFFECT-GRAPH scope).
   Each holds until the owner overrules it.
6. **An attended sitting.** APPLE-TILE M3's re-review and the PLATFORM-RUNTIME-MAC M2 re-record both need the owner
   at an unlocked Mac, with no other GPU work running. See the modules above.

## Decisions the owner made in chat

None of these is recorded anywhere else in the repository:

- **Models.** Builders run on Codex `gpt-6.1-sol` at medium effort. Milestone reviews run on Codex `gpt-6-astra` at
  medium, design reviews on Astra at high, and the planner on Claude Fable `claude-fable-5-2` at high. Name host,
  model and effort on every launch, because defaults have picked the wrong model before.
- **Builder count.** Two builders by default. On 12 Nov, while present, the owner asked for two more ("let's get
  some progress going"), and four ran.
- **Pushing.** Verified Thrum checkpoints may be pushed to main (6 Nov). Publishing or deploying anything else needs
  the owner.
- **Disk.** On 12 Nov the owner allowed reclaiming disk with `gdu-go`: old worktrees, build caches and old scratch.
- **Next goal.** After V1 comes an original swarm-survival game with 2D support and online multiplayer (10 Nov).
  Sea and clouds still have to be finished and consolidated in Thrum.
- **Hands off:**
  - Don't modify Broadsides.
  - Don't touch the main checkout (`~/Workspace/thrum`, branch `codex/handback-review`). It holds the owner's
    uncommitted AGENTS.md and docs/ASSET-LIFECYCLE.md, and its `tmp/` holds the oversight goal.
- **Messaging.** Owner digests are asynchronous: never wait on a reply to a progress report.

## Running Thrum: things the next lead would otherwise rediscover

- **One GPU job at a time.** All GPU work goes through `lock gpu`, and builders queue behind each other. A builder
  whose turn ends while its GPU jobs are still queued can come back `STALLED`, as CULLING did on 12 Nov. When that
  happens, wait for the jobs to finish, then resume it.
- **Stale GPU holds.** A visible review that times out can leave processes holding `.git/orchestrate-gpu.lock` and
  block everything. Check the holders with `lsof .git/orchestrate-gpu.lock`.
- **Attended items fail when nobody is there.** Items that need the owner's display (ProMotion presentation,
  visible windows) come back `could_not_judge` when the console is locked. They also time out when builders hold the
  GPU. Schedule them in a sitting with the owner, with builders paused.
- **Recordings must come from the build under review.** Mac M2's review failed because the recordings came from a
  binary built at 10287802, and 96 of 406 source files had changed since.
- **Landing.** A `land` that fails in GPU tests is usually contention. Re-land it under the GPU lock.
- **Old names in a worktree.** A worktree still using the pre-2.0 names refuses `run`. If it has no commits of its
  own, `git merge --ff-only origin/main` fixes it. If it does, `migrate` is needed, and `migrate` refuses while
  any loop runs.
- **Paused goals.** A Codex thread whose goal is paused comes back `PAUSED` at once from a headless `run`; only the
  app can reactivate it. Use `run --fresh` instead. The new thread finds the handoff through the prompt.
- **Disk.**
  - Builders stall below about 16 GiB free.
  - The biggest consumers are the reviewer checkouts at `.git/orchestrate/verify/<MODULE>/{tmp,target}` and the
    `tmp/` folders of finished milestones. Both are safe to clear when idle; check `lsof` for a live cwd first.
  - Scan specific folders with `gdu-go`, since a full scan of `~` takes more than 10 minutes.
- **Keep-awake.** Long unattended runs need `caffeinate -d -i`.
- **Quota.** On 12 Nov, Codex was at 77% of its weekly quota, resetting Sat 14 Nov at 10:28. The script's `quota`
  forecast warned that the open milestones may run short.
- **Codex desktop threads.** `~/Workspace/codexctl/codexctl.py` drives them headlessly and works with the Mac
  locked. Don't use screen control of the ChatGPT app.
- **Remaining worktrees** under `.claude/worktrees/`:
  - `apple-presentation` (matches main)
  - `apple-tile-m0` (APPLE-TILE M3 claim)
  - `atmosphere` and `creator-m1` (idle, behind main)
  - `culling` (`build/culling`, unmerged)
  - `effect-graph` (`build/effect-graph`, unmerged)
  - `mac-m2` (`build/mac-m2`)
  - Remove each one when its module is done.
