# Orchestrate profile: Thrum

Thrum's work follows the orchestrate process (`/orchestrate`, `/plan-project`, `/plan-module`, `/build-milestone`,
`/review-milestone`, `/build-parallel`). This file is what's specific to Thrum; the skills stay generic.

## Purpose
- **Purpose and goal:** [the build plan](docs/BUILD-PLAN.md): purpose, the V1 exit and the gate order.
- **Quality bar:** would pass in a shipped high-end game of this kind, named per milestone, e.g. sea and wakes as in
  Sea of Thieves, skies as in Red Dead Redemption 2, smooth at the display's full refresh. Judged in words against an
  inspection list, never against target images.

## Where things live
- **Plans:** docs/design/modules  *(read by the loop)*
- **Contracts:** [docs/design/CONTRACTS.md](docs/design/CONTRACTS.md); `/plan-project` is their coordinating owner.
- **Evidence:** docs/evidence  *(read by the loop)*
- **Graded examples:** [docs/design/GRADED-EXAMPLES.md](docs/design/GRADED-EXAMPLES.md)
- **Scratch:** `tmp/<task>/` (gitignored); bulk outputs follow [the storage rules](docs/ASSET-LIFECYCLE.md).
- **Status ledger:** [docs/design/maturity.json](docs/design/maturity.json), with implementation, correctness, visuals
  and device recorded separately; a review pass records `agent_reviewed`.

## Checks and integration
- **Check:** `./dev check` (documentation-only work: `./dev check-docs`). `./dev help` lists commands.
- **Integrate:** main  *(read by the loop)*
- **CI:** manual-only (workflow_dispatch), so a fresh `./dev check` on the rebased tip is the gate.

## Targets

| Target | What it is | Run | Capture evidence |
|---|---|---|---|
| mac-metal | This M3 Max, 120 Hz ProMotion display, Metal | `scripts/run_v1_showcase.py` and `./dev` scenarios | Offscreen fixed-camera captures and frame sequences, `.ppm` converted to PNG; contact sheets |
| chrome-webgpu | Chrome, portable WebGPU recipe | the browser showcase | Screenshots and frame sequences from the exact build |
| ios-sim | iPhone 18 Pro simulator; function only, never speed | the iOS route | `xcrun simctl io booted screenshot`, recorded sequences |

Label Mac stand-ins for iPhone as `proxy:iphone`. No physical iPhone is available; don't wait for one.

## Exclusive resources
- **gpu:** device timing and capture runs share one GPU. Wrap them in `agent_loop.py lock gpu -- <command>`.

## Owner
- **Owner decides:** changing purpose or the V1 exit; spending; publishing, licensing or deploying; irreversible
  actions. Everything else, including gate order within the V1 exit, proceeds without asking.
- **Licences:** no GPL or share-alike code or data; restrictive-licence code may inform ideas only.
- **Don't touch:** Broadsides (historical context only, ported module by module under
  [the port decision](docs/design/decisions/SEA-AND-CLOUD-PORT.md)); the owner's uncommitted files.

## Models
- **Codex generator:** gpt-6.1-sol medium  *(read by the loop)*
- **Evaluator / planner / lead:** the orchestrate defaults (evaluator and lead Opus 5.5 medium, planner Opus 5.5 high).

## Project guidance
[thrum-engineering](.agents/skills/thrum-engineering/SKILL.md) holds the engine rules each level reads:
- **Planning:** engine architecture, research-first module design, performance as a system property, reconciliation.
- **Building:** rigour by state, Rust code shape, hot paths, verification on named targets, build outputs.
- **Reviewing:** the calibration failures this engine has already paid for.
