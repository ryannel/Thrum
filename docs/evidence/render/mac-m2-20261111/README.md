# Mac M2 attended diagnostic, 11 November

M2.2 and M2.4 remain open. The first diagnostic finalized and exported a readable Game Performance trace, but its
raw display-surface-swap/VSync tables and derived displayed-surfaces-interval table have zero rows. The built-in
display identity is present. A readable directory and pan signposts therefore leave the required swap proof
unmet. [Result](result.json), [TOC](toc.xml.gz), [swap export](swaps.xml.gz) and [raw display tables](raw-displays.xml.gz).

The first configuration used Points of Interest `excludeOSLogs=true` and the existing release binary
`aa1591522962933b55ec1f347cc538880f5eb14b7ea227bf4adcb7f00b585507`, matching the completed M2.1/M2.3 proofs.
The trace identifies game PID 49285 and window 45393 at 1920×1080 on the M3 Max built-in 3456×2234, 120 Hz display.
Its recorded duration is 8.611888 seconds. Collection, finalization and the three exports finished in 70.185 seconds.
[Invocation](invocation.json), [process log](process.log), [timing](process.timing.json) and [window](window.txt).

The four MacM2Whole/MacM2Pan SDK marker boundaries all resolve to that PID. Duplicate schema projections contain
identical events and are deduplicated. Their media-clock brackets intersect in a consistent offset interval with
width 0.665896 microseconds. This provides the calibrated clock join for this run. The missing swap stream leaves
cadence unqualified. [Signposts](signposts.xml.gz) and [clock calibration](clock-calibration.json).

The GPU lock covered the fresh ON/untraced baseline, ON/traced collection and bounded probe inspection. Another
builder's attended visual-review holder and earlier GPU checks delayed acquisition. Per-arm host load was sampled
throughout collection and exports. The traced arm's one-minute load rose from 3.52 to 19.51. iostat retains interval
CPU and disk samples, while the process snapshot reports lifetime CPU. These load records cover more than the pan
itself. They establish the conditions for later contamination review. [Load](load.json) and [samples](system-iostat.txt.gz).

A read-only panel guard samples every 0.5 seconds, with a two-second I/O Registry timeout. Its failure stops only
the process group created by the collector. A refusal control verified owned-child termination, and actual panel
observations verified the CoreGraphics binding. Temporary bounded caffeinate assertions kept the panel awake
during the attended queue. [Panel witnesses](panel-witness.jsonl.gz).

The second diagnostic hypothesis restores OS-log collection to test whether exclusion removed the required display
event source. It retains capture OFF, the same build/content/extent/input and the absence of a desktop recorder.
Before this recording acquired the GPU lock, the panel became inactive and asleep while the console remained
unlocked. The queued owned process was stopped immediately. One of the two commissioned diagnostic attempts remains.
[Journal](trace-commission.json) and [one-command continuation](../mac-m2-20261110/RUNBOOK.md).

Complete local original archives remain in `tmp/mac-m2/final-window-20261110/on-untraced-current` and `on-traced`,
with timing, input, submission and writer hashes. Fresh packaging and visual inspection are pending. These new
content arms have no new quality verdict. The completed 10 November M2.1/M2.3 proofs retain their scope. Exact
collector source snapshots are retained by SHA-256 in collectors/. Ignored local source archives have no durable
backup. Further diagnostic attempts after the remaining one require a scoped re-plan.
