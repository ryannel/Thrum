# CREATOR design review

Reviewed integrated revision `ab46bb6ce89eaa694ab675879afb6eb359da6af4` on 11 November 2026.
Result: **design approved with Suggestions; no blocking findings**.

This review uses the requested orchestrate 1.7.1 review-milestone skill's design procedure, Thrum's engineering
review rules and calibration examples. It judges the design and its proofs, not implementation, visual quality
or device performance. No other agents were launched and no runtime captures were needed.

## Previous findings

The previous `design-review.md` approved revision `17cf2703befa0d510acc1e31a7b4bbbf4367038e` with two Suggestions
and no Must Fix findings. I also read the preceding changes-requested review to check that its resolution remains
in the current assembly.

- **F1 remains resolved.** The original consumer-only profiler could not obtain fresh Chrome samples, separate
  encode/submit measurements or control timestamp work. The current design supplies a bounded profile window at
  each shell's completion drain, names the existing combined Encode + submit measurement, and assigns a timing
  switch to the frame host. Its default, in-flight behavior, replacement behavior and reporting exceptions are
  specified in `docs/plans/CREATOR/DESIGN.md:119–158`. The module list's seam ruling assigns the runtime edits to
  CREATOR M2 after APPLE-TILE M3, with later maintenance owned by the runtime. The plan retains serialization
  with PLATFORM-RUNTIME-MAC M3. This is a settled producer/consumer change, not an instruction to invent one.
- **S1 remains resolved.** M2.6 names runnable perturbations: the valley's existing `world.paused` control and
  added fragment work in the starter's material hook. The design lists coupled rows and compares changes with
  repeated unchanged runs.
- **S2 remains resolved.** M3.1 requires the fresh-agent exercise first, with an ordered read log and an explicit
  `cant_tell` result for a reviewer that already read internals. The launch instruction remains in the plan.
  This design review is not that exercise.
- **S3 remains resolved.** M3.4 separates attended display/permission setup from agent-driven interaction and
  requires actual-window evidence. Offscreen output cannot substitute for the visible window.
- **S4 and S5 remain Suggestions.** Neither has been incorporated into the unchanged mechanism or criteria.
  Their scope and nonblocking classification are retained below.

## Request, scope and revision

I read both complete entries in `docs/plans/CREATOR/ACCEPTED.json.briefs`, oldest first, before comparing the
current design: the project-level commission to line up work through the existing V1 exit, then the task to
revise CREATOR as its design review asks. The first request protects the in-flight PLATFORM-RUNTIME-MAC M2 and
APPLE-TILE definitions. Sea and cloud finishing belongs to the project plan and their modules, not CREATOR.

The project brief's G6 and exit require a reusable one-command creator scene, native/browser typed controls,
asset/shader/parameter iteration, useful profiling, actionable diagnostics and a fresh agent building a small
scene from public docs alone. CREATOR covers the remaining work through the shared browser kit, profiler and
newcomer/closure proof. Existing native controls, material hooks and exposure overlay remain governed by the G6
decision. The additional browser starter and the held clipping/cascade overlays and cooked parameter product
are explicit assumptions in the plan; no hidden narrowing of the accepted request was found.

I compared the complete design, plan and all 17 items with the last approved revision, accounting for their move
from `docs/design/modules/CREATOR*` into `docs/plans/CREATOR/`. After excluding execution bookkeeping, every
DONE definition is identical. The design changes are link migration and the unchanged ownership section moved
from the plan. The plan reorganizes assumptions, history and completed M1 work; it preserves M2/M3's mechanism,
order and acceptance requirements. Its one closure-document wording discrepancy is S6 below.

Additional reading covered README, the project brief, CREATOR's module-list entry and interfaces, ORCHESTRATE,
the G6 decision and relevant creator-guide sections, C11/C12, the hardware-native decision and performance policy,
the web-shell boundary, and APPLE-TILE M3's timing obligations. No plans-folder glossary is present. I used the
prompting-agents guidance when writing findings for the planner.

## Mechanism and boundary checks

The inspection questions were whether fresh samples reach both views independently of report retention, whether
labels match measurement authority, whether the off state can stop GPU work without stranding frames, and
whether the proofs can be performed in their milestone's dependency order.

Scoped source inspection supports the proposed mechanism:

- `crates/thrum-app/src/web.rs:328–337` still drains completions before retaining at most 600 samples. Feeding
  the new window there can observe later frames without changing the existing report buffer. The Mac render
  actor carries completed samples in replies, and `app.rs:565,592` consumes them; the proposed feed need not
  create a second frame lifecycle.
- `crates/thrum-render/src/runtime.rs:175–217` supplies the CPU stages, submission identity and optional GPU
  measurements. Its `encode_cpu` includes finish and submit. The design's combined label is accurate. Current
  positional GPU slots are not mistaken for completed pass-name support: APPLE-TILE M3 item D3.3 explicitly
  supplies that schema and migrates its consumers before CREATOR M2 starts.
- `crates/thrum-render/src/timing.rs:177–240` already prepares bounded query/resolve/readback resources.
  `runtime.rs:1520–1590` has distinct render-complete and timing-resolve states. Capturing demand for each frame
  can preserve timed frames already in flight while allowing untimed completions to free their slots.
- `runtime/device.rs:213–225` discards lost-generation batches during replacement. Read M2.3's “loses no frame
  sample” with the design's explicit existing device-loss exception: the switch must introduce no additional
  loss. Report-enabled runs retain timing from their first frame, so their history is not silently changed.

