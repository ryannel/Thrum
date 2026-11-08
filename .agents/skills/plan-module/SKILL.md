---
name: plan-module
description: Designs one Thrum engine module or engine gate and plans it as milestones, each with a done-when file. Use when starting, substantially redesigning or re-planning a module, when reconciling existing or ported code with a target design, or when a milestone has failed review four times. Not for building a milestone (build-milestone) or for engine-wide boundaries and contracts (plan-engine).
---

# Plan a module

Run as Claude Opus 5.5 at high effort.

**Outcome:** the module has one current design and a plan of milestones. A capable generator can build each
milestone without asking, and an evaluator can judge it. Concretely:
- `docs/design/modules/<MODULE>.md` is the design ([what it covers](references/design.md)).
- `docs/design/modules/<MODULE>-PLAN.md` is the plan, with a status, the milestones and a log.
- `docs/design/modules/<MODULE>-DONE.json` holds every milestone's done-when items ([plan and done-when](references/plan.md)).

Thrum is built from established, published techniques, chosen as the best modern option. Most of what a module needs
has been solved in shipped engines and research. The bet is that agents can go further than usual on performance, so
research before designing, and design the whole module before planning it.

## What to produce

- **The brief.** Start from the module brief that `/plan-engine` wrote (in the architecture brief or
  `docs/design/modules/`). If there is none, write a short one and flag it for `/plan-engine`.
- **Research and design** ([research and design](references/blue-sky.md)).
  - Start from what the player must see or feel, with operating ranges.
  - Find how shipped engines and current research solve it.
  - Check that the chosen mechanism can make each cue across the range before anyone builds on it.
  - Earlier work in this repository and in Broadsides is evidence, not a technique to copy.
- **Results, not technique detail.** Keep the design at the level of mechanisms, contracts and cost. Leave
  implementation technique to the generator's research. A wrong detail in a plan carries through the whole build.
- **Milestones.** Each milestone is one small outcome you can see in the game, riskiest first.
  - Each done-when item is behaviour a person can verify, tied to a named target, and proved by the oracle, by an
    inspection list or by review.
  - Visual items use an inspection list judged in the game, with the bar set by shipped high-end games. They never use
    a target image.
- **Performance** ([performance](references/performance.md)): give the cost account and tripwires, never invented
  budgets.
- **Existing or ported code:** reconcile it after choosing the target ([reconciliation](references/reconciliation.md)).

## Done when

- The design, plan and done-when file exist and agree. Run a read-only pass that compares them with
  `docs/design/CONTRACTS.md` and the architecture brief, and record the result in the plan's log. A conflict with a
  contract is a proposal to `/plan-engine`, not a local edit.
- An independent reviewer has derived what the module must do from the brief alone and compared that with the design
  and plan ([review](../review-milestone/references/review.md#the-two-reviews-that-matter-most)), and you have acted
  on its findings. Record the verdict on the plan's `Reviews:` line, with the revision it saw.
- The plan's `Milestone:`, `Now:`, `Next:` and `Blocked:` lines are current.

## Re-planning a failed milestone

When a milestone has failed review four times, read the evaluator verdicts and the progress note. Then decide one of:
- **Split:** the milestone was too big.
- **Re-scope:** it hinged on a cue another owner controls.
- **Redesign:** the mechanism can't make the cue. Revise the design with a cost account and have the change reviewed.

Edit the plan and done-when file, log why, and reset the milestone's `fails` count to 0.

## Boundaries

- **What you may do without asking:**
  - research;
  - delegate research or review to subagents;
  - write throwaway reproductions in `tmp/<task>/`;
  - edit this module's design, plan and done-when file;
  - commit and push verified checkpoints to main.
- **What goes to `/plan-engine`:** boundary questions, contract changes and gate order.
- **What goes to the owner:** spending, publishing and changes of purpose.
- **Licences:** restrictive-licence code may inform ideas only, and nothing may be copied from it. No GPL or
  share-alike code or data is allowed.
- **Capability status:** this lives in [the maturity ledger](../../../docs/design/maturity.json), not in the plan.
