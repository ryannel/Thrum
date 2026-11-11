# CREATOR design review

Reviewed integrated revision `17cf2703befa0d510acc1e31a7b4bbbf4367038e` on 11 November 2026.
Previous review: `d64769fed5b9c5d1f3e4927db9fe269b9269f3de`.
Design revision: `84652bd38c4c31c6b8d84cc5f1946123ef5e3f2d`.
Result: **design approved, with two Suggestions; no blocking findings**.

## Request and scope

I read both entries of `CREATOR-ACCEPTED.json.briefs`, whole and in order: the project-level commission to plan
continuous work toward the existing V1 exit, followed by the task to revise CREATOR as the previous review asks.
For this module, that requires reusable creator scenes, native/browser typed demo UI, hot reload, a useful profiler,
and a fresh agent making a small scene from public documentation alone. The request protects the in-flight
PLATFORM-RUNTIME-MAC M2 and APPLE-TILE definitions. It does not commission a separate profiling framework or a
change to the V1 exit. Sea/cloud implementation and their port timing are outside this module review.

I used the requested review-milestone skill at version 1.6.2, its design-review procedure and calibration examples,
ORCHESTRATE, Thrum's engineering review rules and graded examples. I read the previous review first, then the
complete current CREATOR design, plan and all 17 items, comparing the changed definitions with the previous
revision. I also inspected the relevant build-plan and architecture-brief requirements, module-list ownership,
interfaces and build order, G6 decision and creator-guide sections, C11/C12, the web-shell boundary, and the
APPLE-TILE M3 timing requirement. There is no plans-folder BRIEF.md or GLOSSARY.md.

Source inspection was scoped to the existing frame sample, timing allocation/resolve, completion delivery and
device publication paths; the Mac shell/actor and Chrome shell consumers; and the named valley pause control.
This is a design and interface review, not an implementation, visual-quality or device-performance verdict.
No runtime captures were necessary, and no other agents were launched.

Inspection questions set before checking those implementation seams:

- Can the revised feed deliver current samples after Chrome's report buffer fills?
- Do CPU and interval labels match the actual measurement authority?
- Can timing stop without cancelling in-flight work or changing existing report consumers?
- Are ownership and build order settled before the runtime edits begin?
- Do the revised proofs reject frozen numbers, a UI-only off switch and stale samples after re-enabling?
- Are the newcomer and actual-window exercises executable in the stated order?

## Previous findings

### F1 — Resolved: the producer interface is now designed and assigned

The former consumer-only design could not supply current Chrome samples, separate encode/submit rows or live
control of timestamp work. The revision resolves each part rather than leaving the builder to invent a boundary:

1. **Freshness.** `CREATOR.md:124–130` assigns a bounded profile window in `thrum-app`, fed from each shell's
   completion drain independently of report retention. Chrome's existing `web.rs:328–337` drains every sample
   but retains only the first 600; a feed at that drain can observe subsequent completions without changing
   `samples()` or the V1 report. Mac delivery likewise already carries completed samples through the render
   actor. M2.2 now requires an actual load change after frame 600, not repeated reading of a frozen buffer.
2. **Measurement meaning.** `CREATOR.md:91–105` uses the combined Encode + submit span, matching
   `runtime.rs:194–197,1309`, and labels shell observations as callback intervals and present calls. The design
   explicitly leaves physical cadence with Mac qualification. This narrows an unsupported label, not the
   requested ability to locate expensive stages; the plan records the choice as an assumption.
3. **Timing demand.** `CREATOR.md:131–158` defines the switch on the frame host, its default-on compatibility,
   the untimed sample reason, per-frame in-flight behavior, preallocated query storage, device replacement and
   report-run override. Existing `timing.rs:183–235` has bounded per-slot storage and an unsupported-feature
   path; `runtime.rs:1531–1548` shows the completion/resolve branch that must honor a frame's captured demand.
   The proposed state is sufficient to distinguish an untimed frame from a timed frame still completing.
4. **Ownership and sequencing.** The module list's frame timing interface and CREATOR seam ruling assign the
   runtime switch to the CREATOR M2 builder after APPLE-TILE M3, then to the runtime owner for maintenance.
   M2's explicit dependency agrees. APPLE-TILE M3 already promises graph-pass names and consumer migration.
   The CREATOR plan also serializes Mac shell integration with PLATFORM-RUNTIME-MAC M3 if open. The planner's
   revision changes only CREATOR's three design/plan/item files and the module list, leaving the protected
   module definitions alone.
5. **Existing consumers.** Report and qualification runs remain timed from their first frame; the first-600
   report and Mac history retain their meanings. The default-on host preserves callers that never use the
   new control. M2.3–M2.5 explicitly require tests and consumer inspection, including real browser work counts.

These are bounded additions to the existing authority. They do not require another timer or an independent
frame lifecycle. The required implementation and device tests are still future work.

### S1 — Resolved: executable perturbations and coupled costs

The unavailable shadow/volume toggles were replaced by the existing `world.paused` control
(`examples/v1-game/src/controls.rs:107`) and the existing starter material hook. M2.6 names the expected CPU/GPU
rows, repeated no-change controls and readability inspection. `CREATOR.md:178–185` explains coupled rows and
requires an explanation for changes outside that list. This preserves the causal proof while making its actions
buildable. The builder must retain the exact material edit and run settings so the reviewer can repeat them.

### S2 — Resolved: newcomer ordering is explicit

