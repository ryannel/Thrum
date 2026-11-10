# PLATFORM-RUNTIME-MAC design review

10 November 2026. Reviewed integrated revision `9d6c6f6fcdd0b7ad533cce994a01f9567ab33887` under the review-milestone design-review procedure. No blocking or advisory design findings.

## Previous round

The previous review, dated 9 November against `d0f8c09940651867afb9383be9d74133308e5701`, ended DESIGN OK and raised no blocking findings. There are therefore no outstanding blocking findings to resolve. This review checks the consolidated design and revised acceptance definitions, including console witnesses, current-path M2, windowed capture, bounded trace diagnosis and post-G3t M3.

## Requirements and scope

The commission comes from the build plan's purpose, V1 exit and G3b requirements, the architecture brief, the shared platform runtime design and C02/C06/C07/C12. The plan's embedded review brief adds the held-drawable recovery and observation boundaries. These require the same cooked game, world, window and controls to survive graphics loss, explicit running or paused authority policy, coherent slow input-driven camera motion, and attributable built-in-panel presentation observations.

The requirements imply one authoritative clock, platform-owned surface/device recovery and renderer-owned submission/history. A successful queue call, GPU completion, engine movie or display-link callback cannot alone establish displayed cadence. Browser/iOS qualification, G3t mechanisms, G4 reconstruction and physical-device timing retain their own owners. Numeric performance results are observations, while visible hitches require investigation.

I read these governing requirements before assessing the revised Mac design and all done-when items. The module has no separate brief file. Its review brief is embedded in its plan. This is a review of an existing qualification design, with exposure to its prior review and scoped implementation, rather than a blind technique-selection review.

## Design assessment

The design preserves the governing ownership boundaries. AppKit supplies display targets and window identity. The world clock/lane owns authority. The render actor owns late acquisition, surface epochs and serialized replacement. FrameHost owns GPU admission, slots, submission and submitted history. The scoped source read of `display_link.rs`, `render_actor.rs` and `recovery.rs` supports those seams, including loss checks around acquisition and explicit encoded-frame abandonment. The design confines the recovery claim to observed loss and the specified injection, with unchanged-world and same-build image controls.

The revised unattended proof is consistent with T2 and C12. Actual monotonic elapsed time drives the ordinary semantic input and renderer paths, while a paused world retains authority. Complete submitted source frames and joins prevent endpoint redraws or a fixed-step movie from standing in for runtime motion. Console observations accept either state and transitions. Denied attributed launcher/driver preflights establish the separately stated capture-independence proof.

The current harness still checks `locked_permission_unavailable_qualified` in `scripts/capture_mac_m2_ungranted.py`, and `scripts/capture_v1_runtime.py` still lacks the commissioned driver after-run permission preflight. These are explicitly identified implementation work in the design and plan. M2.1 is open and requires a fresh run after correction, so the design does not convert that unfinished work into an accepted capability. The earlier locked-run pass retains only its historical scope.

M2.3 commissions qualification of the existing renderer tap on the actual window. Its known-pattern correspondence, producer/submission binding, pending-readback loss, stale-generation refusal and disabled-path checks cover the important integration risks. The runbook explicitly says an offscreen pattern test alone cannot pass the live-window item. Capture retains T2's bounded resources and the existing executor.

M2.4 treats trace finalization as an unresolved tooling hypothesis with a refusal outcome. The changed hypothesis, two-attempt total allowance, finite recording/finalization/export limits and owned-process cleanup bound the investigation. The runbook conservatively charges the two matched traced arms against that allowance. A readable export still needs app/window attribution and a valid full-pan clock join. Neither an existing trace directory nor the historical swap stream can pass.

M2.2 uses capture OFF for its shipping cadence observation and matched ON arms for content and observer cost. AppKit controls, build/content/recipe matching, numeric swap times and calibrated attribution constrain the observation. M3 repeats the complete proof after APPLE-TILE M3 changes the assembly. Current-path M2 can proceed independently, and the later delivery dependency does not create a cycle.

## Acceptance-item audit

The prospective inspection list used for this design review was ownership and loss ordering, authority policy, full motion coverage and provenance, console/permission independence, real-window correspondence and lease lifetime, disabled observer work, bounded trace failure, swap attribution/timebase joins, and post-change requalification. These are design inspection questions. Runtime images and motion were not inspected in this review.

| Item | Judgeability and convincing substitutes excluded |
| --- | --- |
| M0.1 | Named contracts, owners and build-plan link permit a concrete boundary audit. It excludes rival clocks and unsupported physical-presentation claims. |
| M1.1 | Same-binary diagnostic rows, raw queue audit, PID/window identity, acknowledgements and tick ordering distinguish retained authority from restart or implicit pause. |
| M1.2 | Full RGBA equality plus normal-speed composited post-recovery response distinguishes intact recovery from a frozen image or a present-call log. |
| M2.1 | Full-resolution elapsed-time sources, holds, yaw/submission joins, denied preflights and explicit console policy are judgeable. Capture omissions refuse the interval. Static parity and historical passes cannot substitute. |
| M2.2 | Matched arms, fresh numeric app/window/display swaps and a calibrated full-interval join are judgeable when available. Ambiguous exports or anchors leave it open. Capture-on observations cannot qualify shipping cadence. |
| M2.3 | Source tracing, real-Metal checks and actual-window pattern/replacement inspection jointly establish producer correspondence, lease ownership, cancellation and the disabled direct path. Offscreen-only evidence cannot pass. |
| M2.4 | Hypothesis and durable attempt records, deadlines, fresh exports and attribution make both success and blocked outcomes observable. Tool completion alone cannot close cadence. |
| M3.1 | Incorporates every M2.1 proof on a fresh integrated post-G3t build with source/shader/content/recipe identity. Earlier output cannot qualify it. |
| M3.2 | Incorporates the full M2.2 observer protocol and revalidates M2.3/M2.4 on that build. Functional-window logs and earlier traces cannot substitute. |

## Limits and verdict

This review assesses design and proof sufficiency. It does not re-award historical milestone verdicts or establish current implementation correctness, motion quality, window correspondence or panel cadence. No runtime, GPU recording, permission change or trace attempt was performed. No agents were launched. Only this review file was changed.

DESIGN OK
