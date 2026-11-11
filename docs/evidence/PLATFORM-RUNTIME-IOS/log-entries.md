# PLATFORM-RUNTIME-IOS: log entries in full

Entries of [the module log](../../plans/PLATFORM-RUNTIME-IOS/LOG.md) that ran past a few lines, moved here unchanged on 11 November 2026 when the plans took the orchestrate 1.7.0 shape. The log keeps each one's opening and a link.

<a id="entry-1"></a>

## Entry 1: 2026-11-09

- 2026-11-09 Built M7. Shared scene/surface resize prepares a replacement ring and presenter, validates the
  current epoch before publication, drains pending frames, reuses identical extents and preserves fixed-render
  surface-only callers. Device candidates refit scene and presenter, then refresh current camera/lighting at
  publication. Contributions retain size-independent pipelines and receive C07 history reset with the new matrix.
  Full pre-change and final fresh-simulator routes passed. Final rotation has two-reply gaps, unchanged identities,
  zero old-extent presents, matrix-derived 65° vertical FOV, and matching actual targets/drawables in both orientations.
  Inspected native whole images/crops and both 60-frame strips. Resource overlap includes recovery's third ring.
  Evidence and limits are in [the M7 record](../render/ios-extent/README.md).
  Initial native regression-test setup used an entry point that could skip unchanged publication; corrected it
  to the serial encoded-token submission used by iOS. The final gate covers that correction. Claim awaits lead review.

<a id="entry-2"></a>

## Entry 2: 2026-11-08

- 2026-11-08 M6 development runs (`tmp/g3b-ios-rotation/m6-dev1..6`). In dev run 1 the held-frame loss faulted
  the surface: wgpu-core's `discard` checks device validity before clearing the acquired texture, so the surface
  stays acquired for any device. A loss that held a drawable now replaces the surface (walk step 6). Three route
  defects followed:
  - the recovery hold sat before preparation, so the rotation never forced a refit;
  - the 90 s hold expired before a 109 s XCUITest rotation at load ~190;
  - the background row counted the designed resume reinstall as a replacement.
  The hold now sits after preparation with a 300 s cap. The route also gained the `stale-wake` row. Dev run 6 passed
  every device row: generations 1→2→3, stale wake and request refused, refit to 2622x1206, one publication after
  resume, and a swipe after recovery moved the actor 9.6 m.
