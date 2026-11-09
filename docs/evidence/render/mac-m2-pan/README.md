# Mac M2 pan on the built-in panel

## Current disposition — 9 November unattended-evidence re-plan

This page retains historical observations and failed attempts, not a pass for a new build or the new capture path.
[Mac M2](../../../design/modules/PLATFORM-RUNTIME-MAC-PLAN.md) now depends on
[T2-CAPTURE M0](../../../design/modules/T2-CAPTURE-PLAN.md). Engine-owned submitted-image capture will supply
fresh elapsed-time runtime motion without Screen Recording access or an unlocked console once implemented and
proved. Three-view parity and a fixed-step movie cannot supply that live-runtime proof. The previous motion verdict
stays historical; changed M2.1 is open for fresh evidence.

M2.2 remains open for fresh independently attributable built-in-panel swaps. It is **attended setup** because it needs
an unlocked active built-in display and any Instruments authorization. The agent can inject the normal AppKit pan
and review output without the owner playing. Engine capture removes the screen-recorder grant prerequisite for
pixels, but cannot observe compositor/native UI appearance, physical cadence or photons by itself. Instruments
finalization failed after the three attempts below. The revised plan now assigns a bounded investigation to the Mac
M2 builder in M2.4, after independent re-review and milestone prerequisites: changed hypotheses only, at most two
new diagnostic record attempts. It does not claim the failure solved. M2.3 separately commissions qualification of
the shared tap on a real CAMetalLayer surface. M2.2 requires both and a fresh capture-OFF traced cadence arm;
capture-ON untraced/traced arms establish content and observer cost, with ON swaps labelled under capture.
No new run or proof is claimed. The historical grant/unlocked-slot resume instructions below are superseded by
this dependency and proof split. Independent design/JSON review is owed.

The 8 November 2026 slot started at 19:30:56 CET on an M3 Max. The
matched release diagnostic was built from `0d53d3aa`. Both recorded
game windows occupied built-in display 1 with a 1920×1080 game
surface, the noon preset and cooked generation 1. The world stayed
at tick zero. Injected input used the AppKit yaw slider and the normal
app handler. The traced arm ran under Game Performance until the
game exited. [Run identities and event excerpts](slot-events.txt)
include the original file digests.

The [untraced excerpt](control-1x.mp4) and [traced excerpt](trace-1x.mp4)
preserve normal speed and include the initial hold, the two-second pan,
and the settled end. The independent reviewer played the original
7.465 s and 7.307 s recordings beside the proposed [Ground E noon
reference](../../v1-workload/g7-showcase/route-ground-e.png) at 1×.
Both pans read as coherent motion. The cooked rock and ridges tracked
together, the sphere exited as authored, and the endpoints matched.
No gross pause, snap or reversal was apparent in those observations.

| Observation during input | Untraced | Game Performance |
| --- | ---: | ---: |
| Yaw actions accepted | 238 / 238 | 236 / 236 |
| App present calls in pan | 132 | 119 |
| Largest yaw step in those calls | 0.95° | 0.90° |
| Local held scene content found by decode | 38.3 ms | 51.7 ms |
| Longest adjacent video PTS gap in pan | 31.7 ms | 33.3 ms |

The control's rock holds near PTS 1.585–1.623 s while input advances
and the app skips present labels 236–238. In the traced row, near-identical
scene images at 1.360, 1.377 and 1.395 s precede a move at 1.412 s. Nearby
app present calls skip labels 197–200 while yaw edits are accepted. These
are recorded content and app-call findings. The 1× observations cannot
establish whether every short hold was perceptible. [Derived diagnostics](diagnostics.json).

## Review verdict and open cadence row

The independent reviewer passed the motion criterion with the local
holds recorded. The reviewer matched the actual binary, preset, recorder,
and cooked root against both manifests, then checked original decoded
frames and game logs. The source recordings and per-action logs are in
`tmp/g3b-mac-m2/slot.mQNT7T/`.

