# Module design

What a Thrum module design must contain, and the engine rules for getting there. The orchestrate planner skill covers the process.
A design lives at `docs/plans/<MODULE>/DESIGN.md`, and it is what the plan, the done-when file and the reviewer check
against. Keep it reading as what is true now: revise sections in place, rather than appending amendments.

Contents: [Start from the experience](#start-from-the-experience) · [Research](#research) ·
[Show the mechanism works](#show-the-mechanism-works) · [What the design covers](#what-the-design-covers) ·
[Existing code](#existing-code)

## Start from the experience

Write the target before reading existing code or old plans. If you have already seen them, say so in the design;
don't claim a blind study.

Turn broad words into cues the player can see. "Realistic water" might become:
- a Kelvin wedge that keeps its angle as the boat slows;
- a darker, flat cloud base at the condensation level;
- foam that persists and decays.

Each cue has an operating range (scales, speeds, distances, lighting, and transitions such as start, stop, turn,
resize and device loss). It must read at the game's camera and distance. A physically calibrated effect the player
can't see fails its cue: true-to-scale wakes and foam once shipped unreadable at the game camera.

A module meant to grow, such as water from puddles to oceans, picks representations that reach the far end without
a rewrite. Build and prove only the commissioned scope, though. Neighbours get explicit contracts, not unsolicited
implementation.

## Research

Prefer the modern technique: current papers with their author code, shipped-game talks, and engine source. An older
method needs a stated reason that it beats the current one here. Existing code, including a better-looking earlier
prototype or the Broadsides port, is a look bar and evidence, not a technique choice. It may be heavy or wrong by
design (`docs/design/decisions/SEA-AND-CLOUD-PORT.md`).

When you trace reference code, pin its revision and record its licence. No GPL or share-alike code or data comes into
Thrum. Restrictive-licence code may inform ideas only.

For each consequential choice, keep a short account in the design:
- the alternatives;
- what you inspected;
- the assumptions and cost conditions;
- what would change the decision.

## Show the mechanism works

For every defining cue, show that the chosen mechanism produces it across the operating range, before the build
depends on it. A short CPU script of the core equations is often the cheapest decisive evidence. Check that:

- the behaviour holds as speed, scale and lighting change, not only at the demo point;
- each amplitude follows its real inputs (mass, draft, velocity, depth, optical density) across the range, and
  no constant changes with grid resolution. A constant tuned by eye is fine when it holds across the range;
- boundaries reflect, absorb or hand off, and don't wrap or vanish;
- it composes with the module's other regimes (swell plus wake, procedural plus baked clouds) instead of replacing
  them;
- it runs at the shipped configuration.

A cue the mechanism can't produce is a design defect. Change the mechanism; don't plan to tune it in later.

This is a check that the mechanism can make the cue, not a pass mark for the build. The bar is the most plausible
result as fast as possible (`ORCHESTRATE.md`, 17 November 2026). So a design names no numerical acceptance for
rendering: no ceiling against an `f64` or photon reference, no bound a work package measures first and then asserts,
no rule that a miss ends `BLOCKED`. Pick the cheapest mechanism that reaches the shipped-game look, and say which
terms a builder may tune by eye. Numerical proof belongs to what a game or another module reads and to the
contracts between modules.

## What the design covers

- **Purpose and scope:** what the module is for, its consumers, what is commissioned now and what is deferred.
- **Cues:** each one with its operating range, where it must read, and the shipped-game bar it is judged against,
  stated in words.
- **Proof:** for each cue, the inspection that judges it and the tripwire that records its cost. For each
  contract with another module or with a game's queries, the test that holds it. Weigh each proof against the
  fault it would catch.
- **Techniques and evidence:** the account above.
- **The system as it runs:**
  - state and owners;
  - representations;
  - update order;
  - CPU or GPU placement;
  - handoffs to neighbours;
  - the creator's path from source file to what is seen;
  - a normal frame walked in order with its cost.
- **Owners:** before adding one, name the engine's existing owners this module uses: the one sky and atmosphere,
  the frame contract, the capability registry, cooked products. A shared invariant goes in
  `docs/design/CONTRACTS.md`.
- **Risks:** the likeliest failures, the early sign of each, and the plausible wrong substitute for each defining
  cue. A wrong substitute is something that could look right while missing the behaviour, such as moving texture in
  place of cloud growth.
- **Performance:** the cost account and tripwires.
- **Open questions:** each with the experiment or work package that will settle it.

The design is ready to build when an implementer can walk its normal frame and a coupled interaction without
inventing a core algorithm, coupling or convention. Tuning values may stay open, labelled as such.

## Existing code

When a module already has code, or is being ported, the saved design is the target. Code earns its place by fitting
the design's model and interfaces. Its volume, the effort it cost, and tests of old assumptions are not evidence of
fit.

A ported module maps onto the engine's existing owners. It doesn't bring a parallel stack along: no second sky, no
private frame loop, no copy of a device request. Prefer a clean boundary to an adapter that keeps an inferior design
alive. A compatibility layer needs an actual consumer and a condition for retiring it.

Plan the transition in complete, dependency-ordered slices. A temporary comparison harness may keep both versions,
as long as each run has one authority for each contribution.
