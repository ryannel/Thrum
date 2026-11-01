# Cloud model comparison closure

1 November 2026. Verified the actual canonical text after integration, rather than relying on parent messages. No canonical edits, code, GPU jobs or commits.

## Artifact identity and scope

- `docs/design/modules/CLOUDS.md`: `67e92ad2a3944c9489ff50d70fd47bc6a3930e3b543c2775c69bc11ef589fb2f`.
- `docs/design/modules/CLOUD-CAPABILITIES.md`: `4ef919ceb068e1514fb86dd10b1e59544b0dd88c86a00d9e9dcce8e2ccae99be`.
- `docs/design/research/CLOUD-SYSTEM-SOURCES.md`: `75c5cb72f7901e3e1aa79a7e4f9630874fa8ef1f17740111decf01b7b5b9d55d`.
- `docs/design/research/CLOUD-SKY-SOURCES.md`: `d13774df623841844c61f438a66daa4f4e30c9d3e05fc18733c6fc1a3530f7f5`.

Read revised source/admission, world/coordinate, lighting/ambient and research sections and checked their integration with the earlier comparison. The capability ledger still distinguishes partial/unproven source/consumer evidence; no new family or full sky is promoted by these edits.

**Verdict:** CM01–CM09 are closed at the proposed-design level in this revision. No remaining P1 or P2 defect was found within this model/research review scope. This qualifies the declared operators and their coupling, not their implementation, visual output, complete cost or user acceptance. The first authored graph, thick/ice lighting and full weather remain unproven as the canonical text states.

## Finding dispositions verified in the canonical

| Finding | Actual resolution and verification |
| --- | --- |
| CM01 protected centre | Lines175–186 add independent spatial threshold D_i, nonnegative eta, positive p/e/duration and bounded exposure delay. Once D_i>=1+e_width, the density argument is <=-e_width even at m=1. Every cohort can terminate exactly. Zero-radius birth bypasses inversion; the text correctly limits continuity to integrated optical support. |
| CM02 excessive low-Sun source | Lines427–445 replace added orders/B_local with globally fixed nonnegative w_n b_n effective branches and ordinary cloud tau. The b prefactor plus normalized phases/albedo bounds makes the direct-source mixture passive under the reviewed assumptions. Ordinary extinction remains unchanged. Historical behaviour remains separate; new quality/energy gates remain unproven. |
| CM03 radial inverse under shear | Lines205–225 select forward launch/cohort integration with actual moving outlet trajectories, normalized sector/K3 deposition, Q_sigma in m²/s and a reference integral. Multiple launches add their actual emitted amount. A special inverse is confined to an independently verified restricted case. This removes the invalid general inverse rather than repairing it with an assumed radius. |
| CM04 wrong forcing coordinates | Lines243–249 explicitly evaluate P(F_base(chi,t),t). The world front is pulled back into layer material coordinates once. |
| CM05 within-layer shear | Lines236–249 select a height-dependent map and inverse with analytic/cooked admission; M is explicitly a reference-column depth. World-vertical depth is integrated across the different sheared columns, not assumed equal to M. |
| CM06 infinite Ci support / inconsistent modes | Lines274–287 select a compact normalized K3 continuous emitter. K2 line mode is separate and requires launch-time-to-arclength conversion. Degenerate conversion uses K3. Finite supports now match conservative bounds. |
| CM07 frozen mature layer | Lines261–267 require asynchronous local wet/dry episodes with compact sin² envelopes, persistent cell/episode identity and300–900s durations. These provide actual local amount/topology renewal rather than only translating C. Actual recipe/output quality remains a proof gate. |
| CM08 B thin-limit ambiguity | B_local is superseded. Lines439–442 correctly identify .82125 as the angular-integrated thin-source ratio, equal-directional ratio only for equal phases; liquid phases have direction-dependent gains/losses and require front/backlit thin-Cu witnesses. |
| CM09 stale research selection | The older source account is explicitly historical/local-profile lineage. CLOUD-SKY-SOURCES records the present synthesis, primary locators, actual inspection scope, prior model limitations and unproven output. It does not attribute the new control graph or passive mixture to a studio. |

## Normalization, endpoint and clamp checks

The selected compact kernels are correctly normalized. For K3,

`4pi * 315/(64pi) * integral_0^1 u²(1-u²)^3 du = (315/16)*(16/315)=1`.

For K2, substituting u=r²/w² gives `3 integral_0^1(1-u)^2 du=1`. The forward ring kernel's nonnegative sector weights integrate to1; therefore integrating normalized K3 over sectors still integrates to1. Increasing ring radius redistributes a fixed cohort amount rather than implicitly creating optical amount. The anisotropic implementation must interpret the stated determinant normalization as the usual unit-kernel formula `K_A(v)=K_unit(A^-1 v)/|det A|`; do not apply both a physical w^-3 factor and a physical-axis determinant twice. That is an implementation invariant of the declared normalized kernel, not a new alternative model.

Lines139–143 now explicitly require positive widths/width law, nonsingular supports, H_max/M_star>0, positive Z for M>0, Q_sigma>=0 and0<=loss<=1. The causal emission interval is `max(s_start,t-a_max)..min(t,s_stop)`, with nonnegative age, empty interval zero and exact terminal loss. Thus future cohorts cannot leak into formation, and a finite age window supports finite virtual-source lookup. The7200s initial maximum age is recorded with the virtual lookup. Kernel normalization alone does not constrain peak sigma; the document now rejects/recooks out-of-range recipes or changes the precision profile and explicitly forbids silent sigma clamping.

The source's max(0,...) column support rule, finite m support and H saturation are declared source operators, not hidden runtime clamps imposed to fit storage. At M→0, H scales as sqrt(M), peak M/H also tends to zero (given valid Z), so the thin layer has a continuous zero-support limit despite a potentially steep derivative. Exact optical integration/footprint error at such fronts remains a numerical witness, not a passed test.

## Ambient and global-source consequences

The finite ambient DAG is actually nonrecursive: ordinary light visibility → direct Sun-lit air/fog incident field A0 with full extinction → isotropic cloud ambient → final joint views/probes → surface illumination. A0 excludes cloud source and Sun disk, uses black ground and does not read the final clouded probe. This gives a defined lossy one-air/fog-to-cloud scattering approximation; ground bounce and cloud interreflection are omitted explicitly. It does not create a second weather authority or require recursively discovering an unbounded set of optical paths. Counts remain substantial and unmeasured; the source/light work caps do not certify frame-time admission. This closure review covers that declared first Sun profile, not a newly qualified Moon/other-source extension.

Moving from a finite view disk to sphere-wide cube-face virtual cells introduces a real lookup obligation: a current blocker may have been born in a different cell or across a face. Lines339–346 now explicitly include full birth-to-current trajectory/residual/support reach or backtraced queries, a finite active-age window, face-neighbour traversal and canonical-ID deduplication. The drifted-in and face-corner witnesses are present. Thus the former radius-only/global-domain omission has not simply moved into a cell-index omission. Conservative bounds and actual traversal remain unimplemented qualification.

No unintended source deletion, source duplication or endpoint normalization defect remains evident in these revised definitions. Bad content can still be authored, and a normalized kernel can still look like beads or a smooth plate; the isolated-family motion gates must reject those outcomes. Retain the current prepared-source/transport evidence while proving this candidate independently.
