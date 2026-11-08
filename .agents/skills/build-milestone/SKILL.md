---
name: build-milestone
description: Builds and verifies the next milestone of a Thrum engine module from its plan and done-when file, in Claude or Codex. Use to implement, fix or refactor engine code toward a milestone or the active build-plan gate, including ordinary fixes and small slices. Not for designing or re-planning a module (plan-module), engine-wide boundaries (plan-engine) or judging a milestone (review-milestone).
---

# Build a milestone

Run as Codex `gpt-6.1-sol` at medium effort, or as Claude Opus 5.5 at medium effort for a bounded slice.

**Outcome:** the next open milestone in `docs/design/modules/<MODULE>-PLAN.md` runs correctly and looks right on its
named targets, and every item in `<MODULE>-DONE.json` for that milestone has `status: pass` with evidence. It is
pushed to main.

Thrum is a showcase of AI-driven engineering: butter-smooth, high-end visuals and performance beyond what a human
team would build and maintain. Without a module plan, for example an ordinary fix, the work serves the active gate in
[the build plan](../../../docs/BUILD-PLAN.md), and the done-when is the fix's own test.

## Done when

1. **Every item in the milestone is `pass`, with evidence.** You may change only items' `status` and `evidence`, and the
   milestone's `state`: `in_progress` when you start, `done` when every item passes and `review` is `false`. If an item
   is wrong or can't be met, say so in your report; the rest of the done-when file isn't yours to edit.
   - `oracle` items pass when their command exits 0 on the named target.
   - `inspection` and `review` items: record your own look and the captures as evidence. The evaluator decides the
     final pass for milestones with `"review": true`.
2. **The change is on main.** Rebase on `origin/main`, run a fresh `./dev check` on the rebased tip, then push without
   force. GitHub CI runs only by hand, so this check is the gate for main. If the push is rejected, rebase and check
   again.
3. **The plan's status lines and your progress note are current.** The progress note is the plan's `Now:` and the
   log: a decision log with reasons, and claims kept apart as verified, approximate or open.
4. **You end with one report line,** `DONE`, `BLOCKED: <what you probed, the evidence, what would unblock it>` or
   `NEEDS_CONTEXT: <what is missing>`, followed by the evidence. `DONE` needs fresh evidence from this session.

Before claiming items, check that each one is complete, clear and checkable on its target. If one isn't, report it.

## Staying on course

- **Research before experimenting.** Shipped games and published work have solved most of what a module needs.
  Needing to try things usually means the research or the design is missing, so find how it is done first, favouring
  current state-of-the-art and shipped practice. An experiment answers a question, has a stopping point, and goes in
  the log. Use subagents for hard research where it helps (on Codex, the `thrum-research` agent).
- **If a cue won't come after about three attempts,** stop tuning it. Research how shipped games and the physics make it
  read ([system diagnosis](references/system-diagnosis.md)), and work the milestone's other items meanwhile. If the
  mechanism can't make the cue, report `BLOCKED` with the evidence. Escalation to a fresh session and then to the
  planner is counted by the loop, not decided by you.
- **Contract or design changes** don't happen locally. Write the proposal into the plan's log and your report, and
  carry on with work that doesn't depend on it.
- **How to build and prove in the game:** see [implementation](references/implementation.md).

## Pick the work

- **Extend running paths before hardening inactive ones.** Code with no consumer by the end of a slice does not land,
  and parked tracks stay parked.
