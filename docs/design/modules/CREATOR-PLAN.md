# Creator experience plan (G6)

Design: [CREATOR.md](CREATOR.md), under [the G6 decision](../decisions/G6-CREATOR-EXPERIENCE.md) ·
Done when: [CREATOR-DONE.json](CREATOR-DONE.json) · Module list: [ARCHITECTURE.md](ARCHITECTURE.md)
Reviews: [design review 2 at `d64769fe`](../../evidence/CREATOR/design-review.md): DESIGN CHANGES (1 Must Fix, 3 Suggestions). Revised 11 November; a fresh design review of this revision is owed through the loop.
Milestone: M1 The starter scene runs in Chrome from one command
Now: design and items revised after the second design review; nothing built under this plan yet (since 2026-11-11)
Next: design review, then M1. M2 starts when `APPLE-TILE:M3` is Done. M3 closes the gate.
Blocked: none. M2 waits on `APPLE-TILE:M3`. M3.4 needs attended setup on the unlocked built-in display.

## Ownership

This module owns `crates/thrum-controls`, the kit, runners, page, profile window and file reload in
`crates/thrum-app`, `examples/starter`, the `scene` and `starter` commands, and the creator docs (`README.md`,
`docs/design/ENGINE-EXPERIENCE.md`, `examples/starter/README.md`). It uses the web shell's frame, clock and loss
policy unchanged.

It reads frame samples from `thrum-render`. In M2 only, under
[the seam ruling](ARCHITECTURE.md#interfaces), its builder adds the timing-demand switch and the "not requested"
reason to the frame runtime, and one call in `examples/v1-game` that declares the profiler group. Any other change
to the frame runtime, the shells' policy or the V1 game goes in the log as a proposal for the module list.

## M1 The starter scene runs in Chrome from one command: not started

- [ ] The kit's web runner, facade and page on `thrum_app::web`, with the Mac-only kit pieces gated by target.
- [ ] The web form of `./dev scene <folder>` and `./dev starter`: build for wasm32, cook, watch, serve, print the
      URL.
- [ ] `scripts/check_starter_browser.mjs` with `scene`, `controls`, `edit` and `journey` cases, on hardware
      WebGPU only, reading pixels.
- [ ] `./dev check-web` also compiles and lints the starter for wasm32.

## M2 The demo UI shows where a frame's time goes: waiting on `APPLE-TILE:M3`

Read [the frame timing feed](CREATOR.md#the-frame-timing-feed) first. Merge after `APPLE-TILE:M3` and, if
`PLATFORM-RUNTIME-MAC:M3` is open, one after the other with it: both touch the Mac shell.

- [ ] The read-only table kind in `thrum-controls`, drawn by the AppKit panel and the DOM view.
- [ ] The profile window in `thrum-app`, fed at each shell's completion drain; the web shell's first-600 report
      buffer and the Mac run history keep their meaning.
- [ ] The timing-demand switch on the frame host and the "not requested" reason, with device tests in
      `./dev check` for both directions, frames in flight and a device replacement.
- [ ] The profiler group: the toggle, the rows by source, each unavailable reason, the Chrome step size.
- [ ] The shells set demand from the toggle; report and qualification runs stay timed.
- [ ] The browser check gains `profiler` (a load change after frame 600) and `profiler-switch` cases.
- [ ] The V1 game declares the group; the paired on/off run and the two known changes on the Mac.

## M3 A fresh agent builds a new scene for both targets, and G6 closes: not started

- [ ] The README, creator guide and starter README cover the web form and the profiler, and stand alone.
- [ ] The Mac mistake journey still explains each case; fix what the M1 and M2 reviews recorded.
- [ ] The on-glass check of the panel and saved-file reload: attended setup, then the agent drives and captures
      the real window. Batch the setup with the Mac qualification session.
- [ ] Maturity rows and the build plan's G6 line brought up to date.

**For the lead, when M3 is Ready for Review.** The reviewer is the fresh agent of M3.1 and must do that exercise
before it reads this plan, the design or any engine source. Start the review in a new session and change nothing
else: the item tells the reviewer the order. A review whose reviewer says it was not fresh comes back Could Not
Judge on M3.1; start another.

## Log

- 2026-11-10 Planned from [the module list](ARCHITECTURE.md). The earlier G6 slices were built before the loop and
  are recorded in the G6 decision and its evidence; this plan covers only what closes the gate.
- 2026-11-10 [Design review 1 at `4bbe44a4`](../../evidence/CREATOR/design-review-1.md): DESIGN CHANGES, one Must
  Fix (F1). Review 2 found it unresolved, because no revision had been made between them.
- 2026-11-11 Revised after design review 2. M1 is unchanged. M2 and M3 items changed.
  - F1: the profiler was a reader of interfaces that could not serve it. The design now settles the frame timing
    feed: a profile window fed at each shell's completion drain, rows named by their real source, encode and
    submit as the one span the runtime measures, a timing-demand switch on the frame host with its behaviour for
    frames in flight, device replacement and report runs, and a read-only table kind in the controls model. M2
    now needs `APPLE-TILE:M3`, and the module list's build order and interfaces say so. No `APPLE-TILE` or
    `PLATFORM-RUNTIME-MAC` file changed.
  - S1: M2.6 names two changes that run today (pausing the valley's world, a starter material-hook edit), and the
    design lists the rows that move together. Changes are read against the row's spread with no change.
  - S2: M3.1 and the design put the newcomer exercise before the rest of the review, with a read log, and a
    reviewer that was not fresh gives Could Not Judge.
  - S3: M3.4's attended text is setup only; the agent drags, saves and captures the real window.
- Assumption: the starter gets a Chrome form now. The G6 decision left it as an owner question and recommended
  deferring it to G7's page; website demos, and the sea and cloud demo scenes, need a creator's scene in Chrome.
- Assumption: "profiler" in the V1 exit is met by readouts in the demo UI on both targets, fed by the frame
  host's completed samples and the shells' callback records, not by a separate tool.
- Assumption: the profiler is built after `APPLE-TILE:M3` and not beside it, so it waits on that module's attended
  items. The other order has the readouts built now on the positional GPU slots, with the switch added later.
- Assumption: the profiler shows encode and submit as one row, and the frame-callback interval in place of a
  display interval, because those are what the engine measures. Physical cadence stays with the Mac qualification.
- Assumption: the profiler toggle is off by default, so a kit scene and the V1 window run untimed unless the
  profiler is on or a frames report was asked for.
- Assumption: the fresh-agent proof is run by the milestone's reviewer, who builds the scene before reading
  engine source.
- Assumption: G6 closes without the clipping overlay, the shadow-cascade overlay and parameter files as a cooked
  product, which the G6 decision holds or parks.