M3.1 puts the exercise first, permits the process prelude and three public documents, requires an ordered read
log, and gives `cant_tell` if the reviewer already read internals. The plan has a launch instruction for a new
review session. Source/design review follows the exercise. This review has read internals and is not that proof.

### S3 — Resolved: attendance is setup, with agent-driven actions

M3.4 names unlocked/awake setup and Screen Recording/Accessibility grants, then has the agent drag, save and
capture the actual window and panel by window ID. The fallback person performs only the drag when Accessibility
is unavailable. Offscreen frames are expressly excluded. The locked-session evidence route remains explicit;
it must use the attended run's actual-window captures, not claim a fresh interaction the reviewer did not perform.

## Changed criteria and plausible substitutes

Every changed item was reread against the prior findings; the renumbering does not silently remove the former
load-response or DOM inspections. M2 grows from five to seven items. M1 definitions are unchanged; M3.2 and M3.5
only gain empty review fields, with no change to their acceptance statements.

| Definition | Assessment |
|---|---|
| M2 bar and needs | Judgeable. Preserves useful, current readouts, replaces the vague overhead promise with an observed paired cost, and puts runtime work after APPLE-TILE M3. Read the off-cost claim with the explicit report-run and in-flight exceptions. |
| M2.1 | Judgeable. Names actual sources, the combined CPU span, sample window and refresh tests. A second timer or physical-display label cannot pass. Present calls are counts; the design's source table governs their meaning. |
| M2.2 | Stronger than before: late-run perturbation catches the first-600 substitute; reasons, faults, missing support and replacement clearing are checked. See S5 on precision labels. |
| M2.3 | Judgeable against the designed per-frame switch and device tests. Untimed frames still complete, timed in-flight frames finish normally, and default callers retain behavior. Read “loses no frame sample” as no additional loss from the switch: the design expressly retains abandonment of a lost device's batches. `runtime/device.rs:213–225` already counts/discards those batches and replaces slots. |
| M2.4 | A new real-browser transition proof counts resolve/readback work and requires continued presentation and fresh post-enable samples. Merely hiding rows cannot pass. S4 sharpens the ordering exercised by that proof. |
| M2.5 | Preserves the former paired-cost requirement and adds report/qualification compatibility. The cost comparison must exercise a genuinely untimed off arm; a report-enabled run keeps timing on by design and cannot alone measure the full on/off difference. |
| M2.6 | Carries the former M2.4 visual/causal inspection with runnable perturbations, repeated controls and explained coupling. The measured row must actually move; a control-state display is insufficient. |
| M2.7 | Carries the former M2.5 DOM inspection and adds an off state. Same-scene rows must remain readable at normal and narrow widths. |
| M3.1 | Stronger ordering and provenance for the existing fresh-agent exercise; still requires an original cooked mesh, both targets and actual captures. |
| M3.4 | Preserves real-window response to all three actions and removes routine dependence on a human operator. No offscreen substitute is accepted. |

The unchanged M1.1–M1.5 still require the real hardware browser scene, shared control declarations, accepted and
refused live edits, shared scene source and legible page states. M3.2–M3.3 still require sufficient public docs
and reproducible mistake/recovery behavior; M3.5 reconciles evidence with separate maturity dimensions.
No new gap against the accepted briefs or interface was found. The browser starter and held overlays remain
explicit scope decisions examined in the earlier review, not hidden changes in this revision.

## Suggestions

### S4 — Exercise late completions across a rapid off/on transition

**Suggestion; verified ordering risk, not an observed implementation failure.** Locations:
`CREATOR.md:124–140`, M2.3 and M2.4. Emptying the profile window at a toggle does not by itself exclude an old
frame that completes after the new on state. For example, encode timed frame A, switch off and on before A
completes, then deliver A. A feed conditioned only on the current toggle would admit A into the new window.
`FrameSample` already has `submission_order` (`runtime.rs:179–180`), and the shells track device generations.

Include that ordering in the switch tests and make sample admission honor the new profiling interval, using
frame/generation identity rather than clearing alone. Do not discard the same completion from the independent
report path. This sharpens an existing proof: M2.4 already forbids displaying pre-switch numbers, so the wrong
implementation would violate its present statement. No new runtime authority is needed.

### S5 — Distinguish observed timestamp spacing from known precision

**Suggestion; verified ambiguity in the inference, not a blocker to usable readouts.** Locations:
`CREATOR.md:110–112`, M2.2 and M2.7. A finite window of durations cannot uniquely identify timestamp resolution:
values of 0.2 and 0.4 ms are compatible with both a 0.1 ms and a finer grid; an all-zero window provides no step
estimate. Inferring a step from those values must not certify the adapter's actual precision.

The cited [Chrome announcement](https://developer.chrome.com/blog/new-in-webgpu-121) documents 100-microsecond
quantization and its development flag. Use a conservatively labeled browser precision policy or an explicitly
estimated spacing, and an unknown/below-resolution reason when the observations do not distinguish it. Include
constant, zero and sparse-value windows in the formatter tests. The existing items already allow an honest
reason instead of a number, so this is clearer labeling and proof coverage, not a requirement for a new timing
measurement or browser capability API.

## Validation and limits

No implementation, milestone state, done-when definition or accepted record was changed. Only this review was
written; no captures were created. Documentation validation is recorded below. Approval concerns this design
revision only and does not promote implementation, correctness, visuals or device maturity.

Validation: `./dev check-docs` passed (622 Markdown files, 2,893 local links, 30 module records, crate policy
and current design hashes). `git diff --check` passed; `git status --short` confirms only this review changed.

DESIGN OK
