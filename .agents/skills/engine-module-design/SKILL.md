---
name: engine-module-design
description: Use when designing, substantially redesigning, building or proving a Thrum engine module (choosing its techniques, planning the work and proving it in the game), or reconciling an existing or ported module with its target. Not for ordinary fixes inside an existing design.
---

# Engine module design

**Outcome:** a module whose mechanisms produce every required behaviour across the range it must serve, at a cost
that fits the shipped configuration, proven in the game and described by one current design.

Thrum is built from established, published techniques, chosen as the best modern option and guided by the
engine's design principles. Most of what a module needs has been solved in shipped engines and research. The bet is
that agents can go further than usual on performance, writing specialised, highly optimised code from scratch where
it pays. So research before you build, design the whole module, plan it as milestones, then work through them. You
choose the techniques, the research, the experiments and how to break the work down.

## Design, then plan, then build

- **Research and design first** ([research and design](references/blue-sky.md)). Start from what the player must
  see, with operating ranges. Find how shipped engines and current research solve it, and check that the chosen
  mechanism can make each cue across the range before you build on it. Earlier work in this repository and in
  Broadsides is evidence and a look bar, not a technique to copy.
- **Keep one design** ([what it covers](references/design.md)). Revise it in place so it always reads as current;
  don't append amendments.
- **Plan it as milestones** ([plan](references/plan.md)). Each milestone is one outcome you can see in the game, with
  how you'll know it's done; the riskiest behaviour comes first. You own the milestones and their breakdown: when one
  closes, start the next. Commit the plan with the work, since it is how you, reviewers and the overseer see where
  the work is.
- **Build and prove** ([implementation](references/implementation.md)). A visual milestone opens from a reference the
  owner has approved and closes when the game matches it side by side, watched in motion: through the module's public
  interface, at the shipped configuration and simulation rate, with an independent reviewer agreeing. Measurements
  explain a mismatch; they are not the done-when. A lab harness or close-up still is a diagnostic.

## Staying on course

- **An experiment is a red flag.** Shipped games and published work have solved almost everything a module needs, so
  needing to try things usually means the research or the design is missing. Find how it is done first; an experiment
  answers a question the design has, has a stopping point, and is logged in the plan.
- **After about three failed attempts at one cue, stop trialling it.** Commission research on how shipped games and
  the physics make that cue read, and work the milestone's other items meanwhile. Then propose one call in the plan
  and tell the overseer:
  - *park* it as a named later spike with what you learned;
  - *re-scope* the milestone so it no longer hinges on the cue;
  - *foundational*: it could change the module's design. The overseer or owner confirms this call, because the agent
    deep in a cue is the one least able to judge it. Once confirmed, the research note ends in a design choice, not a
    list of things to test: revise the design with a cost account, have it reviewed, then change the runtime once.

  Until a call is confirmed, the cue stays parked; analyses, bounds and screens on it count as attempts.
- **Done-when that doesn't move while work items pile up means the milestone is too big or judged wrongly.** Split or
  re-scope it by the plan's rules. Parking and re-scoping this way are part of owning the milestone, not lowering the
  bar: the parked work stays named in the plan.
- **Work that serves no milestone** either earns a place in the plan or waits.
- **Nothing waits on the owner, a screen, a quiet machine or a long job.** The owner reviews in one batched window a
  day. Queue what needs it in the plan with your recommendation, start long jobs in the background, commission
  independent reviews yourself, and do the work these don't gate. A goal is blocked only when no milestone work can
  proceed.
- **The overseer may steer, or correct a milestone or your goal.** Update the plan, log why, have a changed milestone
  reviewed, and carry on.

## Review

Use independent review where it changes decisions ([review](references/review.md)). At minimum: the design and plan
before you build on them, and each milestone in the game before you call it done. Commission reviews yourself and
carry on while they run. Record each on the plan's `Reviews:` line: the revision or build, a link to the reviewer's
brief, and the verdict. A failed review sends you back to the design, not to a stop.

## Routes

Load only what you need: [research and design](references/blue-sky.md) · [design](references/design.md) ·
[plan](references/plan.md) · [reconciliation](references/reconciliation.md) for existing or ported code ·
[implementation](references/implementation.md) · [system diagnosis](references/system-diagnosis.md) for a weak
behaviour or integration fault · [performance](references/performance.md) · [review](references/review.md).

Boundary and whole-engine questions go to [engine-architecture](../engine-architecture/SKILL.md); code changes follow
[engine-contributor](../engine-contributor/SKILL.md); a coordinated programme follows the
[design programme](../../../docs/design/DESIGN-PROGRAMME.md). Oversight uses
[engine-oversight](../engine-oversight/SKILL.md). Capability status stays in the
[maturity ledger](../../../docs/design/maturity.json).

## Reporting and permissions

Ground progress claims in results from this session, and say plainly what is unverified, approximate or failed.

Without asking, you may research, delegate research, review or bounded build slices, write throwaway reproductions in
`tmp/<task>/`, run tests, fixtures and captures, change your module's design, plan and code, replace a mechanism the
evidence rejects, and commit verified checkpoints. Push, publish and start a port only as the AGENTS.md of the
repository you work in and the owner allow; port timing for water and clouds is the owner's
([port decision](../../../docs/design/decisions/SEA-AND-CLOUD-PORT.md)). Keep bulk output under the
[storage rules](../../../docs/ASSET-LIFECYCLE.md) and record licences and provenance. Restrictive-licence code may
inform ideas only; copy nothing. No GPL or share-alike code or data. Anything else goes to the owner.
