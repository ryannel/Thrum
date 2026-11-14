# SEA plan

The wind's waves, from glassy calm to gale: one field that every body of water reads.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) · [brief](BRIEF.md) ·
[module list](../ARCHITECTURE.md) · [the water system, `WATERS`](../WATERS/DESIGN.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines). The limits there hold for every milestone: Broadsides is
never changed, a file copied from it is listed in the provenance file, and another module's exact test or
baseline is never edited to make a change pass.

## Open for the human

- Assumption: after the owner's words of 14 November, "thinking about water as a whole ... built cohesively", this module is the wave field alone and ends at M2. Its old M3 to M10 (the surface, floating bodies, wake, foam, optics, controls and the close) moved to `WATERS`, generalised to every body of water. M1 and M2 are unchanged.
- Assumption: the owner's words of 14 November about clouds, "it should start from first principles", govern the sea too. The 13 November design, the brief and the port decision were judged as inputs.
- Assumption: "you can copy that code out and reuse as is if it's useful" is the owner's licence for what Broadsides' authors wrote, so it enters this repository on this repository's terms. Material that came into Broadsides from elsewhere keeps its own licence, and none that is GPL or share-alike is copied. Licensing is the owner's decision; builders work to this reading.
- Assumption: "bring that across when it makes sense" (13 November) means this plan decides when. Milestones start when their dependencies are Done, which can be before G7, setting aside the water decision's line that implementation follows G7.

## M1 The sea's waves are computed on the CPU alone, from glassy calm to gale, and agree with wave theory

This milestone creates `thrum-sea-model`, with no dependency on the renderer, and registers it in
`.config/architecture.toml`. Neither revision of 14 November changed its items. The interaction and foam
references are `WATERS`'.

## M2 The GPU computes the same sea as the CPU model at every frame's time, after a day of running, through pauses, hitches and a replaced device

This creates `thrum-sea` on `EFFECT-GRAPH`'s seam, which that module's M2 built:
`crates/thrum-render/src/contribution.rs` holds the trait and `examples/v1-game/src/effect` a working effect to
read. Only `simulate`-stage passes are built here, all of them every-frame passes that keep nothing between
frames; nothing is drawn. `WATERS` declares these passes inside its own contribution, so keep their declaration
callable from another crate. This milestone also changes `thrum-sea-model`, which M1 built: cascade bins take
whole-cycle loop frequencies (M2.9), and the model gives each cascade's rate of change. Broadsides'
`ocean_evolve.wesl` and `ocean_fft.wesl` already do the loop phase and the 256-point transform
([what Broadsides offers](DESIGN.md#what-broadsides-offers)). M2.2's bound is measured first and then asserted.
The recorded Chrome limits are in `docs/evidence/EFFECT-GRAPH/M1/`.
