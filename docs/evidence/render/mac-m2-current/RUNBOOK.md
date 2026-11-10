# Attended Mac M2 continuation, historical preparation

Use the [current one-command runbook](../mac-m2-20261110/RUNBOOK.md). The retained notes below describe the earlier checkpoint.

Run from this worktree, on its retained release binary and `tmp/mac-m2/valley` cooked store.
The owner only needs to unlock and activate the **built-in panel**, and arrange Instruments authorization if macOS
requires it. The builder drives AppKit controls and judges evidence. No Screen Recording grant, gameplay,
automatic unlocking or physical iPhone is involved.

One resumable command (the GPU lock also prevents parallel timing work):

```sh
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.4.0/skills/orchestration/build/scripts/agent_loop.py lock gpu --worktree . -- python3 scripts/qualify_mac_m2.py --output tmp/mac-m2/attended
```

The locked-console execution was tried and refused before launching a game or trace. Reuse this exact command after
unlock. Do not compile/cook or run disk-heavy work alongside it. It observes active built-in displays and records
mode/refresh; `window.txt` must then identify the actual game window on that display. A refresh value of zero from
CoreGraphics means adaptive/unspecified, not zero-Hz operation. Check the actual ProMotion setting and record it.
The command refuses changed binary/content pins, failed arms and unchanged trace retries. It never passes JSON items.

1. The first invocation collects real-window diagnostic-grid replacement, cooked-valley replacement, ON/untraced pan
   and OFF/untraced probe, all in the same binary. `THRUM_MAC_QUALIFY` enables the harness; `THRUM_MAC_TAP=on|off`
   explicitly selects capture. `THRUM_MAC_REPLACE=1` is used only for the two replacement diagnostics. At the thirtieth
   presented submission the render owner invalidates its surface lease, withholds its outstanding tap readback,
   reconfigures normally and retires/replaces the capture session. `retirements.tsv` must show a pending staging
   slot, later images must belong to the fresh presenter, and presentation must continue. Retired images are gaps,
   never a continuous-motion pass. The recorder never holds/reacquires a drawable: collection happens after the
   ordinary `queue.present(frame)` consumes it and the frame method returns. The OFF arm creates no tap/writer/spool;
   its preallocated normal markers are retained separately from capture resources.
2. Before tracing, the builder must judge **all of M2.3**, including exact known-pattern channel/row correspondence
   and stale-session refusal. The diagnostic grid is navigation/functional evidence; it does **not by itself** replace
   the required known channel/row oracle. Use the real-Metal odd-width RGBA/BGRA presenter correspondence and loss/
   stale-owner/disabled-allocation regressions from `./dev check`, plus the actual-window producer/submission/lease
   records. If these do not establish the item on this window, leave M2.3 open and fix/extend its live proof before
   proceeding. Inspect the live pattern and cooked scene before/after replacement on the active panel. Offscreen
   tests alone cannot pass it. Package each ON arm with
   `python3 scripts/package_mac_m2_window.py --output tmp/mac-m2/attended/<arm>`; inspect native full frames/crops and
   original host-time 1× sequence. `window-manifest.json` joins slider requests to surface submissions to original
   pre-encoding hashes. Finish this review before writing `window-acceptance.json` in the attended directory:
   `{"M2.3":"pass","binary_sha256":"<identity.json value>","evidence":"<actual proof record>"}`.
   This is a builder evidence record, not a request for owner approval. Resume the same command.
3. The resumed command collects ON/Game Performance, then OFF/Game Performance. These two recordings conservatively
   consume the **entire two-attempt 9 November commission**. The first is also the tooling probe; the second runs
   only if its trace and exports finish. The durable local journal is `tmp/mac-m2/trace-commission.json`; retain it
   across restarts and worktree handoffs. No deletion, new output name or Mac M3 split renews the allowance.
   Any failure stops immediately. A second *diagnostic* after a failed first needs a distinct written falsifiable
   hypothesis; this command deliberately refuses that retry. Exhaustion needs a scoped re-plan.
4. After both recordings, collect full raw exports and inspect them before changing statuses. Each recording is
   bounded to 16 s plus at most 75 s finalization; TOC and `displayed-surfaces-interval` exports each have a 30 s
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
  Establish an **independently correlated trace clock anchor** from this PID (or calibrate a trace event to the media
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
locked-console workflow through the retained ungranted launcher. Keep all four items open where proof is incomplete;
claim M2 only after every item passes, then commit and use `agent_loop.py land --worktree .`. The lead runs review.
