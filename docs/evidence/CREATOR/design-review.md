# CREATOR design review

Reviewed integrated revision `d64769fed5b9c5d1f3e4927db9fe269b9269f3de` on 10 November 2026.
Previous design review: revision `4bbe44a433598650be47c1753c57da79f5d1df93`.
Result: **changes requested — one blocking finding**.

## Request and review scope

I read the complete, sole entry in `CREATOR-ACCEPTED.json.briefs`, including its project-level planning request,
before comparing CREATOR's design and criteria. There are no re-plan or retry briefs in this checkout.
I started from the previous review and independently checked its blocking finding against the current design
and timing producers.
The request requires startable work toward the existing V1 exit, preserving the in-flight Mac and APPLE-TILE
definitions. For CREATOR, the build plan and architecture brief require reusable creator scenes, native/browser
typed demo UI, hot reload, a profiler, and a fresh agent making a small scene from public documentation alone.
Sea/cloud implementation, port scheduling and the completeness of their separate designs are outside this review.

Read: ORCHESTRATE, the requested review skill at version 1.6.2 and its calibration examples, Thrum's review
rules and graded examples, the complete CREATOR design/plan and all 15 done-when items. I also read the relevant
sections of the build plan, architecture brief, module list, G6 decision, creator guide, C11/C12 and web-shell
boundary plan. No plans-folder BRIEF.md or GLOSSARY.md was present. I inspected the timing producer and shell
consumer code named below.
I did not review the entire engine or certify its existing implementation. No other agents were launched.

Inspection questions, set before the source inspection:

- Does the kit reuse one shell and controls authority while keeping scene source shared across targets?
- Can the existing measurements actually supply current CPU-stage, GPU-pass and interval readouts?
- Can a UI-only profiler consumer stop timestamp production without changing runtime ownership?
- Do the criteria distinguish a real cooked scene, visible edits and honest timing from status-only substitutes?
- Can the fresh-agent and attended proofs be executed without acquiring prior implementation knowledge or
  making routine work depend on owner operation?

This is a design and interface review. No runtime, visual-quality or device-performance pass is claimed, and no
captures were necessary. The referenced source is evidence about the proposed mechanism, not a milestone verdict.

## Previous findings and revision check

- **F1: unresolved.** The read-only profiler scope and all three producer limitations remain. The source checks
  below independently confirm them; no producer interface, ownership agreement or sequencing resolution was added.
- **S1: unchanged Suggestion.** The proposed load toggles and interpretation of coupled costs remain unspecified.
- **S2: unchanged Suggestion.** The newcomer exercise still needs launch ordering ahead of normal reviewer reading.
- **S3: unchanged Suggestion.** M3.4 still assigns the drag and saves to a person instead of separating setup from actions.

`git diff 4bbe44a43359 HEAD` contains only review reports and their accepted-record updates. There are no changes
to CREATOR's design, plan or item definitions, the shared contracts, or implementation. All 15 items were reread;
none was weakened or sharpened. There is no new blocking finding in this round. F1 below retains its original
scope and resolution rather than adding a new refinement.

## Must Fix

### F1 — Design the profiler's producer interface; the proposed read-only consumer cannot deliver M2

**Verified.** Locations: `docs/design/modules/CREATOR.md:85–103,119–121`,
`CREATOR-PLAN.md:13–17,28–33`, and `CREATOR-DONE.json` M2.1–M2.3.

The design says the profiler is fed **only** by measurements already taken, and that this module **only reads**
the runtime's frame samples and GPU timings. M2 nevertheless requires separate encode and submit measurements,
current rolling readouts, and an off state with no timestamp writes or readbacks. The named interfaces do not
provide those behaviors:

1. **Chrome's samples stop updating.** `crates/thrum-app/src/web.rs:328–337` drains completed frames but appends
   them only while `samples.len() < SAMPLES`; `SAMPLES` is 600 (`web.rs:40–41`). The public `samples()` getter
   (`web.rs:482–484`) exposes that initial batch. After 600 completions, a median of this buffer remains an
   initial-run value even if the creator changes the load. A repeated read or slower UI refresh cannot make it
   current. A run longer than 600 completions followed by a load change is an ordinary failing case, not a stress
   condition.
2. **Submit time is not a separate measurement.** `crates/thrum-render/src/runtime.rs:184–208` exposes tick,
   extraction, upload and combined encode time. The documented `encode_cpu` includes finish, submit and completion
   registration; `runtime.rs:1256–1309` actually accumulates them into that one value. M2.1's separate encode and
   submit rows cannot both be obtained from these samples. Repeating that value under two labels would mislead the
   creator; adding the required producer measurements contradicts the read-only scope and the ask's prohibition
   on a second timer unless the plan explicitly reconciles it.
3. **Turning the UI off cannot turn GPU timing off.** Slots construct `GpuTimings` in
   `crates/thrum-render/src/runtime/preparation.rs:83–88`. `timing.rs:183–203` disables it only when the enabled
   device lacks timestamp queries. Runtime encoding passes the timing batch unconditionally
   (`runtime.rs:1209–1215,1264–1270`), and completion resolves it in a later submission
   (`runtime.rs:1537–1548`). There is no public profiler enable/disable operation in `FrameHost` or `GpuTimings`.
   Hiding readouts leaves the writes and readbacks running. Removing the device feature at startup would not
   provide the requested live on/off behavior either.

