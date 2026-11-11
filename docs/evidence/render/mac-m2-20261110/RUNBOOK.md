# Attended Mac M2 continuation

11 November stop. Both diagnostic attempts under the 9 November commission are used. Both saved readable Game
Performance traces within the bounds, with capture ON and OFF respectively. Both have zero raw swap/VSync and
derived displayed-surface rows. M2.2/M2.4 stay open. The [complete evidence](../mac-m2-20261111/README.md) retains
both hypotheses, traces' exports, clock calibration, load records and source inspection. Further diagnostic
recording requires a scoped re-plan. Preserve `tmp/mac-m2/trace-commission.json`; renaming an output or resuming
this command grants no new allowance. The commands below document the tested collection methods.

Run from `/Users/ryannel/Workspace/thrum/.claude/worktrees/mac-m2`, on its retained release binary and `tmp/mac-m2/valley` cooked store.
The retained binary hash is `aa1591522962933b55ec1f347cc538880f5eb14b7ea227bf4adcb7f00b585507`.
Its source map is in the current evidence manifest. Rebuilds require fresh matching motion/window evidence.
After attended setup the builder resumes this command and owns all subsequent inspection.
The command leaves any authorization prompt to the person at the Mac, and stops at its bounded deadline if unavailable.
The owner only needs to unlock and activate the **built-in panel**, and arrange Instruments authorization if macOS
requires it. The builder drives AppKit controls and judges evidence. No Screen Recording grant, gameplay,
automatic unlocking or physical iPhone is involved.

One resumable command holds the exclusive GPU lock across the fresh baseline, both recordings and probe inspection.
The existing window proofs are reused. The collector takes a fresh `on-untraced-current` baseline for the current
background load, then `on-traced` and `off-traced`. Each arm records iostat interval CPU/disk samples, load averages,
and a process snapshot. A read-only panel witness samples every 0.5 seconds, with a two-second I/O Registry query
timeout. A locked console, inactive/asleep panel or unavailable witness stops the collector's owned process group.

```sh
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.1/skills/orchestration/build/scripts/agent_loop.py lock gpu --worktree . -- python3 scripts/qualify_mac_m2.py --output tmp/mac-m2/final-window-20261110
```

The four untraced window arms are already retained on the current binary and M2.3 has builder proof. Reuse this exact command after attended setup. Keep at least 16 GiB free before each arm. The command checks this before consuming a recording attempt. The owned development build directory can be reconstructed with `./dev check`; inspect it for active work before using `env RUSTC_WRAPPER= cargo clean --profile dev`. Preserve `target/release/v1-game`, the cooked store, source archives and the attempt journal. Do not compile/cook or run disk-heavy work alongside it. It observes active built-in displays and records
mode/refresh; `window.txt` must then identify the actual game window on that display. A refresh value of zero from
CoreGraphics means adaptive/unspecified, not zero-Hz operation. Check the actual ProMotion setting and record it.
The command refuses changed binary/content pins, failed arms and unchanged trace retries. It never passes JSON items.

1. The first invocation collects real-window diagnostic-grid replacement, cooked-valley replacement, ON/untraced pan
   and OFF/untraced probe, all in the same binary. `THRUM_MAC_QUALIFY` enables the harness; `THRUM_MAC_TAP=on|off`
   explicitly selects capture. `THRUM_MAC_REPLACE=1` is used only for the two replacement diagnostics. At the 360th
   presented submission the render owner invalidates its surface lease, withholds its outstanding tap readback,
   reconfigures normally and retires/replaces the capture session. `retirements.tsv` must show a pending staging
   slot, later images must belong to the fresh presenter, and presentation must continue. Retired images are gaps,
   never a continuous-motion pass. The recorder never holds/reacquires a drawable: collection happens after the
   ordinary `queue.present(frame)` consumes it and the frame method returns. The OFF arm creates no tap/writer/spool;
   its preallocated normal markers are retained separately from capture resources.
2. Before tracing, the builder must judge **all of M2.3**, including exact known-pattern channel/row correspondence
   and stale-session refusal. The live fixture has an independent full-resolution CPU channel/row oracle. Every original must equal it. Use the real-Metal odd-width RGBA/BGRA presenter correspondence and loss/
   stale-owner/disabled-allocation regressions from `./dev check`, plus the actual-window producer/submission/lease
   records. If these do not establish the item on this window, leave M2.3 open and fix/extend its live proof before
   proceeding. Inspect the live pattern and cooked scene before/after replacement on the active panel. Offscreen
   tests alone cannot pass it. Package each ON arm with
   `python3 scripts/package_mac_m2_window.py --output tmp/mac-m2/final-window-20261110/<arm>`; inspect native full frames/crops and
   original host-time 1× sequence. `window-manifest.json` joins slider requests to surface submissions to original
   pre-encoding hashes. Finish this review before writing `window-acceptance.json` in the attended directory:
   `{"M2.3":"pass","binary_sha256":"<identity.json value>","evidence":"<actual proof record>"}`.
   This is a builder evidence record, not a request for owner approval. Resume the same command.
