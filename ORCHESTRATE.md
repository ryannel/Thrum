# Orchestrate profile: Thrum

Thrum's work follows the generic orchestrate skills (`/plan-module`, `/build-milestone`, `/review-milestone`,
`/build-parallel`). This file holds what's specific to Thrum.

- **Purpose:** [the build plan](docs/BUILD-PLAN.md): purpose, the V1 exit and the gate order.
- **Quality bar:** would pass in a shipped high-end game of this kind, named per milestone, e.g. sea and wakes as in
  Sea of Thieves, skies as in Red Dead Redemption 2, smooth at the display's full refresh. Judged in words against
  inspection items, never against target images.
- **Plans:** docs/design/modules  *(loop)*
- **Evidence:** docs/evidence  *(loop; graded examples in [docs/design/GRADED-EXAMPLES.md](docs/design/GRADED-EXAMPLES.md))*
- **Check:** `./dev check` (documentation-only work: `./dev check-docs`). CI is manual-only, so this is the gate.
- **Branch:** main  *(loop)*
- **Remote:** origin  *(loop)*
- **Codex generator:** gpt-6.1-sol medium  *(loop)*

## Targets

| Target | What it is | Run | Capture | Edge conditions |
|---|---|---|---|---|
| mac-metal | This M3 Max, 120 Hz ProMotion, Metal | `scripts/run_v1_showcase.py`, `./dev` scenarios | Offscreen fixed-camera captures and frame sequences (`.ppm` to PNG), contact sheets | Low sun, storm, distance, fast camera, full refresh |
| chrome-webgpu | Chrome, portable WebGPU recipe | the browser showcase | Screenshots and frame sequences from the exact build | Same scenarios as mac-metal |
| ios-sim | iPhone 18 Pro simulator; function only, never speed | the iOS route | `xcrun simctl io booted screenshot`, recorded sequences | Rotation, background and foreground |

Label Mac stand-ins for iPhone `proxy:iphone`. No physical iPhone is available, so don't wait for one.

## Rules
- **Exclusive resources:** gpu (device timing and capture runs share one GPU).
- **Owner decides:** changing purpose or the V1 exit; spending; publishing, licensing or deploying; irreversible
  actions. Gate order within the V1 exit is the agents' call.
- **Licences and paths not to touch:** no GPL or share-alike code or data (restrictive-licence code may inform ideas
  only); never modify Broadsides (ported module by module under [the port decision](docs/design/decisions/SEA-AND-CLOUD-PORT.md));
  preserve the owner's uncommitted files.

## Project guidance
[thrum-engineering](.agents/skills/thrum-engineering/SKILL.md) holds the engine rules for planning, building and
reviewing. The architecture brief is [docs/design/ARCHITECTURE-BRIEF.md](docs/design/ARCHITECTURE-BRIEF.md), the
shared contracts are [docs/design/CONTRACTS.md](docs/design/CONTRACTS.md), and capability status is
[docs/design/maturity.json](docs/design/maturity.json) (implementation, correctness, visuals and device kept apart).
Scratch goes in `tmp/<task>/`, under [the storage rules](docs/ASSET-LIFECYCLE.md).
