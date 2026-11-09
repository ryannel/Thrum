# Cooked draw path plan

Design: [COOKED-DRAW.md](COOKED-DRAW.md) · [brief](COOKED-DRAW-BRIEF.md) · Done when:
[COOKED-DRAW-DONE.json](COOKED-DRAW-DONE.json). Owner: G5 (`thrum-asset-gpu`), under
[the G5 decision](../decisions/G5-CONTENT-ADDRESSED-LOOP.md).
Reviews: design review owed (none yet).
Milestone: M1 Cooked scene draws on adapters without indirect execution (open)
Now: Planned 9 November 2026; nothing is built. The design review and the render owner's agreement on the two additive interface changes are owed.
Next: Independent design review. Then one Codex builder builds M1. T2-CAPTURE M2 resumes once COOKED-DRAW:M1 is accepted.
Blocked: none. The loop holds M1 until a design review passes.

## Ownership and agreements

- **G5** owns `thrum-asset-gpu`: path selection, the direct draw, path-independent history, stats, and the cooked
  tests and parity harness.
- **The render owner (Thrum Core, V1 graphics execution; implements F2's C04 registry)** must agree to two additive
  changes, recorded in the design's contracts table:
  - `DeviceCapabilities` records the adapter's downlevel flags;
  - `SceneSurface` lends them.

  Neither changes any signature that call sites use.
- **The V1 game and G3b iOS shell owners** must agree that the iOS route attaches cooked content through
  `attach_cooked_scene`.
- **G4 seam 3** shares `draw.rs`. There is no sequencing dependency. Merge with care.

Ask for these agreements before the builder starts. The builder may build against them as proposed. If an owner
objects, the change returns here as a re-plan. The builder never redesigns another owner's interface inside this
milestone.

## M1 Cooked scene draws on adapters without indirect execution: open

Work, riskiest first:
- [ ] **Simulator first.** Build the record field, the surface field, selection and the direct draw. Then attach
      the cooked valley in the iOS route and run it on a fresh owned simulator, before any polishing. This settles
      whether the simulator reveals a further unsupported feature. A feature this module owns is fixed here.
      Anything owned elsewhere is reported `BLOCKED`, naming its owner (→M1.7).
- [ ] The diagnostic record mask, and the cooked GPU suite run under both records in `./dev check`. This covers
      discard, capture-no-history, device replacement, remote feed, material promotion and casters. The
      motion-history flag moves to a step both paths run (→M1.1, M1.2).
- [ ] `scripts/cooked_draw_parity.py`, built on the T2 Mac and browser taps:
  - the `paths` arm, forced direct against indirect, on one build (→M1.3);
  - the `indirect` arm, base against claim, on mac-metal and chrome-webgpu (→M1.4, M1.5).

  Both arms compare decoded output bytes and include a same-path rerun control. Cook the store once, and use it for
  both builds.
- [ ] Record `docs/evidence/render/cooked-draw/BASE`: the parent of this module's first commit in the integrated
      history. Integrate the module's commits contiguously on the main tip, so that BASE..claim holds only this
      module's changes. Re-record BASE after each rebase.
- [ ] A command diff of the indirect path against BASE, the G5 tripwires, and paired, order-alternated
      indirect/direct measurements on Mac, labelled with their load class (→M1.6).
- [ ] `scripts/cooked_draw_ios_sim.py`, derived from the T2 M2 probe, without its source patch. It creates and
      deletes a fresh simulator, then builds, cooks, installs and launches with the store in the app container. It
      checks the route state, takes `simctl` stills and a pan sequence, and runs a `--lifecycle` mode. It fails on
      errors and never skips (→M1.7–M1.10).
- [ ] Inspect the simulator stills and the pan sequence against the M1.9 and M1.10 inspection lists. Keep the list
      and the crops in the evidence.
- [ ] Update the G5 decision (meshlet section, capability inventory, open items), `maturity.json`, the evidence
      README and the T2 handoff. Close out scratch (→M1.11).

Stop rules:
- A failed parity arm whose difference shows the direct path drawing geometry the indirect cull rejected is an
  existing indirect cull defect. Report `BLOCKED` with the diff. Do not touch the indirect path, and do not loosen
  the comparison.
- A non-deterministic same-path rerun invalidates that arm. Find out why before you claim anything.
- If a `simctl` capture turns out to need an unlocked console or an OS grant, record that row's reason and continue
  the engine-state rows. The owner then decides whether to rescope the item as attended.

Checks: `./dev check` under `gpu` for code. This planning revision used `./dev check-docs`. Scratch goes in
`tmp/cooked-draw/`, under [the storage rules](../../ASSET-LIFECYCLE.md).

## Log

- 2026-11-09 Planned, at the owner's direction through the lead, after T2-CAPTURE M2 was blocked
  ([evidence](../../evidence/render/t2-capture-m2/README.md)). The design selects a direct full-range indexed draw,
  chosen from adapter downlevel flags in the capability record. It rejects CPU culling, compaction culling, delayed
  readback, per-meshlet direct draws and a simulator override, on cost and complexity grounds. T2-CAPTURE M2 now
  `needs` `COOKED-DRAW:M1`. The design names an adjacent gap for G3c: synthetic culled modes use indirect draws
  without selection. No implementation, and no agents launched. Independent design review is owed.