The bounded sample window, prepared query storage and infrequent table refresh account for the repeated work.
M2.5 requires paired observed cost, rather than an invented performance gate. Its off arm must actually be
untimed: a report-enabled arm alone cannot establish full on/off cost. Callback intervals and present calls
remain explicitly separate from physical display cadence.

The cited external precedents support the approach, not Thrum qualification:
[wgpu-profiler's switch](https://docs.rs/wgpu-profiler/latest/wgpu_profiler/struct.GpuProfiler.html#method.change_settings)
allows pending queries to finish after disabling new queries.
[Bevy's diagnostics](https://docs.rs/bevy_render/latest/bevy_render/diagnostic/struct.RenderDiagnosticsPlugin.html)
illustrate per-pass readouts but document CPU-only collection on Metal and WebGPU; they are not evidence that
Thrum's GPU path works on its targets. That remains for M2's device and browser proofs.

## Acceptance-item audit

These are assessments of whether the items can judge their intended behavior, not new milestone verdicts.

| Item | Assessment |
|---|---|
| M1.1 | The real documented command, hardware adapter and lit-pixel response reject a canned status-only scene. |
| M1.2 | Shared declarations and checked parameter/refusal paths reject a separate web control model. |
| M1.3 | Visible live asset replacement and retained last-good output distinguish working reload from notification alone. |
| M1.4 | Reading scene source and the runner can establish shared game source and no additional web game lines. |
| M1.5 | The named DOM and canvas states can be captured and judged for legibility and actionable errors. |
| M2.1 | Source tracing and window/median/refresh tests establish one measurement authority; M2.6/M2.7 inspect its views. |
| M2.2 | A load change after completion 600 rejects the frozen report buffer. Missing features, faults, stale frames and replacement have explicit outcomes. |
| M2.3 | Device tests and lifecycle inspection can judge per-frame demand, in-flight completion, default compatibility and replacement. |
| M2.4 | Counts of actual resolve/readback work reject a UI-only off state; post-enable frame identity rejects old numbers. See S4. |
| M2.5 | Existing report consumers and recorded paired runs can establish compatibility and observer cost. |
| M2.6 | Repeated causal load changes beyond unchanged-run spread reject arbitrary displayed values; panel captures establish readability. |
| M2.7 | Actual DOM screenshots at default and narrow widths judge the same row set and honest unavailable/precision labels. See S5. |
| M3.1 | The first-in-session exercise, own mesh, shared source and captures on both targets can establish newcomer usability. |
| M3.2 | Public-doc inspection after the exercise can identify missing commands, keys and prerequisites without substituting engine knowledge. |
| M3.3 | The automated and inspected mistake journey tests diagnostics and last-good recovery together. |
| M3.4 | Attended setup plus actual-window interaction can prove visible updates; offscreen captures cannot pass it. |
| M3.5 | Evidence reconciliation can establish separate maturity dimensions and honest remaining scope. See S6 for the migrated document name. |

All required capabilities can exist by their milestone: M2 waits for pass-name timing, and M3 follows the kit and
profiler. No new convincing substitute that satisfies the written proofs while missing the requested behavior
was identified. Existing M1 acceptance is not re-certified by this design review.

## Suggestions

### S4 — Exercise late completions across a rapid off/on transition

**Suggestion; verified ordering risk, not an observed implementation defect.** Location:
`docs/plans/CREATOR/DESIGN.md:124–140`, M2.3/M2.4.

Encode timed frame A, switch off and on before A completes, then deliver A. Clearing the window at each toggle
and checking only the current on state would admit that old sample. Exercise this ordering and admit samples
from the new profiling interval by frame/generation identity; preserve independent report delivery. M2.4 already
forbids pre-switch numbers, so this sharpens an existing proof rather than adding a new acceptance requirement.

### S5 — Distinguish observed timestamp spacing from known precision

**Suggestion; verified inference ambiguity.** Location: `docs/plans/CREATOR/DESIGN.md:110–112`, M2.2/M2.7.

A finite window containing 0.2 and 0.4 ms is compatible with multiple timestamp grids; an all-zero window reveals
no step. The design's instruction to infer the step from the window cannot certify adapter precision.
[Chrome's cited announcement](https://developer.chrome.com/blog/new-in-webgpu-121) documents 100-microsecond
quantization and the development flag that disables it. Label a conservative policy or an observed-spacing
estimate honestly, and use unknown/below-resolution outcomes when the sample cannot distinguish them. Include
constant, zero and sparse windows in formatter coverage. The existing items already permit an honest reason
instead of a number; this remains a labeling/proof clarification, not a blocker.

### S6 — Name M3's closure document after the migration

**Suggestion; verified wording discrepancy introduced by the migration.** Locations:
`docs/plans/CREATOR/PLAN.md:48`, `docs/plans/CREATOR/DONE.json:209–210`, `docs/plans/BRIEF.md:3–6`.

The plan now names the brief's G6 line, but M3.5 still names the build plan. The brief explains that the old build
plan was folded into it while status moved to STATUS.md. Align the item's destination wording with that split
when the planner next edits it: keep the approved requirement in the brief, link closure evidence and remaining
scope from the intended current record, and retain the ledger's separate maturity dimensions. This is not a
missing proof or authority to change the owner's V1 exit; the destination is already recoverable from the plan.

## Validation and limits

Only this review was changed. No implementation, done-when file, accepted record or milestone state was edited,
and no commits or captures were made. Runtime tests and visual/device checks were not run for this design-only
review. `./dev check-docs` passed: 662 Markdown files, 3,191 local links, 30 module records, crate policy and
current design hashes. It emitted only advisory source-size notes. `git diff --check` passed, and
`git status --short` showed only `docs/evidence/CREATOR/design-review.md` modified.

DESIGN OK
