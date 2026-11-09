# COOKED-DRAW design review

9 November 2026. Reviewed integrated tip `58c3414ee8f86cc06fe0dd5867cf07c1a9fc7372` using
`orchestrate:review-milestone` and `thrum-engineering`. This verdict covers the current brief, complete design,
plan and twelve done-when definitions, including the console-witness correction and accepted iOS extent dependency.

**DESIGN OK. Zero blocking findings and zero advisory findings.**

## Requirements derived from the brief

Before reading the design, I read the brief and derived this inspection list.

- Cooked scenes must draw without indirect execution. Selection must use actual adapter facts in the capability
  registry, handle unknown facts safely and repeat on device replacement.
- Supported Mac and Chrome routes must retain indirect selection, byte-identical before/after output and unchanged
  per-frame work.
- The simulator must draw complete, textured, lit and shadowed cooked content. Its proof establishes function.
- C04 selection authority, the contribution seam, C06 lifetime, C07 history and C12 evidence must retain their meaning.
  Interface changes must identify producers, consumers and agreement owners.
- The cost account must precede construction, cover repeated work and resources, and use measurements and existing
  tripwires without inventing a budget.
- The module must deliver the prerequisite for T2-CAPTURE M2 and identify its handoff.

I then compared the design and plan against these requirements and assessed every acceptance definition for
judgeability and plausible false passes. I read the previous review afterward. This session also inspected scoped
existing source, so this is a review of the integrated design and its seams rather than a blind technique study.

## Design assessment

The mechanism satisfies the brief. Adapter-backed capture supplies downlevel facts through `DeviceCapabilities`
and `SceneSurface`; unknown facts select direct. Rebuild reselects for the current generation. This closes the public
adapter-less `FrameHost::new` route as well as the simulator route. The Mac diagnostic mask removes a recorded
capability through explicit opt-in. Platform and adapter-name selection are expressly forbidden.

One whole-range indexed draw per placement fits the cooked meshlet-ordered index representation. Base vertex zero
and one instance avoid the missing indirect and base-vertex capabilities. Shared pipelines, buffers, materials,
placement records, Sun casters and volume preserve scene composition. Removing cull resources and dispatches is
compatible with the contribution's optional compute phase. History maintenance and tentative/submitted publication
remain common to both paths. M1.1 and M1.2 require stateful tests and resource/command inspection that can reject a
path which merely reports Direct or produces a correct still while skipping history or recovery.

The cost account in `COOKED-DRAW.md` lines 235–268 covers representative scene scale, per-placement writes,
meshlet dispatches, command expansion, index work, argument/counter/readback storage and fallback work. The fallback
avoids CPU meshlet traversal and delayed visibility feedback. Probe reference resources and commands are separately
accounted for and excluded from ordinary frames and timings. M1.6 combines the BASE command audit, existing G5
tripwires and paired, order-alternated measurements with load labels. No simulator or physical-device speed
qualification follows from this plan.

The contracts table names the registry/render, G5, V1 game and iOS shell interfaces and their consumers. Agreement
obligations remain explicit in the plan. The adjacent synthetic-renderer indirect gap is identified and assigned to
G3c. It does not undermine the scoped cooked fallback. PLATFORM-RUNTIME-IOS M7 is marked done in its own record;
COOKED-DRAW retains that dependency and independently requires cooked rotation and visual proof. T2-CAPTURE M2
retains `needs: ["COOKED-DRAW:M1"]` and owns subsequent submitted-frame temporal qualification.

## Parity and scene correctness

M1.3 distinguishes production direct D, diagnostic full-range indirect reference R and selected production meshlet
indirect I. Exact D/R equality at matching partitions tests direct command execution. Independent cooked-range and
placement validation, command/submission identities, exact repeats, a D-only missing-mesh control and restored
equality make a self-comparison or shared faulty command source detectable.

The I/D comparison separately requires complete geometry, correct placement, materials, lighting, shadows and volume.
Full images, unthresholded differences and native-resolution inspection cover terrain edges, entering geometry,
silhouettes and diagnostic meshes. An unclassified discrepancy or visible defect blocks acceptance. Historical pixel
counts and the derivative/ordering hypotheses supply no waiver. M1.4/M1.5 preserve the brief's exact before/after
production indirect comparison, and M1.6 preserves its work. The design supplies sufficient acceptance mechanisms
without requiring a causal claim about the historical Metal discrepancy.

