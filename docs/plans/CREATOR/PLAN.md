# CREATOR plan

Closes gate G6. [Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

## Open for the owner

- Assumption: the starter gets a Chrome form now. The G6 decision left it as an owner question and recommended deferring it to G7's page; website demos, and the sea and cloud demo scenes, need a creator's scene in Chrome.
- Assumption: "profiler" in the V1 exit is met by readouts in the demo UI on both targets, fed by the frame host's completed samples and the shells' callback records, not by a separate tool.
- Assumption: the profiler is built after `APPLE-TILE:M3` and not beside it, so it waits on that module's attended items. The other order has the readouts built now on the positional GPU slots, with the switch added later.
- Assumption: the profiler shows encode and submit as one row, and the frame-callback interval in place of a display interval, because those are what the engine measures. Physical cadence stays with the Mac qualification.
- Assumption: the profiler toggle is off by default, so a kit scene and the V1 window run untimed unless the profiler is on or a frames report was asked for.
- Assumption: the fresh-agent proof is run by the milestone's reviewer, who builds the scene before reading engine source.
- Assumption: G6 closes without the clipping overlay, the shadow-cascade overlay and parameter files as a cooked product, which the G6 decision holds or parks.

## M1 The starter scene runs in Chrome from one command, with its controls and live asset edits

- [x] The kit's web runner, facade and page on `thrum_app::web`, with the Mac-only kit pieces gated by target.
- [x] The web form of `./dev scene <folder>` and `./dev starter`: build for wasm32, cook, watch, serve, print the
      URL.
- [x] `scripts/check_starter_browser.mjs` with `scene`, `controls`, `edit` and `journey` cases, on hardware
      WebGPU only, reading pixels.
- [x] `./dev check-web` also compiles and lints the starter for wasm32.
- [x] The Must Fix findings F1 and F2 of [review 1](../../evidence/CREATOR/M1-review.md) and its suggestions S3–S5,
      as [the work list after review 1](../../evidence/CREATOR/M1-review-1-work.md) records.

## M2 The demo UI shows where a frame's time goes on Mac and in Chrome

Read [the frame timing feed](DESIGN.md#the-frame-timing-feed) first. Merge after `APPLE-TILE:M3` and, if
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

## M3 A fresh agent builds a new scene for Mac and Chrome from the public docs alone, and G6 closes

- [ ] The README, creator guide and starter README cover the web form and the profiler, and stand alone.
- [ ] The Mac mistake journey still explains each case; fix what the M1 and M2 reviews recorded.
- [ ] The on-glass check of the panel and saved-file reload: attended setup, then the agent drives and captures
      the real window. Batch the setup with the Mac qualification session.
- [ ] Maturity rows and the brief's G6 line brought up to date.

**For the lead, when M3 is Ready for Review.** The reviewer is the fresh agent of M3.1 and must do that exercise
before it reads this plan, the design or any engine source. Start the review in a new session and change nothing
else: the item tells the reviewer the order. A review whose reviewer says it was not fresh comes back Could Not
Judge on M3.1; start another.
