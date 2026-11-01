# Water research intake for Thrum

1 November 2026. Supports the [water commission](../modules/WATER.md); not an algorithm-selection verdict.
The existing [baseline specification](../PLAY-ENVIRONMENT-SPEC.md) owns its Airy/shallow-water derivations and
initial operators. This intake refreshes consequential directions from the prior research without importing its
implementation or treating a bibliography as complete design. No source code or recording was executed/watched.

## Material inspected for this alignment

| Primary source / date | Actual inspection and useful locator | Consequence and limit for Thrum |
| --- | --- | --- |
| Stempfle and Mantler, [Water Simulation & Rendering in Enshrouded, GPC 2025](https://static.graphicsprogrammingconference.com/public/2025/talks/water-simulation-rendering-in-enshrouded/Mantler-Koenen-water-simulation-rendering-in-enshrouded.pdf) | Public PDF downloaded and extracted after browser size failure; physical PDF pp9–10, 17–19, 29–34, 36–42, 45–46 inspected, not a watched talk | Separate simulation, derived surface and optical passes. Their local ripple field, gradient-derived visual flow and precomputed caustics have explicit roles. They do not supply physical current, a queryable general hull wake or exact layered transport for Thrum. |
| Fournier et al., [Dynamic Wave Trains, CGF, May 2026](https://onlinelibrary.wiley.com/doi/full/10.1111/cgf.70495) | Publisher full text, abstract and §§10.2–12, particularly future-work limits | Retain as a current spatial synthesis/filtering candidate. Dynamic obstacles and current/wave advection are not solved by its appearance control; do not select it as the interaction producer by title alone. |
| Bao et al., [Pressure distribution shapes and Kelvin wakes, Ocean Engineering, 2026](https://dcwan.sjtu.edu.cn/userfiles/1-s2_0-S0029801826018147-main.pdf) | Author-hosted paper, §§2.1–2.2, equations10–12 and modelling discussion | Economical hull forcing needs justified spatial support, draft/displacement and resolution. Pressure patches approximate the hull; the paper omits dynamic pressure correction. Its HOS implementation is not selected or ported. |
| Mayer, Assarsson and Sintorn, [Ultrafast Screen-Space Refractions and Caustics via Newton's Method, JCGT 15(1), 2026](https://jcgt.org/published/0015/01/03/paper-lowres.pdf) | Primary PDF §§2.1–2.2, 3.4–4; convergence/failure and missing-geometry passages | Candidate for bounded visible-buffer intersections. Convergence cannot recover absent/occluded geometry. Missing coverage, real shadow and failed rays need different treatment; no published timing transfers to wgpu or mobile. |

The inspection establishes these claims only. It is not a fresh full read of every earlier paper, private engine source
audit, device measurement or assurance that these methods form a compatible combined runtime.

## Retained lineage to re-evaluate at the selecting decision

- [Horizon Forbidden West water, SIGGRAPH Advances 2022](https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Water-Malan.pdf):
  useful prepared deformation/appearance precedent; reopen substantive speaker notes for coast/river surface selection.
- [Jeschke and Wojtan, generalized heightfields, SIGGRAPH 2023](https://research-explorer.ista.ac.at/download/14240/14725/2023_ACMToG_Jeschke.pdf):
  compare a complete depth/dispersion/current design, not just its displayed waves; not re-inspected or selected here.
- [Interactive Water Simulation in Atlas, GDC 2019](https://gpuopen.com/gdc-presentations/2019/gdc-2019-agtd6-interactive-water-simulation-in-atlas.pdf):
  retained compression/foam production lineage. Reopen the relevant slides when choosing causal whitewater; optical
  birth proxies do not establish predictive entrainment or a physical reaction solver.

## Decision-driven refresh still required

| Workstream | Decisions that must receive current paper and substantive conference coverage |
| --- | --- |
| Coastal and interaction waves | Dispersion, current, bed/boundaries, hull forcing/support, reaction/self-field and CPU query feasibility |
| Inland and thin water | Wet/dry/storage, narrow channels, finite falls/landings, connected pools, puddle runoff and terrain reconstruction |
| Secondary water | Breaking/contact/propulsion production, foam transport/evolution, bubble rise/transfer and spray/medium roles |
| Optics and reconstruction | Surface/medium properties, reflection/refraction visibility, layered exceptions, glint/normal filtering, caustic light ownership and prior state |
| Production cost and authoring | Prepared content, source/cooked products, pass/data movement, query latency, cross-platform limits and ranked reductions |

Retain established mathematical lineage where it remains appropriate, then inspect recent competing methods and
successors for each consequential selection. Classify each adopted relationship as published, derived or authored.
Record inaccessible material and its effect; listings and abstracts do not close a design gap. This starts the
Thrum-specific intake rather than declaring the previous water literature survey universally transferred.
