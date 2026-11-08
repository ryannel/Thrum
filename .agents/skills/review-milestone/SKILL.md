---
name: review-milestone
description: Judges a Thrum milestone from a clean context against its done-when file and inspection list, by running the oracle and inspecting captures from the named target, then records the verdict. Use when a milestone marked for review has all items claimed, at module close, or to review a module design and plan before building. Not for building or fixing (build-milestone).
---

# Review a milestone

Run as Claude Opus 5.5 at medium effort, in a clean context, ideally as the `thrum-evaluator` agent.

**Outcome:** a verdict on one milestone that the generator, the loop and the owner can act on without asking you
anything. It is written to `docs/evidence/<module>/<milestone>-review.md` and summarised in the done-when file.

Your value is that you haven't seen the author's reasoning. Work from the requirements and raw artifacts:
- `docs/design/modules/<MODULE>-DONE.json`: the items, the inspection list and the shipped-game bar;
- the design, for what the module must do;
- the commit range;
- the captures and logs.

Don't read the author's progress narrative until you've formed your own view.

## Done when

1. **Each item has a verdict:** `pass`, `fail` or `can't tell`, with evidence.
   - **Oracle items:** run the check yourself on the named target, at the commit under review.
   - **Inspection items:** inspect a fixed-camera frame sequence (or a contact sheet) captured offscreen from that
     build on the named target, converted to PNG.
     - Say whether you inspected a sequence or stills, at what size, and which build.
     - Judge against the item and the `bar`. The bar is acceptance in a shipped high-end game of this kind, described
       in words. There is no target image to match.
     - Cover transitions such as starts, stops, turns and camera moves, where faults hide.
     - Capture what you need yourself if it's missing.
   - **Review items:** read the code or contract change across its producers and consumers.
2. **Everything else wrong is listed, ranked by consequence,** each marked verified (file and line, a reproduction, or
   a frame) or suspected. Include items too weak to catch the plausible wrong substitute.
3. **The verdict is recorded:**
   - In the review file: the commit, the build, the target, each verdict and the findings.
   - In the done-when file: each item's `status`, and an `evidence` link to the review file.
   - If every item passes, set the milestone's `state` to `done` and record `agent_reviewed` with the evidence link in
     [the maturity ledger](../../../docs/design/maturity.json).
   - If any item fails, increase the milestone's `fails` by one, and add the failure findings to the plan as new work
     items under the milestone. Append them; don't rewrite the plan. `scripts/agent_loop.py next` reads `fails` to
     decide the escalation.
4. **You end with one line:** `PASS <milestone>` or `FAIL <milestone>: <n> items`.

## Reviewing a design and plan

When asked to review a module's design and plan before building:
1. Derive what the module must do from its brief alone.
2. Compare that with the design and plan.
3. Report omissions, unsupported additions, mechanisms not shown to make their cues, milestones that wouldn't show the
   behaviour, and items that couldn't be judged.

## Pointers

- [Review](references/review.md) covers what to challenge, how to close the loop, and calibration failures this engine
  has already paid for. Read its calibration section before judging visual output.
- [Graded examples](references/graded-examples.md): verdicts the owner has confirmed or overturned. Match their
  standard.

## Boundaries

- **Read-only on code.** You may build, run the oracle and capture. You may edit only the review file, the done-when
  file, the plan's work list and the maturity ledger.
- **Don't fix what you find.** The generator does that, so findings stay independent.
- **Commit and push** the verdict files.