## Console-witness correction

The revised design at lines 203–209 and plan at lines 102–120 define a concrete correction across ordinary,
lifecycle, rotation and visual modes. Locked, unlocked, changing and unavailable console observations remain recorded
metadata. Manual-lock waits and lock-state refusals must be removed. Regression controls must exercise these cases.

M1.7 and M1.8 independently require before/after denied read-only Screen Recording preflights under the identified
launcher/harness permission identity. Missing, failed or granted preflights leave the independence proof incomplete.
The explicit rejection of an unrelated denied helper addresses a plausible false pass. The repository's
`capture_mac_m2_ungranted.py` and `mac_m2_capture_launcher.swift` provide an existing launcher-attribution pattern
for the builder to adapt. Its old lock admission requires the correction already specified here.

Rotation may retain a separately recorded attended gesture requirement. M1.9 cannot transfer that requirement to
M1.7/M1.8. Simulator tools, content, validation, advancing counters, lifecycle and cleanup remain required. This
preserves the functional proof while allowing normal console use.

The current `scripts/cooked_draw_ios_sim.py` still contains `wait_for_console_lock` and its admission calls. The plan
explicitly identifies this as builder work and requires fresh standalone M1.7/M1.8 passes before implementation
review. Historical locked runs cannot satisfy the revised proof on their own. This design verdict grants no
milestone acceptance.

## Acceptance definition assessment

| Item | Judgeability and false-pass coverage |
| --- | --- |
| M1.1 | Adapter-backed, masked and unknown records, exact public-host attachment, actual resources/commands and generation reselection distinguish capability selection from a platform switch or invented support. |
| M1.2 | Both suite arms assert their paths and counts. Discard, capture without history, recovery, remote feeds, material promotion and casters cover state beyond a still image. Resource inspection enforces the direct path's omissions. |
| M1.3 | All named views, exact D/R equality, independently validated arguments, three-arm repeats, a D-only defect and restoration test the oracle. I/D inspection rejects scene and shading defects independently. |
| M1.4 | Pinned pre-implementation BASE, one store, original views, unchanged visible counts and repeat controls make exact Mac regression proof judgeable. Review must trace distinct builds and actual submitted outputs. |
| M1.5 | Real hardware WebGPU, one browser session, build identities, selected-path witnesses and repeats permit exact Chrome comparison and rejection of cached/substituted output. |
| M1.6 | Command/resource/write audit complements pixel equality. Adapter-backed bench selection, same-load history and labelled paired measurements address accidental path changes and misleading cost comparisons. |
| M1.7 | A fresh owned simulator, actual drawn generation, advancing presented/cooked counters and error refusal establish function. Console controls and attributed denied preflights make the revised independence claim testable. |
| M1.8 | Standalone witnesses, held background counters, resumed drawing and generation-specific publication cover lifecycle and replacement. It owes its own revised preflights. |
| M1.9 | Current extents and surface epochs, stable process/window/lease/device identities and advancing cooked frames cover both rotations. Any attended reason is local to this row. |
| M1.10 | Named views and native-size crops independently inspect complete geometry, texture channels, materials, facing, Sun/ambient light, shadows, volume, held-view stability and rotated aspect. |
| M1.11 | Sample times, holds, entering geometry and maximum gaps bound the continuity claim. The explicit exclusion of unsampled frames prevents sampled imagery from certifying submitted temporal output. |
| M1.12 | Decision, capability inventory, maturity and evidence reconciliation plus scratch closeout retain separate implementation, correctness, visual and device status and an explicit T2 handoff. |

## Review scope and validation

Read both calibration sets, the engine review/design/performance guidance, C04/C06/C07/C12, the hardware-native
decision, dependency records and the scoped capability, contribution, draw/history and launcher source described
above. No fresh GPU, simulator, visual or motion qualification was performed. Those proofs belong to implementation
review. Historical evidence and the previous review were treated as context, without adopting their runtime verdicts.
No other agents were launched. Only this review file was changed.

`./dev check-docs` passed with `CARGO_TARGET_DIR=/private/tmp/thrum-cooked-draw-design-review`, including local links,
module records and current design hashes. Existing source-size notes were informational. The acceptance-file audit
confirmed twelve unique items, each with a proof. `git diff --check` passed.

DESIGN OK
