---
name: engine-module-design
description: Use when designing or substantially redesigning a Thrum engine module (choosing its technique and representations, then proving them) or reconciling an existing or ported module with its target. Not for ordinary fixes inside an existing design.
---

# Engine module design

**Outcome:** a module whose chosen mechanisms produce every required behaviour across the range it must serve, at a
cost that fits the shipped configuration. It is proven in the running engine and described by one governing design.

You choose the techniques. Research current practice, decide, build, check the result and change course when the
evidence says so. For whatever was commissioned (design only, or design through build), continue through running,
inspecting and correcting without adding approval stops. Research alone doesn't commission a build. Only the actions
[AGENTS.md](../../../AGENTS.md) reserves for the owner need the owner.

## What good looks like

- **Each defining behaviour is shown to work before it's built on.** Analysis, a published result or a small numerical
  reproduction shows the mechanism produces it across the operating range, not only at the demo point. An experiment
  tests a stated prediction; it doesn't stand in for a missing design.
- **It's judged as it ships.** It is judged at the shipped configuration, budget, camera and frame rate. Offline,
  high-sample or close-inspection results are references and diagnostics.
- **It's driven by real causes.** Simulated responses come from physical inputs, not hand-set gains or constants
  tied to a grid size. Authored exaggeration is legitimate when its role is stated and kept separate from the physical
  quantity.
- **It can grow.** Representations can extend to the module's far end (puddles to oceans, near clouds to the
  horizon) without a rewrite. Fixed domains, constant depths and wrapping edges won't do. Build only the commissioned
  scope; neighbours get explicit contracts, not unsolicited implementation.
- **It has one owner per kind of state.** It uses the engine's existing owners: the one sky, the frame contract, the
  capability registry and cooked products. Shared [C4 contracts](../../../docs/design/CONTRACTS.md) are agreed with
  their producers and consumers.
- **It has been checked by eye.** Visible output was watched in motion against references and judged by an independent
  reviewer ([visual review](../../../docs/design/AI-DEVELOPMENT.md#visual-review-and-human-steering)). Owner feedback
  steers; it is never a gate.
- **It's performant by design.** Best-practice foundations, tripwires, tuning later ([performance](references/performance.md)).
- **Its status is honest.** Research, implementation, correctness, visual quality and device measurements are recorded
  separately in the [maturity ledger](../../../docs/design/maturity.json). Unrun or unavailable paths stay
  unqualified; a failure stays open.

## Failure patterns this engine has already paid for

- **Tuning a mechanism that can't make the cue.** Hours of gain, resolution, foam and shading work went into a wake
  whose wave model could not form a Kelvin wedge at varying speed. A short CPU reproduction showed it. When a cue stays
  wrong after tuning, question the model.
- **Correct but invisible.** Physically calibrated wakes and foam came out true to scale and unreadable at the game
  camera. A cue must read where the player sees it.
- **Judged in the lab.** A cloud look judged at seconds per frame shipped as a 288 ms fog. Critics gated on close
  views and offline-paced clips while the real run stuttered.
- **Trials that hold the suspect fixed.** Six rounds of horizon-shelf fixes all kept base height, profile and lighting
  constant.
- **Swinging between cost-only and look-only work.** Chasing invented budgets before the look existed, then ignoring
  cost, produced designs that were heavy by design.
- **Copying the better-looking prototype.** An earlier implementation looked better but was too heavy; it was a look
  bar, not a technique.
- **Attractive frames without the behaviour.** Moving texture passed for growth and breakup. A fade or age label is
  not structural change.
- **Paperwork displacing the defect.** Notes and qualification grew while the visible defect stayed. Contracts and the
  ledger still matter; they just don't count as progress on the cue.
- **Verdicts from descriptions.** Techniques were endorsed from their write-ups and critics' passes were relayed as
  fact, then proved wrong. Trust reproductions, measurements and your own look.
- **Idling at self-imposed stops.** Take the reversible option, record the question and continue.

## Routes

Load only the reference you need:
- **Researching or designing a module:** [research and design](references/blue-sky.md).
- **Comparing with existing code, or porting in:** [reconciliation](references/reconciliation.md).
- **Building and proving:** [implementation](references/implementation.md).
- **A weak behaviour, artifact or integration fault:** [system diagnosis](references/system-diagnosis.md).
- **Cost, budgets, quality tiers or optimization:** [performance](references/performance.md).
- **Getting or giving an independent review:** [review](references/review.md).

Elsewhere:
- **Boundary and whole-engine questions** go to [engine-architecture](../engine-architecture/SKILL.md). Return
  contract changes and unresolved dependencies to that owner.
- **A coordinated programme** follows the [design programme](../../../docs/design/DESIGN-PROGRAMME.md). Report the
  evidence level you actually reached, and don't claim another module's result as this module's proof.
- **Code changes** follow [engine-contributor](../engine-contributor/SKILL.md).

## Working permissions

Without asking, you may:
- research;
- delegate research or review within AGENTS.md's delegation terms;
- write throwaway reproductions in `tmp/<task>/`;
- run local tests, GPU correctness fixtures and captures;
- change your module's design and code;
- replace a mechanism the evidence rejects;
- commit verified checkpoints.

Push, publish and start a port only as AGENTS.md and the owner allow. Port timing for water and clouds belongs to the
owner ([port decision](../../../docs/design/decisions/SEA-AND-CLOUD-PORT.md)).

Keep bulk output under the [storage rules](../../../docs/ASSET-LIFECYCLE.md). Record licences and provenance. The
owner's current policy:
- Restrictive-licence code may inform ideas only; copy nothing.
- No GPL or share-alike code.
- Anything else goes to the owner.
