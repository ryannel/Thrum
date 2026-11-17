# Orchestrate profile: Thrum

Thrum's work follows the generic orchestrate skills (`/plan-module`, `/build-milestone`, `/review-milestone`,
`/build`). This file holds what's specific to Thrum.

- **Purpose:** [the brief](docs/plans/BRIEF.md): purpose, the V1 exit and the gates.
- **Quality bar:** would pass in a shipped high-end game of this kind, named per milestone, e.g. sea and wakes as in
  Sea of Thieves, skies as in Red Dead Redemption 2, smooth at the display's full refresh. Judged in words against
  inspection items, never against target images.
- **Plans:** docs/plans
- **Evidence:** docs/evidence  *(loop)*
- **Graded examples:** docs/design/GRADED-EXAMPLES.md  *(loop: past verdicts the owner agreed with or overturned)*
- **Test command:** ./dev check  *(loop: CI is manual-only, so this is the gate; documentation-only work may run ./dev check-docs while building)*
- **Merge:** push  *(loop: owner allows verified checkpoints straight to main)*
- **Branch:** main  *(loop)*
- **Remote:** origin  *(loop)*
- **Builders:** two Codex builders  *(owner, 11 Nov 2026: "Yes, run two builders now")*
- **Codex model:** gpt-6.1-sol  *(loop)*
- **Claude model:** claude-opus-5-5  *(loop)*
- **Default host:** codex  *(loop: builders)*
- **Review host:** codex  *(loop: owner, 11 Nov 2026: "move milestone reviews to astra on medium"; design reviews stay on Codex since Claude plans)*
- **Planner host:** claude  *(loop)*
- **Claude planner:** claude-fable-5-2 high  *(loop: owner, 10 Nov 2026)*
- **Codex design reviewer:** gpt-6-astra high  *(loop: owner, 10 Nov 2026)*
- **Codex reviewer:** gpt-6-astra medium  *(loop: owner, 11 Nov 2026: "move milestone reviews to astra on medium")*
- **Claude reviewer:** high  *(loop: owner, 10 Nov 2026; Opus 5.5; used only when a review is sent to Claude by hand)*
- **Codex sandbox:** danger-full-access  *(owner, 14 Nov 2026: agents get full access; the Mac stays unlocked)*
- **Writable roots:** ~/.cargo  *(loop)*

## Starting from a clean clone
`./dev help` lists the commands; `./dev check` builds and tests the workspace. Builders work in their own worktree on
a `build/<module>` branch and push `HEAD:main`. Large captures and recordings go in `tmp/<task>/` under
[the storage rules](docs/ASSET-LIFECYCLE.md); only selected evidence is committed.

## Targets

| Target | What it is | Run | Capture | Edge conditions |
|---|---|---|---|---|
| mac-metal | This M3 Max, 120 Hz ProMotion, Metal | `scripts/run_v1_showcase.py`, `./dev` scenarios | [T2 engine capture](docs/plans/T2-CAPTURE/DESIGN.md): fixed-time images and elapsed-time motion, lossless sources/contact sheets; active-panel rows separate | Low sun, storm, distance, fast camera, full refresh |
| chrome-webgpu | Chrome, portable WebGPU recipe | the browser showcase | [T2 engine capture](docs/plans/T2-CAPTURE/DESIGN.md) M1: submitted fixed checkpoints, realtime output and time records; DOM/browser chrome separate | Same scenarios as mac-metal |
| ios-sim | iPhone 18 Pro simulator; function only, never speed | the iOS route | [T2 engine capture](docs/plans/T2-CAPTURE/DESIGN.md) M2: engine output/time records; `simctl` screenshots/video corroborate simulator function/UI/lifecycle within recorded timing limits | Rotation, background and foreground |

Rules for proof on these targets:
- A visual milestone that claims submitted output or elapsed-time engine motion uses T2 engine capture on its
  target and names the capture mode it selects: `still`, `fixed` or `realtime` on the Mac, `fixed` or `realtime` in
  Chrome. Chrome endpoint images and `simctl` captures alone cannot prove submitted temporal output or elapsed-time
  engine motion. Simulator evidence is function only.
- Engine capture needs no OS Screen Recording grant, by design. A check that claims independence from desktop
  capture requires denied, attributed read-only Screen Recording preflights. Console state is recorded as a
  witness, never an admission or pass condition and never a manual-lock wait. Evidence accepted from locked runs
  stays valid history.
- Built-in-panel cadence is **attended setup**: an unlocked active panel and any Instruments authorization, with
  the agent's actions and review automated. Cadence is qualified with capture OFF; capture-ON arms show content
  and observer cost separately.
- OS-window capture, as in `APPLE-TILE`'s actual-window checks, separately needs attended unlocked setup and Screen
  Recording permission for its observer. That grant is not required by the engine tap and is not inferred from Mac
  panel setup. Native or compositor UI and physical output checks name their own attended reason.
- Never treat engine pixels or movie PTS as display swaps or photons.

Rules for when a proof runs ([why](docs/plans/ARCHITECTURE.md#proofs-run-once-on-main), 15 November 2026):
- **The proof of record** is the script's run of `./dev check` and every item's check on the merged tip. Prove a
  thing on your branch once, merge, and don't prove it again after a rebase: rerun only what the script says
  failed. A visual milestone takes one capture pass, after its last merge.
- **Exact images** are verified on main after the merge, by the main-branch check `exact-images`. No proof is
  needed before a merge. An item whose check is `python3 scripts/exact_baselines.py verify` passes from that
  shared run: set it `pass` with the evidence `main-branch check exact-images`, and don't run it yourself.
- **A lock is about capacity.** A command takes one lock, for the device it loads most, around the part that needs
  it. It never takes a second lock and never runs `merge` inside one. `gpu.alone` is only for a run that needs the
  whole GPU, such as all its memory. It doesn't make a timing trustworthy, because builds elsewhere take no lock.
- **Nothing waits for a person under a lock.** Before an attended run, check that the console is unlocked and the
  panel awake, and only then take the lock. If they aren't, end `BLOCKED needs_access` at once.
- **Timing inside a milestone is a tripwire.** It is taken while other builders work, records the load, and
  fails only on a collapse or a missing measurement. A module's qualified numbers are taken in its last
  milestone, in one sitting with the other builders paused: its builder ends `BLOCKED` asking the lead for that
  sitting, and the lead keeps the others paused until the milestone is Ready for Review.

Label Mac stand-ins for iPhone `proxy:iphone`. No physical iPhone is available, so don't wait for one.

## Rules
- **Exclusive resources:** simulator (the one booted iPhone simulator; an `ios-sim` check takes it)
- **Shared resources:** gpu: 2, chrome: 1  *(owner, 15 Nov 2026; a Mac GPU run takes `gpu` and a run that starts Chrome takes `chrome`)*
- **Lock hold limit:** 30 minutes
- **Main-branch check exact-images:** `python3 scripts/exact_baselines.py verify --full`; every 30m; lock gpu, chrome; timeout 45m  *(owner, 15 Nov 2026; the lead runs `main-check --due` each hour)*
- **Human decides:** changing purpose or the V1 exit; spending; publishing, licensing or deploying; irreversible
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

Some items need a fresh reader, for example CREATOR's M3.1: an agent that has opened only the documents the item
names. A reviewer cannot be that reader, because the review skill and these settings load engineering guidance
before any item is read. The lead commissions the fresh reader as a separate new session, never the builder or the
reviewer, from the brief the module's design names. The builder retains its record as evidence, and the reviewer
judges the record and reruns what it retains.
