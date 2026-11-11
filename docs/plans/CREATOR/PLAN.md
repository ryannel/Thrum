# Creator experience plan (G6)

Design: [CREATOR.md](CREATOR.md), under [the G6 decision](../decisions/G6-CREATOR-EXPERIENCE.md) ·
Done when: [CREATOR-DONE.json](CREATOR-DONE.json) · Module list: [ARCHITECTURE.md](ARCHITECTURE.md)
Reviews: [design review 2 at `d64769fe`](../../evidence/CREATOR/design-review.md): DESIGN CHANGES (1 Must Fix, 3 Suggestions). Revised 11 November; a fresh design review of this revision is owed through the loop.
Milestone: M1 The starter scene runs in Chrome from one command
Now: M1 reclaimed after fixing review 1 blockers F1 and F2 (since 2026-11-11)
Next: lead re-review of M1. M2 starts when `APPLE-TILE:M3` is Done. M3 closes the gate.
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

## M1 The starter scene runs in Chrome from one command: claimed after review 1

- [x] The kit's web runner, facade and page on `thrum_app::web`, with the Mac-only kit pieces gated by target.
- [x] The web form of `./dev scene <folder>` and `./dev starter`: build for wasm32, cook, watch, serve, print the
      URL.
- [x] `scripts/check_starter_browser.mjs` with `scene`, `controls`, `edit` and `journey` cases, on hardware
      WebGPU only, reading pixels.
- [x] `./dev check-web` also compiles and lints the starter for wasm32.

### Review 1 work

Review: [M1 review 1](../../evidence/CREATOR/M1-review.md).

- [x] F1. Clear a retained loader refusal when a later generation draws. Extend the edit case through
      restoring the mesh, checking restored submitted pixels and a resolved status on the same page.
- [x] F2. Put unsupported native platform handling inside the kit, preserve the scene's common source,
      and compile the starter on an installed non-Mac native target through a normal dev check.
- [x] S3. Gate hardware proof on the adapter actually returned to the running page.
- [x] S4. Use the scene's Config title in the browser.
- [x] S5. Describe Mac and web controls and parameter loading in the source comment.

S2's duplicated next steps are removed in the reviewed load-failure and missing-store paths. S1's cooker
error channel and S7's pre-existing view-heading duplication remain follow-up suggestions. S6's Mac diagnostics
retain their existing path and remain covered by the planned M3 mistake journey.

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