- **Aim for showcase quality, not parity.** Prefer named techniques, paper reproductions and specialisations agents
  can sustain. Claimed wins need measurements. Record sources and evidence in
  [the technique register](../../../docs/design/AI-PERFORMANCE.md#technique-register).
- **Native and browser.** Native leads and uses every hardware feature. The browser runs the portable recipe at
  showcase quality and never limits native.
- **Crates.** Before building a general subsystem, look for a proven crate that meets the targets (Wasm, mobile,
  determinism where needed). Build your own where a specialised path is the point and can show its win.

## Match rigour to the state

| What the code touches | On violation |
| --- | --- |
| Authoritative world state, saves, wire input, security boundaries | Refuse with a typed error, leave state unchanged, test the failure path |
| Presentation, VFX, audio, cosmetic budgets, frame resources | Degrade: clamp or drop, count it, log it; `debug_assert!` in development. Never stall a tick or fail a frame |
| Authored content | Validate with tolerances; importers repair what they safely can (weld, drop degenerates) and warn |

Don't add exact multi-precision arithmetic, certificates, logical byte-credit accounting or diagnostic-ordering
contracts unless an observed failure needs them and a simpler bound failed.

## Shape the code

The lint policy and `./dev check` enforce suppression discipline, API reachability, test placement and size
backstops. When designing types, APIs, errors, decoders or test support, read [rust code](references/rust-code.md).
- **One path per behaviour.** Express variants with generics, typestate or a builder.
- **Engine crates hold no game schema.**
- **Keep internals `pub(crate)` behind a small facade,** with one error enum per public boundary.
- **When a size note or limit fires,** split by responsibility, never by line count.

## Hot paths

For per-tick, per-frame and per-message code:
- Reuse buffers and arenas, and move values.
- Store iterated data as columns, and validate once.
- Use a fast non-cryptographic hasher for internal maps.
- Account cost before choosing a representation
  ([the cost account](../../../docs/design/AI-PERFORMANCE.md#the-cost-account-in-each-owning-design)).
- Select accelerated paths from the capability registry and keep a portable fallback that the same tests exercise
  ([hardware-native decision](../../../docs/design/decisions/HARDWARE-NATIVE-ENGINE.md)).

## Verify

- **Tests** cover behaviour and plausible failure; authority work covers rejected, stale and duplicate input.
- **Timing** lives in `benches/` with stored baselines. Allocation claims use the counting-allocator assertion.
- **Parsers:** every parser of untrusted bytes gets a fuzz target.
- **Device, frame-time and visual claims** come only from runs on named targets:
  - this M3 Max;
  - the iPhone simulator (function only, never speed);
  - Chrome.

  Label Mac stand-ins "proxy". Device timing runs alone on the GPU: wrap timing and capture
  runs in `python3 scripts/agent_loop.py gpu -- <command>`, which holds a machine-wide lock.
- **Regressions.** Investigate visible hitches, about 25% regression from a comparable baseline, or unexpected scaling
  ([performance](../plan-module/references/performance.md)).
- **Visible output:** capture fixed cameras and a frame sequence from the exact build on the named target, offscreen.
  Convert captures to PNG for review, since capture writes `.ppm`. Look at them yourself against the inspection list.
- **Status** goes in [the maturity ledger](../../../docs/design/maturity.json), separately for implementation,
  correctness, visuals and device. Evaluator passes record `agent_reviewed`.
- **Evidence** is at most one page per milestone, linked from the done-when items. Bulk output follows
  [the storage rules](../../../docs/ASSET-LIFECYCLE.md).

## Permissions

- **Without asking, you may:**
  - research;
  - delegate research to subagents;
  - write throwaway reproductions in `tmp/<task>/`;
  - run tests, fixtures and captures;
  - change this module's code;
  - update the plan's status and log;
  - commit and push verified checkpoints to main.
- **Never:** force-push or rewrite history.
- **What goes to the owner:** changing purpose or the V1 exit; spending; publishing, licensing or deploying; irreversible actions. Continue unblocked work meanwhile.
- **Licences:** no GPL or share-alike code or data.
- **Broadsides:** don't modify it.

## Build outputs

- **Run `cargo clean` in your worktree** when work stops. A daily sweep deletes stale `target` directories.
- **Remove the worktree once the commit has landed,** that is, once `git merge-base --is-ancestor <commit>
  origin/main` passes after a fetch.
- **In fan-out,** each stream keeps one target directory across its milestones.
