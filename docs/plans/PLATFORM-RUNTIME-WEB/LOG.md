# PLATFORM-RUNTIME-WEB log

- 2026-11-08 Reviews: design and plan at 12d4c81d, REVISE: pause/resume are game-owned, so the shell cannot be generic over the
  runtime alone; material reload does GPU work while lost. Brief `tmp/g3b-web-shell/review-brief-design.md`, report
  `tmp/g3b-web-shell/review-design.md` (Opus 5.5, 8 November). Recheck at 14fb0b8a: APPROVE WITH CHANGES (R1–R7, same
  report), all taken in this revision. Extraction code at 987a3ac3: APPROVE WITH CHANGES (brief
  `tmp/g3b-web-shell/review-brief-extraction.md`, report `tmp/g3b-web-shell/review-extraction.md`, Opus 5.5), all taken.
- 2026-11-08 Plan drafted from Thrum Core's D-M3 routing, docs only. It was submitted with the shared-file seam and two
  open questions before any code.
- 2026-11-08 Revised to Thrum Core's ruling. Placement is the wasm-only `thrum_app::web` module; report text stays in
  the game facade over typed shell counters and samples; the second starter page is parked as a reuse follow-up. The
  blanket `host_mut` refusal is withdrawn: it would have refused solar time, camera and input edits that today's facade
  intentionally keeps usable while lost. The loss policy now classifies each operation, the shell gates only capture
  start, and M1 gains a retained-view checker addition, landed first on today's facade, proving the recovered device
  presents a camera and time changed during loss.
- 2026-11-08 Revised for the independent design review (REVISE).
  [Full entry](../../evidence/PLATFORM-RUNTIME-WEB/log-entries.md#entry-1).
- 2026-11-08 Recheck at 14fb0b8a approved with changes, all taken. Game edits compare through `host()` before one
  mutable borrow, so unchanged edits reset no history (R1). Both gate witnesses count GPU calls on the lost device, and
  attach-after-loss gets its own navigation (R2). A deferred attach that is refused ends the attempt in
  `recovery_error`, and a later attach replaces a held one (R3). Each witness state converges before its images, and
  either repeat failing withdraws the screenshot claim (R4). The route reference is the witness commit (R5). The panic
  hook moves without a new crate (R6). `web::clock` is a pure state machine (R7).
- 2026-11-08 First work item built. The attach-after-loss witness is a separate `valley-late-attach` case, since it needs
  the cooked requests held before navigation. A deferred attach refused on the replacement device is reported in
  `cooked_status` errors, the page's existing home for cooked failures, rather than `recovery_error`, which would offer
  a retry for a healthy device.
- 2026-11-08 Extraction built (987a3ac3). Independent code review (brief `tmp/g3b-web-shell/review-brief-extraction.md`,
  report `tmp/g3b-web-shell/review-extraction.md`, Opus 5.5): APPROVE WITH CHANGES, all taken. The play state is now
  recorded only after the world applies it; plan text says `web_clock` and puts overload in the shell (the R7 entry
  above predates the rename); doc nits. The seam-table row for the game host changed: `host/web.rs` is new and the
  native wrapper is not cfg-gated. Thrum Core sees it in the extraction seam diff.
- 2026-11-08 M1 runs: reference (a66352e3) and after (c1e03736) both 30/30 twice and material twice; route images
  byte-identical at the route stops; Wasm +10,414 B; `./dev check` passes. The route check's baseline item is held:
  it fails on the reference with the same night-upslope drift.
- 2026-11-08 Loss gates with Thrum Core's retained-view correction landed on main at 0a9009c1. The extraction's route
  check passes on Thrum Core's refreshed night baseline (73fdca0c).
- 2026-11-08 Thrum Core cleared the extraction at 68423c0a with one cleanup, taken: the console panic hook stays in V1's
  `create_game`, thrum-app drops its direct `wasm-bindgen` dependency and `web-sys` `console` feature, and `WebGpu`'s
  doc says the `FrameHost` keeps generation-bound handles while the game keeps none of its own.
- 2026-11-08 M1 closed. Thrum Core cleared the extraction at 68423c0a on one condition: keep V1's console panic hook in
  `create_game`, drop thrum-app's unused wasm-bindgen import and console feature, and clarify the `WebGpu` handle doc.
  All three are done. Rebased over Thrum Core's contribution-recovery change (000ffcdb) and G5's cooked mesh witness.
  Reruns all pass:
  - on 000ffcdb: the five Chrome recovery and loss cases, `material` on the development page and `./dev check`;
  - on 50d33835: `--case all` 30/30 and `./dev check`;
  - on the docs-only head: the repository checks.
  Landed as a fast-forward at 15934fe7.
- 2026-11-08 Follow-up agreed with G3t, not yet built. When `SceneSurface.device` becomes `&DeviceSite`, the shell drops
  its raw `wgpu::Device` fields (`WebGpu`, `WebShell`, `GraphicsDevice`) and reads the device through the published
  capabilities' site, so the facade keeps one device authority. That diff goes as its own seam.
- 2026-11-11 Converted from the flat web shell plan for orchestrate 1.7.0: the design sections became
  [DESIGN.md](DESIGN.md), M1's criteria and work moved to
  [the record](../../evidence/PLATFORM-RUNTIME-WEB/M1-record.md), and this log took the plan's log and review
  line. No done-when file was written, because the module has no open milestone.
