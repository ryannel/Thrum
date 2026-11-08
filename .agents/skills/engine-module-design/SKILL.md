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
- **Plan it as milestones** ([plan](references/plan.md)). Each milestone is an outcome you can see in the game, with
  how you'll know it's done; the riskiest behaviour comes first. Break each milestone into work however suits you,
  tick it off as you go, and keep a short status at the top of the plan. Your goal is to work through these
  milestones to the plan's end on your own; when one closes, carry straight on to the next. The plan is how you, reviewers and the
  overseer see where the work is, so commit it with the work.
- **Build and prove** ([implementation](references/implementation.md)). A milestone is done when it works in the game
  through the module's public interface, at the shipped configuration and simulation rate, watched in motion (an offscreen capture of the running game counts), and an
  independent reviewer agrees. A lab harness or close-up still is a diagnostic.

## Staying on course

- **This is not new ground, so an experiment is a red flag.** Shipped games and published work have solved almost
  everything a module needs. Needing to try things usually means the research or the design is missing. Before a
  trial, find how it is done; an experiment answers a question the design has, has a stopping point, and is logged.
- **After about three failed attempts at one cue, stop and decide.** Don't run a fourth trial. Find how shipped games
  and the physics make that cue read (delegate the research and keep working), take it back to the design, and record
  what changes. Then make the call and log it in the plan:
  - *Foundational* (it could change the module's design, or blocks the milestone's game outcome): push through with
    research and design work.
  - *Not foundational*: park it as a named later spike with what you learned, and move on to the milestone's other
    work.
  Tell the overseer which call you made.
- **Re-read this skill at each milestone start and whenever an experiment fails.** Your goal should cite it.
- **Work that serves no milestone** either earns a place in the plan or waits.
- **Long jobs run in the background.** Solvers, bakes, builds and benchmarks shouldn't hold your turn. Start them,
  work other plan items, and check them between items.
- **The overseer may steer, or correct a milestone or your goal.** When it does, update the plan, log why, have a
  changed milestone reviewed, and carry on.

## Review

Use independent review where it changes decisions ([review](references/review.md)). At minimum: the design (and its
plan) before you build on it, and each milestone in the game before you call it done. Commission reviews yourself and
carry on with other work while they run. Record each one on the plan's `Reviews:` line: what was reviewed (revision
or build), a link to the brief the reviewer was given, and the verdict. A failed review sends you back to the design, not to a stop.

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

Without asking, you may research, delegate research or review within AGENTS.md's terms, write throwaway
reproductions in `tmp/<task>/`, run tests, fixtures and captures, change your module's design, plan and code, replace
a mechanism the evidence rejects, and commit verified checkpoints. Push, publish and start a port only as
[AGENTS.md](../../../AGENTS.md) and the owner allow; port timing for water and clouds is the owner's
([port decision](../../../docs/design/decisions/SEA-AND-CLOUD-PORT.md)). Keep bulk output under the
[storage rules](../../../docs/ASSET-LIFECYCLE.md) and record licences and provenance. Restrictive-licence code may
inform ideas only; copy nothing. No GPL or share-alike code. Anything else goes to the owner.
