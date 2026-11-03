# Water alignment review — 1 November 2026

**Claim:** a D2 water-family commission is aligned to Thrum's selected contracts and build sequence.
**Not claimed:** a new solver selection, build-ready full water design, runtime implementation, visual acceptance,
device performance or repository-wide readiness.

## Reviewed basis and scope

The coordinator read the Thrum README/AGENTS, build plan, C4 contracts, baseline R3 and budgets, play/environment
specification, environment family, D2/construction sequence, composition/developer workflow, performance policy and
E1.water maturity entry. The old Broadsides lake checkpoint supplied historical lessons, with prior exposure disclosed.
Thrum is a separate repository; no historical code/assets or external working-tree links are part of its build.
The prior research is a source intake, not automatically an engine decision.

A fresh Sol High reviewer independently derived capability/dependency coverage from the user brief and Thrum's
plans before seeing the coordinator's new water document. It read no Broadsides water implementation or research.
Its recommendations were design/coverage findings, not independent numerical or source-code validation. The
coordinator performed the fresh primary-source checks recorded in [WATER-SOURCES](../../design/research/WATER-SOURCES.md).

## Findings and dispositions

| Finding | Resolution in the [commission](../../design/modules/WATER.md) |
| --- | --- |
| R3 intentionally covers prescribed linear waves, finite shallow storage, one-way buoyancy, cosmetic wakes and decorative falls; the original water ambitions are broader | Separate admitted baseline from required wider coverage. Keep selected bounds and unsupported combinations unchanged; cosmetic behavior cannot qualify interactive waves or finite transfer |
| Cause/state/effect producers are missing for richer wakes, breaking and whitewater | WAT-02–08 require explicit generation, persistent state, reaction/transfer and source-off behavior before runtime work |
| Model topology and transitions cannot follow renderer meshes, stream cells or peer interest | Preserve C09 unique authority and explicit edges; require full finite-hull and receiver support for any newly advertised physical transition |
| One surface/clock still requires complete world publication, not only a successful fluid texture swap | Work cases require staged body, fluid, source, history and time consistency under F1; asynchronous failure/context behavior remains detailed work |
| Water can leak renderer/network requirements into headless authority | Preserve CPU/headless physical queries and separate save/render/replica products; client visual tiers and missing data cannot alter force or flux |
| Correct numeric components can assemble into the wrong visual/temporal result | Require ordinary motion and shared optics/receiver visibility as separate gates from invariants and still captures |
| Water can consume an entire game budget through repeated work or staging copies | Add frequency/byte/critical-path accounting, render-pass multipliers, query waits, leased snapshot peaks and ranked reduction hypotheses at selection time |
| A water-first implementation would displace active foundation work | D2 planning proceeds in parallel; R0 remains water-free, no speculative crate/backend or dependency is added |

## Historical findings retained without inherited proof

Broadsides checkpoint `2f84d07df`, with field `1840d7b9b`, actors `d820ebe60` and scene `79d08a6e2`, is historical
provenance. Its scoped tests and manual browser smoke do not qualify a Rust implementation or the intended full lake.
Its ordinary wake/caustic appearance and sustained runtime remained unaccepted; its proposed current extension was
not implemented. This is not code reconciliation or a claim that those numerical choices should survive.

- Correct equations plus arbitrary pressure footprint gave the wrong spatial response. Source support and geometry
  deserve the same scrutiny as the propagation operator; a quartic footprint is not a universal selected solution.
- A displaced surface did not guarantee readable water under an inadequate radiance environment. Shared light/material,
  reconstruction and actual motion need their own experience proof; amplifying a ring to fix a still is not validation.
- Incident-light occlusion, underwater receiving geometry and eye-ray visibility are distinct products. Missing ray/atlas
  coverage is not zero illumination. Those are generic V3 visibility obligations, not permission to transplant a shader.
- Buoyancy, contact, wave reaction, current response and propulsor aeration have different closures but common geometry,
  timing and relative-flow facts. A producer restricted to an object label can break that cohesion.
- Source-off must retain flight, waves and whitewater tails. A route crossing an older path is not proof that a second
  body encountered or reacted to the actual wavefront. A full-scene query/visual workload must reject those substitutes.
- Manual stepping and compact kernel timings did not certify ordinary frame pacing. Repeated work, synchronization,
  staging, warmup, tails, allocations and simultaneous views belong in the integrated cost account.

## Validation and remaining work

The independent follow-up found no material scope or contract-alignment defects. It did not independently verify
the new publications or historical checkpoint assertions. `./dev check-docs` passed: 136 Markdown files, 919 local
links, 30 module records, crate policy and current design digests. No runtime tests, GPU jobs or platform measurements
were run for this planning-only change. Existing E1.water maturity is preserved. WAT-02–09 contain explicit wider
mechanism decisions; scheduling a future probe does not close them. Return shared-boundary changes to the relevant
engine owners before dependent construction. The next commissioned work is analytical D2-water B/C selection,
not another isolated polished effect or automatic promotion of the old lake.