The cadence box remains open. The headless launch probe stalled before
recording. A headless attach recorded but exceeded its finalization cap.
The live trace reached “Saving output file” and did not finish in 75 s.
Its export refused with `Document Missing Template Error`. The separate
stop-rule review agreed to park **Instruments trace finalization**. The
required next evidence remains a readable trace with game-window swaps
attributed to the built-in panel. ScreenCaptureKit PTS, app present calls,
and the historical trace remain separate from that result.

## Independent review briefs

Watch both original game-window clips at 1× against the proposed noon
reference. Verify the matched build and built-in placement, that both
clips bracket the full sweep, and whether any repeat, pause or jump
affects the motion verdict. Compare decoded scene content and PTS with
the input and present-call records. Assess the failed trace export
separately. For the stop-rule review, inspect all three Instruments
attempts and keep the original cadence criterion open if none yields
fresh attributable swap rows.

## Historical resume instructions (superseded above)

G3c changed the default vertex path after the reviewed slot. The
retained release diagnostic over `474b4306` passed the three-view offscreen
parity check on 9 November: all complete PPM bytes at 0°, 18° and 36°
match `framing.mKC7g4`. The builder inspected the three-view contact sheet
for the rock, sphere, ridge continuity and endpoint framing. This is static
parity, not new live-motion or cadence evidence. [Run identity and results](parity-20261109.json).
The reproducible original command is:

```sh
out=/Users/ryannel/Workspace/thrum/tmp/g3b-mac-m2
work=/Users/ryannel/Workspace/thrum/tmp/g3s-stars-main-wip
framing="$out/framing.mKC7g4"
fresh=$(mktemp -d "$out/resume-parity.XXXXXX")
THRUM_CAPTURE_SCENE="$fresh/capture" THRUM_SCENE=valley \
  THRUM_ASSET_STORE="$work/tmp/g7-showcase/runs/check-20261108-162905/store" \
  THRUM_CAPTURE_ROUTE_ONLY=1 \
  THRUM_CAPTURE_PARAMS="$framing/noon-yaw-00.params.json:$framing/noon-yaw-18.params.json:$framing/noon-yaw-36.params.json" \
  "$out/source/target/release/v1-game"
for yaw in 00 18 36; do
  cmp "$framing/capture/route/noon-yaw-$yaw.ppm" "$fresh/capture/route/noon-yaw-$yaw.ppm"
done
```

Then corroborate the new build with one untraced live pan in an unlocked
slot using the command
below. The parked Instruments investigation needs a separate follow-up.

```sh
slot=$(mktemp -d "$out/slot.XXXXXX")
python3 "$out/run_pan.py" control "$slot/control" --owner-window
```

## 9 November blocker probe

The retained live `slot.mQNT7T/trace/pan.trace` still fails a fresh
`xcrun xctrace export --input <trace> --toc` with exit 10 and
`Document Missing Template Error`. The console reports both
`IOConsoleLocked = Yes` and `CGSSessionScreenIsLocked = Yes`. No new
live row was launched. Instruments finalization remains parked after its
three recorded attempts; M2.2 stays open. Full probe output is in this
worktree’s `tmp/platform-runtime-mac-m2/`.

## Unlocked-console probe, 9 November 01:13 UTC

The owner unlocked the console for the resumed M2 slot. A fresh direct
`ioreg -n Root -d 1` probe reports `IOConsoleLocked = No`. The retained
`474b4306` diagnostic executable, recorder and fixed cooked store were
copied into this worktree's `tmp/platform-runtime-mac-m2/`; the runner
was adjusted only to use those local paths.

Under the shared `gpu` lock, `run_pan.py control <fresh-output>
--preflight-only` refused at `CGPreflightScreenCaptureAccess()` with
`M1 SCREEN RECORDING PERMISSION REQUIRED` (recorder exit 3). The same
preflight outside the Codex sandbox returned the same refusal. No game,
recording or trace was launched, and no permission prompt was requested.
Unlocking the console therefore resolved the old lock blocker but did
not provide Screen Recording permission to this execution context.

At that probe M2.2 remained open and the milestone was not claimed. The then-planned next step was a Screen
Recording grant and matched control/trace rows. The current re-plan above replaces that recorder prerequisite with
engine capture; the failed preflight remains a historical fact. Instruments finalization and fresh attributable
swaps remain unverified, and the prior motion verdict retains its original scope.
