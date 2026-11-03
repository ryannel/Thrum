# Finite-material source design and Cu pilot review

3 November 2026. Proposed source replacement; **only the bounded liquid Cu pilot is commissioned**. The [canonical candidate](../../design/modules/CLOUD-AUTHORED-SOURCE.md) explains the complete point-field semantics and remaining family/cost gaps. [Model commission](model-commission.md) and [pilot commission](pilot-commission.md) retain the exact implementation instructions. Neither is a second active checkbox queue.

Independent design review rejected the permanent-master alternative: sustained height shear accumulates indefinitely, while clamping/rewinding violates wind or introduces resets. Finite-lived new-material elements under stable weather-cell IDs are coherent. A complete direct sparse-template query closes sampling semantics without inventing a required global adaptive field. It does not pass frame cost or supply missing family anatomy.

Review found and closed these consequential pilot issues before implementation:

| Finding | Disposition |
| --- | --- |
| Inheriting a parent's full material map carries strain through generations. | New condensate gets fresh rest, age and differential wind; inherit socket position and initial internal velocity after subtracting shared W once. Surviving parents never reset. |
| The first field equation omitted M although its extinction bound used M_min. | Use d − M(E_age + E_weather) consistently in local metres. |
| Penetration depth alone permits a tiny, unresolved child ball. | Use the full minimum-radius/intersection certificate and separately qualify visible attachment. |
| Keeping the whole child buried until its interior reaches 8 m is impossible for the chosen .04 template. | Burial applies during the tiny seed's amplitude ramp only. After A_seed=1, geometry may emerge; positive attachment remains, followed by the 8 m resolved-neck gate. |
| A sparse core atlas can lose support across an omitted neighboring core. | Admit every core intersecting the one-texel support dilation. Independent counts reproduce 2,464 / 2,927 cores. |
| Wet relaxation can prevent finite death and make population/strain bounds false. | Irreversible age floor reaches a certified endpoint. Weather may create replacements, never cancel age. Hidden maximum-dominated cohorts remain counted until zero. |
| Target scale, drying rate, control bandwidth and live-edit ranges were incomplete. | Pin b=4·target_scale·growth, the explicit persistent dry law, finite wind/front controls and the fixed 17³ lattice derived from Jacobian edge lengths. |
| Unconditional 80-second attachment conflicts with intentional destructive drying. | Guarantee wet establishment only; record adverse-weather cancellation/early breakup without calling it healthy renewal. |
| Interpolation lag was stated against requested time even during backlog. | Bound against committed source time; expose requested-clock backlog separately. |
| “Monotone by 360” did not determine parent survival through establishment. | Pin age exposure to endpoint·smoothstep(200,360,age). |
| Front loss was named without its growth/admission coupling. | Sample D at the regional admission centre and each cohort centre for the prescribed wet target, while local loss uses mapped material lattice positions. |

Final independent prebuild review found no remaining representation or birth-mechanism blocker for the liquid affine-wind pilot after these definitions. It did not accept implementation, appearance, general weather, Cb/anvil, the five-family system or full-sky cost. The [actual build queue](../../design/modules/CLOUD-MIGRATION.md) retains those gates.

The local callback/recorder failure review and geometry proof remain [separate evidence](../cloud-geometry-operators-2026-11-03/README.md). No previous numerical pass transfers automatically to the new combined source.

[The measured template counts](template-brick-counts.json) can be reproduced with [the retained script](count-template-bricks.py), NumPy/SciPy and the pinned compressed masters. They establish useful slot payload only; exact GPU atlas addressing, rounded allocation and performance are unproved.
