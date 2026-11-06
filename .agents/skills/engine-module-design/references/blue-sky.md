# Research and design

The outcome is a saved design that another implementer could build from without inventing a core mechanism. Each
defining behaviour is traced from its driver to what the player sees, and is shown to work across the full range the
module must serve.

## Start from the experience, not the code

- **Connect to the engine.** Take the required experiences, responsibility, consumers and budget from the engine
  brief. Use [engine architecture](../../engine-architecture/SKILL.md) only for questions that cross module
  boundaries. A proposed boundary change goes into both the brief and the module design.
- **Write the target before reading existing code or old plans.** If you have already seen them, say so; don't claim
  a blind study.
- **List outcomes, not subsystems.** Inventory what must be observable: presentation, dynamics, queries,
  interactions, authoring and lifecycle. For each, record its basis: explicit requirement, necessary implication,
  assumption, option or exclusion.
- **Turn broad words into cues.** Words such as "realistic", "natural" or "convincing" become specific cues: a Kelvin
  wedge that keeps its angle as the boat slows, a darker flat cloud base at the condensation level, foam that persists
  and decays.
- **Make each cue readable where it's used.** It must read at the intended camera, distance and speed. A physically
  calibrated effect that the player can't see fails the cue.
- **State the operating range for each cue.** Give scales, speeds, distances, lighting and transitions (for example
  start, stop, turn, resize, device loss, or load and unload). For a module meant to grow, such as water from puddles
  to oceans, choose representations that extend to the far end without a rewrite. Build and prove only the
  commissioned scope, though, and give neighbouring systems explicit contracts, not unsolicited implementation.
- **Use a reviewer to find what you missed.** Where delegation is available, give a fresh subagent the brief alone
  and have it derive the capabilities independently ([review](review.md)), then reconcile its findings.

## Research the decisions that matter

Prefer current primary sources:
- papers and supplements;
- author code;
- shipped-game talks and developer articles;
- official engine documentation and source.

Search for recent successors and competing whole-system approaches, not only component algorithms.
- **Read sources substantively.** Cite the slide, page or timestamp, and say what you actually inspected. A title or
  abstract supplies no algorithm.
- **Trace reference code end to end.** Pin the revision and follow the actual producer and consumer path. Record the
  licence. Restrictive-licence code may inform ideas only: copy nothing. No GPL or share-alike code or data.
- **Prefer the modern technique.** An older method needs a stated reason that it beats the current one here. Existing
  code, including a better-looking earlier prototype, is a look bar, not a technique choice. It may be heavy or wrong
  by design.
- **Stop when more sources would not change the decision.**

Keep a compact evidence account for each consequential choice:
- the alternatives considered and why this one was chosen;
- the sources and what was inspected;
- the assumptions and cost conditions;
- what would change the decision.

## Check the mechanism before you invest in it

For every defining cue, show that the chosen mechanism produces it across the stated operating range. Use analysis, a
published result or a small numerical reproduction: a short CPU script of the core equations is often the cheapest
decisive evidence. Then ask:
- **Does the behaviour hold across the range?** Check that it still holds as speed, scale or lighting changes, not only
  at the demo point.
- **Is each amplitude derived from physical inputs?** Mass, draft, velocity, depth and optical density should set it,
  not a hand-set gain or a constant that changes with grid resolution.
- **Do the boundaries behave?** Shores, obstacles and domain edges must reflect, absorb or hand off. They must not
  wrap or vanish.
- **Will it compose?** It must layer with the module's other regimes, such as swell plus wake, or procedural plus
  baked clouds, without replacing them.
- **Can it run within the shipped budget?** A look that exists only in an offline or reference configuration is a
  comparator, not a design.

A cue the mechanism can't produce is a design defect. Change the mechanism; don't plan to tune it in later.

A reproduction or experiment tests a stated prediction or settles an uncertainty that analysis can't. It doesn't stand
in for a missing design. Analytical confidence doesn't turn unrun numerical, perceptual or device measurements into
passed evidence; keep those visible as unrun.

## Design the whole system as it will run

Choose the governing relationships, state, representations, owners, update order and public contracts, including
prepared content and authoring. Prefer combinations already proven in production; justify departures. Credible
individual techniques do not establish that they work together.

- **Causal chain.** For each defining behaviour, connect driver → state change → operator → representation →
  observed response. Fill publication gaps with explicit, testable deductions. Naming a governing law is not a
  mechanism.
- **Handoffs.** For each consequential connection, name the quantity, its producer and consumers, units, frame,
  timestamp, sampling and normalization. Assign every contribution exactly once across normal, bypass and fallback
  paths. A surface can look right while contact queries describe a different surface.
- **Roles and placement.** Give authoritative state, derived detail, prepared content and reference comparators
  explicit roles. Choose each consumer's fidelity, update rate and CPU or GPU placement. Shared causes don't require a
  universal solver.
- **Worked cases.** Walk the initial state, a normal frame, a coupled interaction and a consequential transition or
  failure in execution order. Estimate the combined cost with the important consumers active together.
- **Creator journey.** Trace an editable source through preparation, versioned products, loading and observation. Then
  change the source and trace what rebuilds, invalidates or resets.
- **Shared owners.** Name one owner for each kind of state. Check the engine's existing owners (for example the one
  sky, the frame contract and the capability registry) before creating a parallel one.
- **Reuse.** For a reusable module, cover the supported capability profiles and their important combinations, and
  optional presentation versus the services other consumers keep. State the disabled cost, and how a game registers,
  replaces or removes the module through the engine's composition model. A flag that hides output while all the work
  continues is not modularity.

Keep one canonical design. Track each decision's status (proposed, provisional, selected, superseded) separately from
implementation and proof. Experiments link back to the design rather than becoming competing targets.

The design is build-ready when an implementer can follow the worked cases without inventing a core algorithm,
coupling or convention. Tuning values and bounded feasibility questions may stay open, labelled as such. For a
substantial design, try building a hard component on paper from the saved text, and resolve each point where you
would have had to invent something.

## Plan the proof around what could invalidate the design

- **Prove the riskiest behaviour first.** Put the least-established defining behaviour first: before broad
  infrastructure, content production or polish.
- **Make the first proof a small complete experience.** Use real consumers, representative inputs, normal
  presentation and the intended distances and speeds. Shrink the world or content count, but keep the interaction that
  could break the model.
- **Name a plausible wrong substitute and the observation that rejects it.** For example, a single-speed wave equation
  can pass a still-image check at one speed, but it fails a check of wake angle against speed.
- **Check structural change, not its presentation.** For evolving structure, such as clouds growing and breaking up
  or foam forming and decaying, separate transport, deformation, creation, destruction and changes in connectivity.
  Moving texture, an age label, a global scale or a fade doesn't show them, and correct reprojection can hide absent
  evolution.
- **Check motion with motion.** A still cannot qualify motion. Normal speed shows pacing; accelerated speed shows
  repetition and missing development.
- **Separate error sources.** Distinguish missing-model error from sampling error: more resolution cannot supply an
  absent mechanism.

Keep deferrals explicit: a bounded first slice can't establish the whole commissioned set, and a deferral doesn't
quietly shrink the deliverable.

Save the recommendation, evidence account and dependency-ordered proof plan, then continue into
[implementation](implementation.md) if the build was commissioned.
