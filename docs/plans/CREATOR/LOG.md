# CREATOR log

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

- 2026-11-11 Built and claimed M1. `./dev starter --web` and `./dev scene <folder> --web` build the shared
  40-line scene, cook and watch its assets, and serve the kit page. Shared model setters drive its DOM controls.
  Hardware Chrome 157 proofs cover submitted Sun edits, mesh and texture publications, refused generation
  retention, the same running page/scene, parameter refusals, tab visibility and a 480 px window. The builder
  inspected all six page states. [M1 evidence](../../evidence/CREATOR/M1.md) records the source and artifact hashes.
  The final milestone gate has four passes, including `./dev check`. One earlier V1 allocation-suite failure
  cleared in focused exact-binary runs and full reruns, with its cause unconfirmed and retained in the evidence.
  Mac and Wasm linting include the native test gate and the kit's Wasm test target. Independent review is the lead's.

- 2026-11-11 Reclaimed M1 after [review 1](../../evidence/CREATOR/M1-review.md). F1 is reproduced by the
  strengthened edit case before the fix. Refusals now expire when a later generation draws, while transport
  errors retain their own lifetime. Generation 5 restores the pre-refusal pixels on the same running page.
  F2 moves platform handling into the kit, reuses its existing public types on native targets, and reports
  unsupported native window runners. The starter's common 40-line source compiles for iOS simulator, and
  `./dev check-mobile-core` now includes that starter probe. Linux/Windows host CI was not run here.
  S3 observes the adapter actually returned to the page, S4 applies Config.window_title, and S5 clarifies
  web parameter loading. Reviewed S2 duplicate messages are shortened. [Fresh evidence](../../evidence/CREATOR/M1-r1.md)
  includes four passing Chrome cases, native compile/lint records, six inspected page states and the full gate.
