# Orchestrate profile: Thrum

Thrum's work follows the generic orchestrate skills (`/plan-module`, `/build-milestone`, `/review-milestone`,
`/build`). This file holds what's specific to Thrum.

- **Purpose:** [the build plan](docs/BUILD-PLAN.md): purpose, the V1 exit and the gate order.
- **Quality bar:** would pass in a shipped high-end game of this kind, named per milestone, e.g. sea and wakes as in
  Sea of Thieves, skies as in Red Dead Redemption 2, smooth at the display's full refresh. Judged in words against
  inspection items, never against target images.
- **Plans:** docs/plans
- **Evidence:** docs/evidence  *(loop)*
- **Graded examples:** docs/design/GRADED-EXAMPLES.md  *(loop: past verdicts the owner agreed with or overturned)*
- **Check:** ./dev check  *(loop: CI is manual-only, so this is the gate; documentation-only work may run ./dev check-docs while building)*
- **Integrate:** push  *(loop: owner allows verified checkpoints straight to main)*
- **Branch:** main  *(loop)*
- **Remote:** origin  *(loop)*
- **Builders:** two Codex builders  *(owner, 11 Nov 2026: "Yes, run two builders now")*
- **Codex model:** gpt-6.1-sol  *(loop)*
- **Claude model:** claude-opus-5-5  *(loop)*
- **Default host:** codex  *(loop: builders)*
- **Review host:** other  *(loop: milestone reviews on Claude, design reviews on the host that didn't plan)*
- **Planner host:** claude  *(loop)*
- **Claude planner:** claude-fable-5-2 high  *(loop: owner, 10 Nov 2026)*
- **Codex design reviewer:** gpt-6-astra high  *(loop: owner, 10 Nov 2026)*
- **Claude reviewer:** high  *(loop: owner, 10 Nov 2026; Opus 5.5)*
- **Codex sandbox:** workspace-write  *(loop: GPU and Instruments runs go through Codex's auto-reviewed approvals)*
- **Writable roots:** ~/.cargo  *(loop)*

## Starting from a clean clone
`./dev help` lists the commands; `./dev check` builds and tests the workspace. Builders work in their own worktree on
a `build/<module>` branch and push `HEAD:main`. Large captures and recordings go in `tmp/<task>/` under
[the storage rules](docs/ASSET-LIFECYCLE.md); only selected evidence is committed.

## Targets

| Target | What it is | Run | Capture | Edge conditions |
|---|---|---|---|---|
| mac-metal | This M3 Max, 120 Hz ProMotion, Metal | `scripts/run_v1_showcase.py`, `./dev` scenarios | [T2 engine capture](docs/design/modules/T2-CAPTURE.md): fixed-time images and elapsed-time motion, lossless sources/contact sheets; active-panel rows separate | Low sun, storm, distance, fast camera, full refresh |
| chrome-webgpu | Chrome, portable WebGPU recipe | the browser showcase | T2 M1 qualified submitted fixed checkpoints/realtime output/time records; DOM/browser chrome separate | Same scenarios as mac-metal |
| ios-sim | iPhone 18 Pro simulator; function only, never speed | the iOS route | Until T2 M2, `simctl` screenshots/video prove simulator function/UI/lifecycle within recorded timing limits; M2 adds engine output/time records | Rotation, background and foreground |

T2 M0 (Mac) and M1 (Chrome) are done; simulator M2 is implemented and claimed for lead review after accepted COOKED-DRAW M1. Later visual
milestones requiring submitted-output or elapsed-time engine motion proof depend on the relevant accepted target
milestone and explicitly select its implemented capture mode (Mac still/fixed/realtime; Chrome fixed/realtime).
Simulator interim simctl evidence remains function only. Engine capture requires no OS Screen Recording grant by design;
open checks claiming desktop-capture independence require denied, attributed read-only Screen Recording preflights. Console state is recorded as a witness, never an admission/pass condition or a manual-lock wait.
Accepted locked-run evidence remains valid history. Built-in-panel cadence remains **attended setup** (unlocked active panel
and any Instruments authorization), with automated actions/review. Native/compositor UI and physical output checks
name their own attended reason where required. Never treat engine pixels/movie PTS as display swaps or photons.
Mac M2.3 owns the real-window tap integration/proof and M2.4 the bounded trace-finalization investigation;
M2.2 qualifies panel cadence with capture OFF, keeping capture-ON content/observer arms separate.
Mac M2 needs only T2 M0 and qualifies the current path. APPLE-TILE remains parked after two failed design reviews;
the lead has commissioned its scoped re-plan and fresh independent design approval/unpark through the loop is owed.
New Mac M3 needs Mac M2 and APPLE-TILE M3: delivered T2 M0/M1 then G3t M0–M3, including attended actual-window
checks and diagnostic simulator function, not simulator cooked/capture. T2 completion cannot restart APPLE-TILE.
G3t OS-window capture separately requires attended unlocked setup and Screen Recording permission for its observer;
this grant is not required by the engine tap or inferred from Mac panel setup.
Interim Chrome endpoint/`simctl` captures cannot prove submitted temporal output or elapsed-time engine motion;
those stronger rows retain their T2 target dependency or an unavailable result.

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