3. The resumed command collects ON/Game Performance as the first tooling probe. After its exports, it holds the GPU
   lock for at most 120 seconds while the builder resolves the fresh TOC and actual app/window swap records.
   Only after that proof passes, write `tooling-acceptance.json` with
   `{"M2.4":"pass","binary_sha256":"<identity.json value>","evidence":"<fresh attributed swap evidence>"}`.
   This is the builder's evidence judgement. The collector then takes OFF/Game Performance for cadence. If the
   inspection wait expires, resume the same command after saving the proof. A failed probe requires diagnosis
   and a distinct falsifiable hypothesis before another diagnostic recording. The durable attempt journal is
   `tmp/mac-m2/trace-commission.json`. The two recordings consume the entire two-attempt 9 November commission.
   Retain the journal across restarts and worktree handoffs. A new output name or the Mac M3 split grants no new
   diagnostic attempt. Exhaustion needs a scoped re-plan.

4. After both recordings, collect full raw exports and inspect them before changing statuses. Each recording is
   bounded to 16 s plus at most 75 s finalization; TOC, `displayed-surfaces-interval` and signpost exports each have a 30 s
   timeout. Only the runner's owned process group is terminated at a deadline; never kill unrelated Instruments/
   game processes. If Instruments leaves its separately launched child, identify the owned PID from that arm's
   `clock-anchor.tsv`, verify its command and creation identity before stopping it. No process-name-wide kill.

## Changed trace hypothesis, written before recording

The retained [slot log](../mac-m2-pan/slot-events.txt) already says **target app exited**, then saving exceeded 75 s
and export failed with `Document Missing Template Error`. Merely ending the target earlier is not a new hypothesis.
The raw historical bundles are not present in this worktree; these retained logs and
[historical failure account](../mac-m2-pan/README.md#review-verdict-and-open-cadence-row) are the available diagnosis.
Fresh nonrecording `--show-recording-options` succeeds outside the sandbox and exposes
`Points of Interest.excludeOSLogs=false`. The changed probe keeps **Game Performance** but sets this option true,
with no desktop recorder: unrelated OS-log material may account for finalization work. This is unproven.
The runner writes the hypothesis, exact options and journal **before** launching; it succeeds only if fresh TOC and
actual app/window-attributed swap exports are readable within their bounds. Failure rejects this configuration;
trace directory existence is never success. No diagnostic recording was made while locked.

## Evidence the resumed builder must judge

- Resolve every XML `id`/`ref` in the displayed-surfaces table, including rotating surfaces. Require one window number
  and PID matching `window.txt`/`clock-anchor.tsv`, on the active built-in display, in one app-labelled stream. Use
  numeric **Swap Time**, not narrative frame labels or generic start-time fields. The old
  `qualify_macos_display.py` ten-minute/numeric-threshold check does not qualify this four-second worded criterion.
- `controls.tsv` records requested and normal AppKit-accepted yaw. `requests.tsv` binds accepted yaw and callback target
  to render request sequence; `surface.tsv` binds that sequence to actual renderer submission and platform lease.
  `images.tsv` records actual `Instant` submission time relative to the world origin and later map-observation media
  time separately. `clock-anchor.tsv` brackets the origin/media/unix relation. For origin elapsed E and media bracket
  [A,B], an image at origin time S lies in [A-E+S,B-E+S]; keep the full bracket as its uncertainty.
  The SDK `MacM2Whole` and `MacM2Pan` intervals use this same PID. Their four boundaries are bracketed in `signposts.tsv` by `media_before`/`media_after`. Resolve their exported trace timestamps, intersect the four permitted clock-offset ranges, and retain the uncertainty. Establish an **independently correlated trace clock anchor** from this PID (or calibrate a trace event to the media
  clock with a bounded error); do not assume trace zero equals process launch or shell `time.time_ns`. If the export
  lacks a usable anchor, leave cadence open. This collector supplies clock observations, not an automatic valid join.
- Bracket the complete one-second initial hold, two-second 0°–36° pan and one-second settled final hold in actual
  submissions and independent swaps. List every missing/unmatched sample and all intervals/gaps; investigate visible
  holds. No numeric performance threshold gates this milestone.
- Package/review the ON/traced sequence at 1×; compare it to ON/untraced for tracing overhead. Compare ON/OFF traced
  CPU (`load.json`, `system-iostat.txt`, game report) and per-frame GPU (`gpu-samples.txt`) rows for capture overhead;
  retain run order and background load. **Only OFF/traced** supplies shipping cadence; ON swap figures say “under
  capture.” OFF must have zero images, no spool and no enabled-tap marker. Record any missing GPU samples explicitly.
- Engine submission time, readback availability, movie PTS, callback targets, VSync and attributed compositor Swap
  Time remain separate. Native controls/final compositor appearance and photons are excluded from engine pixels.
  Input-to-photon, physical panel output and physical iPhone performance remain unmeasured. A required physical/
  compositor appearance observation needs its own attended reason and evidence.

M2.3 and M2.4 must pass before M2.2 is qualified. If the current build differs from the M2.1 evidence, first rerun its
permission-denied runtime workflow through the retained ungranted launcher. Keep M2.2/M2.4 open while trace proof is incomplete;
claim M2 only after every item passes, then commit and use `agent_loop.py land --worktree .`. The lead runs review.
