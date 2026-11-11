# COOKED-DRAW log

- 2026-11-09 Builder applied planner revision `58c3414e`. Removed every console admission/refusal and manual setup
  wait across all simulator modes. Console observations retain actual raw output or an unavailable reason.
  [Full entry](../../evidence/COOKED-DRAW/log-entries.md#entry-1).

- 2026-11-09 Owner corrected console locking from a run precondition to a recorded capability witness. Overnight
  locked runs established that the route could work without desktop capture; requiring that setup on every check
  blocked verification during normal Mac use without strengthening the proof.
  [Full entry](../../evidence/COOKED-DRAW/log-entries.md#entry-2).

- 2026-11-09 Replanned dependency after builder checkpoint `b0dae34c` blocked M1.10. Its README and
  `rotation/layout-failure.json` show the source stays 640×360 while surface/presented extents rotate. G3b now owns
  PLATFORM-RUNTIME-IOS M7 for coherent scene, internal target, projection and surface extent changes. Added
  `needs: ["PLATFORM-RUNTIME-IOS:M7"]`, changed blocked to open for dependency sequencing and removed the obsolete
  free-text blocker. All item definitions, including M1.10, are unchanged; fails/replans are unchanged because this
  routes missing owner work, not a four-review failure. No new proof or acceptance claimed. The enclosing loop
  records the definitions and invalidates obsolete proof; fresh design review remains owed. No owner decision.
- 2026-11-09 Resumed from rebased checkpoint `478c17ff` under the revised, reviewed M1.3. Added compile-time opt-in reference/omission controls, independent cooked-product coverage validation and command/resource observations. Normal builds omit these controls. M1.3 passes exact D/R and I/D/R repeats in every required view, detects each D-only terrain omission, retains R output/arguments across the defect and restores equality. All-meshlet I controls are exactly I in every view. Whole-frame/native-crop inspection classifies the residuals as partition-dependent shading with matching geometry, materials, lighting, shadows and volume. The low-level cause remains unverified.
  - Direct loading now omits unused meshlet range/bounds buffers. The material GPU suite covers both records. Device replacement covers unchanged paths and capability re-selection in both directions. The qualification feature exposes actual buffer usages and bound commands for the public adapter-less-host assertion, including a child calling the exact game attachment entry point. `./dev check` includes that observed-resource arm and passes 664 Rust test/doc-test results; final feature-enabled all-targets Clippy also passes.
  - M1.9 runs the real XCUITest rotation row on a fresh owned iPhone 18 Pro / iOS 27.2 simulator with both console-lock witnesses true. Process/window/device identities persist and cooked submissions advance at each new extent.
  - M1.10 fails its current-extent fill requirement. Observed presenter viewports are portrait `[0,972,1206,678]` inside `1206×2622` and landscape `[239,0,2144,1206]` inside `2622×1206`. **Owner dependency** is G3b iOS/render runtime support for matching scene/internal render aspect to current oriented extent on the same process, window, lease and generation. `resize_surface` updates the presenter and retains the internal source extent. This builder leaves that owning-runtime work for its owner, as build-milestone requires.
  - M1.4/M1.5 base comparisons, M1.6 cost/tripwires, the rest of M1.10 and M1.11 pan proof remain unfinished. M1.7/M1.8 stand as the owner directed. T2-CAPTURE M2 remains gated on M1 acceptance. Acceptance definitions remain unchanged. No reviewer launched.

- 2026-11-09 Re-planned M1.3 at the owner's request after blocked checkpoint `9c230ff4`. The first valley view's 1,506
  differing pixels / max delta 18 persisted with all meshlets kept; one full-range indirect command matched direct
  exactly. This isolates draw partitioning for that view, not the low-level cause.
  [Full entry](../../evidence/COOKED-DRAW/log-entries.md#entry-3).

- 2026-11-09 Builder checkpoint, following the fresh DESIGN OK and building against the proposed additive interfaces as permitted above. The capability record lends known/unknown downlevel flags; unknown selects direct. Direct draws allocate no cull/indirect/count/readback resources; history writes run on both paths. Cooked GPU cases run on adapter-backed and masked records; the bench retains an adapter-backed indirect record. The ordinary iOS selection attaches cooked content and exposes drawn-device generation. The initial simulator feature risk is cleared; no additional unsupported feature surfaced. M1.7/M1.8 pass on the final source: locked-console advancing counters, background hold/resume and a drawn-device witness on generation 2.
  - M1.3 is blocked: 1,506 final submitted pixels differ in the noon valley view, despite exact same-path repeats. Scratch controls disprove a rejected-meshlet explanation and isolate command partitioning. **Proposal to planner:** investigate partition-invariant output, or explicitly reconsider the whole-range strategy or exact oracle. Do not alter the production indirect path or weaken equality within this build.
  - Remaining views, base/Chrome parity, cost/tripwire work, rotation/pan and full visual qualification are unfinished. The parity command refuses incomplete modes. T2-CAPTURE M2 remains gated on acceptance. See the evidence README for actual checks and simulator records. `./dev check` passes (651 tests, including all 13 cooked GPU cases on both paths); documentation checks pass. No reviewer launched, acceptance definitions unchanged, no milestone claim.

- 2026-11-09 Answered the [design review](../../evidence/COOKED-DRAW/design-review.md).
  [Full entry](../../evidence/COOKED-DRAW/log-entries.md#entry-4).

  Items were renumbered (stills M1.10, pan M1.11, docs M1.12). No proof existed, `fails` and `replans` stay 0, and a
  fresh design review is owed.

- 2026-11-09 Planned, at the owner's direction through the lead, after T2-CAPTURE M2 was blocked
  ([evidence](../../evidence/render/t2-capture-m2/README.md)). The design selects a direct full-range indexed draw,
  chosen from adapter downlevel flags in the capability record. It rejects CPU culling, compaction culling, delayed
  readback, per-meshlet direct draws and a simulator override, on cost and complexity grounds. T2-CAPTURE M2 now
  `needs` `COOKED-DRAW:M1`. The design names an adjacent gap for G3c: synthetic culled modes use indirect draws
  without selection. No implementation, and no agents launched. Independent design review is owed.

- 2026-11-09 Resumed after PLATFORM-RUNTIME-IOS M7 review. M1.10's former fixed-source blocker is resolved by the owning runtime. A fresh locked held-view run covers noon start, 36-degree end, low sun, mirrored diagnostic twins/volume and both orientations. Sixteen pan samples span the start/end holds and moving view; native edge/horizon crops are inspected, with the 1.118-second largest sampling gap explicitly limiting the conclusion. Ordinary lifecycle and the existing rotation row also pass on the integrated host without held controls. The rotation harness now checks the selected authored 46-degree valley FOV instead of assuming the 65-degree diagnostic camera.
  - Production indirect BASE/current output and counts match exactly for five Mac views and four Chrome views, with both-build repeats. BASE is the integrated parent of the first implementation commit. Baseline archives receive caller/fixture/witness overlays only; production encoders/shaders are unchanged. Separate Cargo target roots avoid mixed-revision build-cache reuse.
  - Release Mac costs use four order-alternated I/D pairs, 128 GPU-complete samples per arm, and actual host load records. Direct has no contribution compute sample. These are descriptive costs, with no selection or speed promotion. The first tripwire run reported an unseeded quiet class and was not treated as a qualified comparison; genuine BASE bench runs were collected under their actual load classes before the final gate. No reviewer launched; acceptance definitions and planner needs remain unchanged.

- 2026-11-09 M1 claimed: all twelve items pass with named-target evidence. Final integrated-M7 D/R/I rerun reproduces the retained inspected payloads; Mac/Chrome BASE comparisons and both-build repeats are exact. Fresh locked ordinary simulator lifecycle/recovery/rotation and held/sampled visuals pass. All tripwire metrics pass against same-load history; three actual quiet BASE measurements per history were added without changing the encoder. Final `./dev check` passes 666 tests in 62 groups, zero failures; explicit iOS target Clippy passes. Owned bulk scratch is retired after retained-source/evidence verification. Only M1 state and item status/evidence changed in the done-when file; planner needs and review fields are preserved. No reviewer launched. Integration through land is next.

- 2026-11-09 Fixed the loop's plain Check failure. The original launcher inherited the user Cargo configuration's sccache wrapper and exited 2 with empty output. A non-quiet Cargo diagnostic reported Operation not permitted. Earlier successful checks supplied RUSTC_WRAPPER= externally. The launcher now supplies that default itself and preserves explicit caller values. The regression fixture covers unset, empty and explicit wrappers, and rejects the original launcher in the unset case. Plain ./dev check passes 666 Rust tests and all twelve repository Python tests under the GPU lock with the caller environment unmodified. Renderer and target proof remain valid. Only M1 item evidence changes in the done-when record. The correction goes through the required land gate.

- 2026-11-09 Requalified the exact M1.7 and M1.8 commands after the owner manually locked the Mac. The loop failure was the real unlocked-host precondition, confirmed by IOConsoleLocked=No. The harness now allows a bounded 120-second manual setup wait at startup and records lock observations, while missing lock evidence still fails. Seven capture tests cover immediate locked admission, a manual-lock transition and timeout refusal. Exact fresh function proof records twelve advancing observations over 22.1 seconds. Exact fresh lifecycle proof holds background counters and redraws Direct on device generation 2. Both runs have true before/after lock witnesses and deleted their owned simulators. The owner retains manual console setup through loop verification. Acceptance definitions and reviewer fields are preserved.

- 2026-11-11 Plan reshaped for orchestrate 1.7.0: status lines and M1's finished work list removed (git keeps
  them), `Ownership and agreements` moved to the design, and the console witness handoff moved to
  [evidence](../../evidence/COOKED-DRAW/console-witness-handoff.md). No item or scope changed.
