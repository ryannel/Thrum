# Wider water design review — 1 November 2026

**Claim:** independently challenged analytical mechanism/integration selection for a bounded wider water profile.
**Not claimed:** full water-family completion, executable bindings, correctness/stability, visual acceptance,
measured capacity, device performance or a commissioned runtime. The smaller initial C4 baseline is unchanged.

Owning documents: [commission](../../design/modules/WATER.md), [dynamics](../../design/modules/WATER-DYNAMICS.md),
[appearance](../../design/modules/WATER-APPEARANCE.md), [optics](../../design/modules/WATER-OPTICS.md) and
[integration/proof](../../design/modules/WATER-INTEGRATION.md). The [source record](../../design/research/WATER-SOURCES.md)
distinguishes primary full-text/conference inspection from abstracts and historical lineage.

## Review independence and scope

Three fresh, bounded Sol High commissions independently read Thrum's contracts and the water brief, then researched
dynamics, secondary water and optics respectively. They had no previous Broadsides implementation/research in their
context. The coordinator retained historical exposure, which is not claimed as blind independence. Each study
inspected primary papers and substantive conference material, supplied mechanisms and actual limits, then read all
four combined drafts and cross-checked other contributors' additions. The coordinator derived the thin storage
reduction and optional fluctuation integration; other readers checked their equations and counterexamples.

The final resolution rereads found no remaining structural blocker **within the revised bounded scope**, after the
datum-only force-bound precision below. This is analytical review, not a source-code, recording, native motion or
device review. Agreement does not validate the authored frozen-stage adaptation, coefficients or hardware costs.

## Consequential findings and dispositions

| Finding / counterexample | Governing correction and remaining limit |
| --- | --- |
| Correcting a hull's sampled surface and feeding it back into its own pressure generator creates an unintended integral controller | Independent reference immersion produces pressure; last-applied particular solution is removed only for load sampling. Transient self-load bias remains explicit and must be bounded |
| Global reference inventory changes can instantly load a remote hull despite finite-speed waves | Explicit quasi-static nonlocal approximation. Isolate datum-only change at fixed pose/velocity/fluid; use maximum section over the affected interval for buoyancy and counterfactual P3 loads for full force/torque. Check previous and admission datum tolerances; no exact causal/reciprocal claim |
| Native nonlinear gradient filtering cannot support a proof merely labelled linear L | Freeze physical-depth coefficients, masks, ghosts and pass schedule; evaluate pressure-inclusive potential consistently. Exact rest theorem is limited to fully wet support; native dry shoreline extension/stability/leakage remains unqualified |
| A sine-only pressure kick misses static equilibrium | Retain shifted-oscillator displacement/velocity and common split; apply high-pass forcing once |
| Applying momentum directly to stored bulk discharge also changes residual-height advection flux | Scatter onto total continuity flux with native face dual areas, then use the explicit frozen-depth mapping to bulk state. Preserve source first moments or record unresolved torque |
| Continuous jet cohorts treated as discrete impacts create artificial tick-frequency rings | Integrate disjoint release intervals through the first-arrival map. Steady arrival gives mean traction; isolated pulse is exclusive. Optional zero-mean fluctuation has a dimensional envelope, arrived-energy store, full work cross term and validated mean-only fallback |
| Time-varying jet thickness Q/velocity can disagree with actual in-flight volume | Use release-coordinate Jacobian Q_release/|partial_a x|, detect singular/folded support; stationary Q/velocity is only a special case |
| An inflow pressure formula ignored outgoing physical spray and receiver motion | Cache incoming-minus-spray-minus-credited-carrier momentum; wave/bed reaction partitions that residual. Spray, wave, gas/interface work share one arrival allocation |
| First-hit search used body motion that had not been produced | Common provisional body/fluid trajectories, earliest-event rollback/recompute of both, bounded candidates and atomic world commit. Provider-local success is insufficient |
| Bubble births occurred inside impact supports with no defined local carrier; newborns could receive a whole step of age | Explicit post-deposit column velocity with inlet-flux boundary limit, direct admitted injection depth, swept dry/solid/surface rules, bounded integration and actual lived birth intervals. No resolved downward entrainment/plume claim |
| Foam conversion used total gas instead of allocated surface gas | Partition gas volume first, then area from surface gas/size. Venting/decay is not a liquid sink or second bubble birth |
| Ideal propulsor wash power was treated as dissipated foam power | Actual equal-opposite flow reaction, no volume source; ventilation-gated appearance proxy or separately partitioned real work. Signed external-drive residual is explicit; no ideal motor conservation claim |
| Positive volume alone does not prove discrete thin-flow energy decay | Equal-head, donor and closing-sill caps in deterministic edge sweeps give the pairwise energy argument. Horizontal metric, orthogonal dual and gentle-slope regime are required |
| Even exact volume/energy/hypsometry can deposit water at a low corner across an intervening dry triangle | Restrict selected thin exchange to preconnected wet receiving support. Advancing dry-slope runoff is a **structural WAT-06 gap**, not a passed experience or only a calibration issue |
| Bare breaking charts lacked clear-transmission thickness/occupancy and safe future contact support | Watertight oriented optical shell or explicit reflection/foam-only fallback. Noncontact physical scope plus lifetime reachable hull/query/camera exclusion; no current-frame hide |
| Screen-space roots and missing ray coverage were mistaken for first hits or darkness | Complete regional visibility product, purpose masks with overlapping membership, validation/fallback and explicit incomplete/ambiguous status. No environment substitute for unknown bed |
| Overlapping fall/curl/base surfaces could each emit full Sun caustic flux | Emit only at first light interface; transport the already-split beam through ordered later events. One focused/mean light partition, no duplicate Sun/attenuation; view and light overflow explicit |
| Thin-sheet removed flux was all labelled absorption | Absorption/out-scattering separated. Initial collapsed tier is absorption-only; explicit lossless grazing limit avoids 0/0 |
| Per-view preparation and four full-screen interface layers can exceed shared memory/traffic budgets | Shared world products, bounded leases/reservations and staged peaks; classified/tiled layers and explicit overflow. Operation/byte estimates and ranked cue-loss reductions are recorded, not advertised as timings |

## What remains unfinished

The wider-family D2 B/C checkbox remains open. In particular: advancing dry-slope runoff, traversable heterogeneous
seams, overlapping hull pressure supports, richer sea-spectrum/spatial control and open boundaries, moving domains,
thin-wall wave exclusion, rigid no-penetration, contact-qualified overturning and dense predictive entrainment.
Physical breaker-loss/coupling beyond the selected presentation proxy is not supplied by a curl mesh. These require
real mechanisms before their dependent experiences can be claimed.

For the selected profile, numerical dispersion/phase, frozen-stage positivity/stability and shore leakage, pressure
shape/transient/nonlocal errors, population calibration, optical reconstruction, ordinary motion and actual target
CPU/GPU/memory remain unrun. The next useful proof tests the least-established pressure/body coupling in a bounded
shared domain, then connects the real finite fall and appearance/optical consumers; it does not build another unrelated
showcase lake or promote historical output. Runtime work waits for its foundation dependencies and commission.

## Repository verification

`./dev check-docs` passed after integration: 147 Markdown files, 976 local links, 30 module records, crate policy
and current design hashes. No runtime tests, GPU jobs, performance claims, dependency additions or remote publication
are part of this change. Existing foundation work is preserved. The final post-polish check is recorded at handoff.
