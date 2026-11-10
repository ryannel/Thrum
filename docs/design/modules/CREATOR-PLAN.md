# Creator experience plan (G6)

Design: [CREATOR.md](CREATOR.md), under [the G6 decision](../decisions/G6-CREATOR-EXPERIENCE.md) ·
Done when: [CREATOR-DONE.json](CREATOR-DONE.json) · Module list: [ARCHITECTURE.md](ARCHITECTURE.md)
Reviews: none yet. The design review is owed through the loop before a builder starts.
Milestone: M1 The starter scene runs in Chrome from one command
Now: planned on 10 November 2026; nothing built under this plan yet (since 2026-11-10)
Next: design review, then M1, then M2. M3 closes the gate.
Blocked: none. M3.4 needs an attended session on the unlocked built-in display.

## Ownership

This module owns `crates/thrum-controls`, the kit, runners, page and file reload in `crates/thrum-app`,
`examples/starter`, the `scene` and `starter` commands, and the creator docs (`README.md`,
`docs/design/ENGINE-EXPERIENCE.md`, `examples/starter/README.md`). It reads frame samples and GPU timings from
`thrum-render` and uses the web shell's frame, clock and loss policy unchanged. A change to either goes in the
log as a proposal for [the module list](ARCHITECTURE.md#interfaces).

## M1 The starter scene runs in Chrome from one command: not started

- [ ] The kit's web runner, facade and page on `thrum_app::web`, with the Mac-only kit pieces gated by target.
- [ ] The web form of `./dev scene <folder>` and `./dev starter`: build for wasm32, cook, watch, serve, print the
      URL.
- [ ] `scripts/check_starter_browser.mjs` with `scene`, `controls`, `edit` and `journey` cases, on hardware
      WebGPU only, reading pixels.
- [ ] `./dev check-web` also compiles and lints the starter for wasm32.

## M2 The demo UI shows where a frame's time goes: not started

- [ ] The profiler group in the controls model, fed by the shell's frame samples and `GpuTimings`.
- [ ] Unavailable rows with their reason; the off path writes no timestamp query.
- [ ] Both views show it; the browser check gains a `profiler` case.
- [ ] A paired on/off run on the valley recording the profiler's own cost.

## M3 A fresh agent builds a new scene for both targets, and G6 closes: not started

- [ ] The README, creator guide and starter README cover the web form and the profiler, and stand alone.
- [ ] The Mac mistake journey still explains each case; fix what the M1 and M2 reviews recorded.
- [ ] The attended on-glass check of the panel and saved-file reload, batched with the Mac qualification session.
- [ ] Maturity rows and the build plan's G6 line brought up to date.

## Log

- 2026-11-10 Planned from [the module list](ARCHITECTURE.md). The earlier G6 slices were built before the loop and
  are recorded in the G6 decision and its evidence; this plan covers only what closes the gate.
- Assumption: the starter gets a Chrome form now. The G6 decision left it as an owner question and recommended
  deferring it to G7's page; website demos, and the sea and cloud demo scenes, need a creator's scene in Chrome.
- Assumption: "profiler" in the V1 exit is met by readouts in the demo UI on both targets, fed by the engine's
  existing frame samples and GPU pass timings, not by a separate tool.
- Assumption: the fresh-agent proof is run by the milestone's reviewer, who builds the scene before reading
  engine source.
- Assumption: G6 closes without the clipping overlay, the shadow-cascade overlay and parameter files as a cooked
  product, which the G6 decision holds or parks.