**Consequence:** building the stated readout-only mechanism fails the M2 statements. A builder must instead
invent and change a runtime interface outside the settled scope. That also invalidates the module list's
assumption that CREATOR M2 is ready in other files while APPLE-TILE changes the frame lifecycle.

**Required resolution:** settle a bounded producer/consumer interface with explicit ownership and sequencing
relative to the in-flight runtime/APPLE-TILE work, without changing their protected definitions. Specify the
fresh-sample delivery path, the actual CPU measurements and labels, and how profiling enable/disable handles
in-flight query batches, completion, device replacement and existing reporting consumers. Define interval rows
by their actual source (callback/target/present observation); shell timing must not silently become physical
display cadence. Amend M2's proofs to exercise a load change after the first 600 completed Chrome frames and a
live on/off transition. Keep one measurement authority. The planner can choose the interface; this review does
not require a particular data structure or a new profiling framework.

## Suggestions

### S1 — Make the M2.4 load perturbations executable and interpret coupled costs

**Verified wording and source mismatch.** M2.4 asks the reviewer to toggle Sun shadows, exposure zones and the
cooked volume. `examples/v1-game/src/controls.rs:84–110` declares the exposure toggle but no shadow or volume
toggle; CREATOR's plan does not assign those additions. Name an existing runnable perturbation or commission
the minimal diagnostic control explicitly. Also explain which timings should respond together: GPU passes share
resources, and some current `FrameSample` rows aggregate work (`runtime.rs:204–213`). Requiring every unrelated
row to remain steady can reject honest measurements under contention. A bounded repeated before/after test with
recorded noise and expected affected rows would make this inspection more useful. This sharpens the proof;
it is not a separate architectural blocker.

### S2 — Route the M3 newcomer exercise before the normal review prelude

**Verified process ambiguity.** M3.1 prohibits reading any other design document before doing the exercise,
whereas ordinary reviewer startup reads project settings, graded examples and the design. Preserve the strong
fresh-agent requirement by making its ordering explicit in the future M3 launch: the three public documents and
the exercise first, with a read log, then the normal source/design review. A reviewer contaminated by the normal
prelude should not silently count as fresh. This design review did read internals and is not the M3 exercise.

### S3 — Keep attended setup separate from routine test actions

**Verified.** M3.4's `attended` text requires a person to drag the slider and save the files. An unlocked visible
window is a legitimate setup requirement, but ORCHESTRATE and `AI-DEVELOPMENT.md:178–190` distinguish attended
setup from automated actions and review. Permit the agent to perform the drag and file saves once setup is ready;
name the selected window/native-UI capture method and any required grant. Preserve proof of the actual visible
window rather than substituting an offscreen image. This avoids an unnecessary human-operation dependency.

## Coverage and criterion assessment

The browser starter is a justified, explicitly recorded extension of the earlier Mac-only starter. Reusing
`WebShell`, the DOM controls model and the cooked-client path is a plausible mechanism, and the design preserves
page ownership of RAF/listeners and shell ownership of clock/loss policy. The held overlays, cooked parameter
product, iOS and public-page shader reload are explicit inherited limits rather than hidden omissions. M1 reaches
a useful scene before the profiler and closeout work.

All done-when items were assessed for judgeability and plausible substitutes:

| Item | Design assessment |
|---|---|
| M1.1 | Judgeable: the command, real hardware WebGPU, cooked-scene pixels and slider response are required; the ask rejects status-only/fallback substitutes. |
| M1.2 | Judgeable: shared declarations and checked file application can be traced through both views. |
| M1.3 | Judgeable: live accepted edit and refused edit retaining the previous drawn generation are distinct observable states. |
| M1.4 | Judgeable by reading scene source and the kit runner and counting game lines. |
| M1.5 | Concrete page-state inspection, with canvas and DOM evidence correctly separated. |
| M2.1 | Blocked by F1: the current producer does not expose all the promised measurements or a continuously updating Chrome sample set. |
| M2.2 | Honest unavailability is judgeable once F1 establishes current sample identity and delivery. Include freshness in that resolution. |
| M2.3 | Blocked by F1: the stated consumer cannot disable the runtime's timestamp work. Paired cost observations are otherwise appropriate. |
| M2.4 | Useful causal/legibility inspection after F1; S1 makes the selected perturbations reproducible. |
| M2.5 | Judgeable DOM inspection; numeric rows still need M2.1/M2.2's provenance, not just readable labels. |
| M3.1 | Strong end-to-end proof rejecting starter copies and internal-source dependence; preserve newcomer ordering as in S2. |
| M3.2 | Judgeable against the commands and files actually used by M3.1. |
| M3.3 | Concrete diagnostic/recovery cases with report and image evidence; initial missing-store failure need not fabricate a last-good scene. |
| M3.4 | Observable native-window behavior with a legitimate setup requirement; S3 removes unnecessary manual operation. |
| M3.5 | Judgeable reconciliation of the evidence and separately recorded maturity dimensions. |

No done-when fields or other design files were changed. Approval here would concern the design only; it would not
promote implementation, visual or device maturity.

Validation: `./dev check-docs` passed (616 Markdown files, 2,802 local links, 30 module records, crate policy and
current design hashes). Only this review is changed; no captures or implementation edits were made.

DESIGN CHANGES: 1 blocking findings
