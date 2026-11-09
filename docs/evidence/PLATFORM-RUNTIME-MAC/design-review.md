# PLATFORM-RUNTIME-MAC design review

9 November 2026. Reviewed revision `d0f8c09940651867afb9383be9d74133308e5701`, including the complete Mac plan migration diff and the new done-when file. No findings requiring design changes.

This is a design and acceptance-criteria review under `review-milestone`, not a new milestone verdict. No agents were launched. No runtime, recording or device qualification was performed, and no historical pass was independently re-awarded here. The done-when file and design remain unchanged.

## Commission and scope

The requirements come from the build plan's purpose, V1 exit and G3b row, the architecture brief's platform requirements, C02/C06/C07, and the Mac plan's design-review brief. The Mac commission is to qualify the existing bounded frame runtime in the cooked game: survive held-drawable device destruction in the same world/window, retain explicit running or paused authority policy, restore controls and composition, and observe slow camera motion and attributable presentation on the built-in ProMotion panel.

The design must preserve one world clock, platform surface/device ownership and renderer submission/history ownership. Display callbacks, queue calls, GPU completion, video timestamps and attributed display swaps must remain distinct. Native recording, reconstruction/governor work, sibling platform qualification and physical iPhone timing have separate owners. Numeric performance figures are observations under the owner's performance decision.

The Mac plan was the first module-specific document read; its review brief is embedded after its proposed design. This was a review of an existing qualification design and migration, not a blind technique-selection exercise. I read the requirements and shared governing documents before judging the JSON items, then consulted the historical evidence records. I had no builder discussion or expected verdict.

## Design assessment

The ownership model is coherent. The shared platform design and bounded-frame-host decision put tick admission in `ClockedWorld`/`WorldLane`, surface configuration/acquisition and device reconstruction in the Mac render actor, and frame slots/submission/history in `FrameHost`. The scoped code read supports those seams: `display_link.rs` only delivers target timestamps; `render_actor.rs` encodes before acquisition, checks observed loss around acquisition, abandons lost encoded work and publishes a prepared newer device; `recovery.rs` tracks device attempts separately from world lifecycle. The normal game edit handler applies camera changes through the host, and the shipped yaw control has the planned 0.05° step.

The recovery mechanism and proof distinguish an unchanged paused image from a working restored game. The arm-disabled diagnostic supplies an exact same-build image control, while running-authority events, queue-call auditing and composited post-replacement input response address different failure modes. The proposed Ground E image supplies composition context rather than an independent exact pixel oracle. The scope is observed injected destruction, not proof of every possible hardware loss race.

The cadence mechanism is a qualification hypothesis with an explicit refusal condition. A readable, app-attributed Game Performance swap stream joined to the full pan would answer the stated observation; a display-link callback, present call, ScreenCaptureKit timestamp or historical trace would not. The plan explicitly leaves cadence open if attribution, export or alignment fails. The three failed trace attempts do not become positive evidence or a numeric performance gate. No raw drawable bridge or replacement rendering backend is introduced.

## Done-when audit

| Item | Can a later reviewer judge it? | Convincing substitute it excludes |
| --- | --- | --- |
| M0.1 | Yes. Read named contracts/designs, follow owners and verify the build-plan link. | A second world clock, rival submission owner or an unsupported presentation claim. |
| M1.1 | Yes. Inspect source boundaries and same-build raw event/audit records for all three rows, device generations, PID/window, acknowledged policy and tick ordering. | Restarting the process, pausing running authority, or ignoring empty old-queue submissions. |
| M1.2 | Yes. Full paused RGBA comparison plus normal-speed transition and input-response inspection, with capture gaps disclosed. | A static restored screenshot, a partial image comparison or a present call standing in for visible interaction. |
| M2.1 | Yes. Matched manifests, full sweep and endpoint holds, normal input route and content/log comparison constrain the motion judgment. | A fine-looking still, partial/slow playback, mismatched builds or authored yaw steps mistaken for pacing. |
| M2.2 | Yes, if fresh readable attributed trace data exists; otherwise it cannot pass. Numeric Swap Time, process/display identity and correlated pan markers define the required evidence. | VSync, recording PTS, app submissions or an old trace presented as current displayed cadence. |

The JSON preserves the substantive pre-migration criteria. M0/M1 remain historical completed work; M2 remains in progress with its cadence item open. M2.1 explicitly limits its carried pass to `0d53d3aa` and requires later-build corroboration before extending it. The resume evidence specifies three-view offscreen parity followed by an untraced live pan; image parity alone cannot extend a motion verdict. A future cadence run must satisfy M2.2's matched-run and pan-attribution requirements rather than borrowing the old motion result for a different workload.

The items use `ask` and `inspect` rather than command checks. That is judgeable here because they require concrete source boundaries, raw logs/readbacks, complete recordings and a trace; the reviewer must answer those questions from the artifacts, not accept the evidence summaries. Historical records were read only to check migration scope and revision limits. Their recordings and raw scratch inputs were not re-reviewed.

## Limits and verdict

This review covers the Mac qualification design, its governing lifecycle boundaries and all five acceptance items. It does not approve the entire platform-family roadmap, revalidate the implementation, close the current-build corroboration work or establish displayed cadence. The runtime pause and parked trace-finalization investigation remain as recorded.

DESIGN OK
