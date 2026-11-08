# Mac M1 live recovery, 8 November 2026

V1's noon cooked valley ran on the M3 Max with its Metal device and built-in
display. The release diagnostic was built over `43e1d488` using witness
source `0c9a32dc` on `codex/wip-g3b-mac-m1-witness`. All rows used the same
binary, parameters and cooked generation. The raw game and recorder logs,
recordings and RGBA inputs are in `tmp/g3b-mac-qualification/slot-04.0qL58y/`.
[Inputs and event excerpts](events.txt) include their SHA-256 digests.
[Machine audit](audit.json) uses the full logs.

| Row | PID / captured window | World endpoint after replacement | Old queue calls after destroy |
| --- | --- | --- | --- |
| Arm disabled | 41901 / 42019 | No replacement | — |
| Paused loss | 42487 / 42035 | Epoch 1, tick 0, timeline 0 | 0 |
| Running loss | 42761 / 42051 | Epoch 1, tick 119, timeline 0 | 0 |

Both loss rows destroyed generation 1 with a drawable held, received its
callback, abandoned the encoded frame, and first presented on device 2 under
surface generation 3. The app window and PID stayed fixed within each row.
The queue check includes empty submits. The running row logged injected P and
the normal Resume acknowledgement before arming. Its authority reached tick
79 at loss and advanced through ticks 80 and 81 before the first replacement
submit. The paused row never requested Resume. Its pre-loss, post-loss and
arm-disabled readbacks were identical across all 2,073,600 RGBA pixels.

[Four-second side-by-side excerpt at 1×](1x-recovery.mp4) shows the arm-disabled
control at top left, paused loss at top right, running loss at bottom left,
and the proposed Ground E noon reference at bottom right. The full recordings
are 18–19 seconds. The first replacement present call falls about 1.07 seconds
after recording start in both loss rows. The injected E actions follow at
about 1.58 and 1.73 seconds. These are event offsets for locating the motion
in the recording. The capture does not measure panel cadence or photon latency.

## Independent review

Both M1 boxes passed. The reviewer replayed the originals at 1× in a
hidden browser alongside the proposed noon reference, then checked the
decoded transition frames and raw logs. Both loss rows recovered the
valley composition and shifted the sphere highlight about 72 pixels
after E, at recording PTS 1.608 s paused and 1.792 s running. The control
held its framing. The reviewer independently checked the runtime input
path, window identity, queue calls, lifecycle order, cooked root and
all 8,294,400 bytes in the paused comparison.

The running recording had one 58.3 ms gap between captured samples near
recovery. The reviewer observed no blank scene in the decoded transition
frames. The sample gap has no display attribution and leaves ProMotion
pacing to M2.

## Independent review brief

Watch the original three recordings at 1× beside the proposed noon
reference. Verify the raw acquire, destroy, loss, abandon and replacement
sequence, all old-queue call boundaries, and the running authority
progress before replacement submit. Verify paused endpoint and byte
equality and the same PID and window. Identify an actual composited
camera response after replacement separately from its queue present call.
Record any observation limits and rule on both M1 done-when items.
