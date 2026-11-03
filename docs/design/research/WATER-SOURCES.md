# Water research intake for Thrum

1 November 2026. Supports the [water commission](../modules/WATER.md). The initial alignment intake is retained
below with its original inspection scope; the later B/C refresh records the wider selections and limits.
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

## Decision refresh for D2-water B/C

The following selections and comparisons extend the alignment intake. The three bounded Sol High studies read
primary material and independently compared dynamics, populations and optics, then cross-reviewed the combined
design. No conference recording was watched; conference evidence below is the substantive public speaker material.
Inspection does not establish a Rust implementation, private engine internals or transferred timings.

| Decision / primary material | Inspection and recency | What it changes |
| --- | --- | --- |
| [Jeschke/Wojtan generalized heightfields, SIGGRAPH 2023](https://research-explorer.ista.ac.at/download/14240/14725/2023_ACMToG_Jeschke.pdf) and [author errata](https://visualcomputing.ist.ac.at/publications/2023/GSWSDSW/) | Dynamics study read full operators/appendices and author corrections, including equations 13/17/19–21/27 | Select complete bulk/dispersion/transport together. Published boat point-forcing/snapping is not a rigid-body coupling law. Thrum's frozen source split, pressure coupling and bounded CPU admission are explicit adaptations |
| [Yao et al., static versus dynamic ship pressure, JFM 2024](https://www.cambridge.org/core/journals/journal-of-fluid-mechanics/article/influence-of-static-versus-dynamic-pressure-distribution-strategies-for-modelling-nonlinear-waves-generated-by-ships/61331ACF6FD86F38DD4AFD291462257F), alongside Bao2026 above | Dynamics study inspected §§2,5–8 and compared pressure-volume/shape behavior | Static hull pressure is a calibrated surrogate with motion-dependent bias. Select geometry/support/error bounds; do not claim the dynamic HOS correction or rigid exclusion is implemented |
| [Dynamic Wave Trains, 2026](https://onlinelibrary.wiley.com/doi/full/10.1111/cgf.70495) and [wave-particle dispersive simulation, 2025](https://arxiv.org/html/2511.02852v1) | Fresh dynamics/optics study inspected relevant synthesis, temporal/filtering and current/interaction limitations, including DWT§8 and wave-particle §§4.2–4.3 | Current appearance synthesis remains useful but cannot replace finite storage, a force-query field or moving-source/current transport. Do not choose a new representation merely because it is newer |
| [Enshrouded, GPC2025 speaker slides](https://static.graphicsprogrammingconference.com/public/2025/slides/water-simulation-and-rendering-in-enshrouded/Mantler-Koenen-water-simulation-rendering-in-enshrouded.pdf) | Alignment full-text inspection above; secondary study rechecked flow-map, foam/VFX and optics roles | Gradient/SDF-derived visual flow is not physical discharge. Reuse the separation of simulation, derived surface and effects, not invented flow inputs |
| [Horizon Forbidden West, SIGGRAPH2022 speaker notes](https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Water-Malan.pdf) | Secondary study read deformation/surface/flow/foam sections, PDF pp7–15,24–29,40–41 | Prepared front-swept shape/material maps are a credible production role. Select noncontact chart animation with physical source/front ownership, exact attachment and closed optical-shell requirements; no claim that a curl mesh is a flow solver |
| [Wretborn et al., whitewater, SIGGRAPH2022](https://alexey.stomakhin.com/research/siggraph2022_whitewater.pdf) | Secondary study read rise equation 5, interface/birth relations§5.2 and transport/foam§§6.4–6.6 | Select dilute rise and population bookkeeping, not the full cinematic/neighbour solver. Surface area, gas volume and actual spray liquid require different state and transfer units |
| [Immersed bubbles, CGF2025](https://onlinelibrary.wiley.com/doi/10.1111/cgf.70033), [PF-FLIP2025](https://www.animation.rwth-aachen.de/publication/0594/), [HOME-FREE2025](https://haoxiang-wang.com/assets/WWQD25.pdf) | Secondary study inspected immersed-bubble full HTML §§3–5, PF-FLIP author method/workload, HOME-FREE full paper | Current research requires resolved 3D mixture/flow or offline-scale computation. It does not license deriving vorticity/turbulence from unavailable heightfield data. Keep predictive flow optional rather than faking its inputs |
| [Newton refractions/caustics, JCGT2026](https://jcgt.org/published/0015/01/03/paper-lowres.pdf) | Optics study read full method and failure discussion§§2–3.5, beyond alignment excerpts | Select screen-space acceleration with regional complete-visibility validation/fallback. Successful roots do not recover offscreen or occluded geometry. Software-BVH work/traffic needs its own device proof |
| [Bitsch et al., subgrid shallow water, 2026 preprint v2](https://arxiv.org/html/2601.01895v2) | Coordinator read §§3–4, especially§4.2.2; version dated 2 October 2026 | Explicit unresolved-barrier and steep-slope limitations prevent using the newest subgrid storage as a universal puddle fix. This alternative is not selected; volume/energy proofs do not prove connected dry-slope runoff |
| [Lubrication/contact-line lineage, Physics of Fluids2007](https://www.princeton.edu/~stonelab/Publications/pdfs/From%20Howard/PhysicsFluids/SnoeijerLeGrandLimatStoneEggersPhysFluids07.pdf) | Coordinator inspected thin-flow/contact-line regime and compared the current subgrid alternative above | Thin connected wet-edge exchange can use a gravity/viscosity reduction on an admitted orthogonal mesh; capillary pinning, inertial runoff and dry-path film transport remain distinct requirements. The discrete edge energy limiter is Thrum-derived |
| [MIT actuator-disk momentum theory](https://web.mit.edu/16.unified/www/SPRING/propulsion/notes/node86.html) | Coordinator checked§11.7.3; secondary study independently rederived momentum/power | Propulsor reaction changes actual flow; swept throughput creates no water. Ideal wash kinetic power is not automatically dissipated foam energy. Non-reversing disk/ventilation limits and external-drive residual are explicit |

The sources change individual decisions, not only the bibliography. Consequential derived additions include
pressure-reference separation, frozen-filter rest scope, total-flux staggered impulse mapping, release-coordinate
transit thickness, continuous-arrival traction, discrete wet-edge energy caps, one caustic light partition and
complete visibility/overflow semantics. These additions need independent numerical and experience proof.

This is a decision-driven 2025/2026 refresh across dynamics, thin water, secondary water, reconstruction/optics and
cost. It is not exhaustive literature coverage. Wider spectra, dry-slope film transport, heterogeneous crossings and
contact-qualified overturning still need selecting studies before the full family design can be called complete.
