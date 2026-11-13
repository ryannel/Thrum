# PLATFORM-RUNTIME-MAC design review

11 November 2026. Reviewed integrated tip `4af24b81f803bbc055fad6b72ac6fc5e87b85fb7` using
review-milestone's “Reviewing a design” procedure and Thrum's engineering review rules.

No Must Fix findings. Two Suggestions below. The revised qualification design is suitable to build;
this verdict does not qualify current motion or panel cadence.

## Previous round and request

The previous review at `3e5be4a760190434acd2655d43d7cf92c549ee8e` ended `DESIGN OK`, with no
Must Fix findings or Suggestions. There are no outstanding findings to resolve. Its approval concerned
the earlier swap-table protocol and does not establish the replacement witness's implementation.

I read both complete tasks in [ACCEPTED.json](../../plans/PLATFORM-RUNTIME-MAC/RECORD.json), oldest
first, and [the project brief](../../plans/BRIEF.md). The first task authorized a format-only reshape.
The second explicitly authorizes a scoped re-plan after readable attended traces contained no display
events: diagnose the evidence, provide a working display-event source or another authoritative cadence
witness, and identify any necessary owner action. That later task permits this witness change; the
reshape's earlier prohibition on scope changes does not forbid the commissioned re-plan.

The required outcome remains a recoverable Mac game with independently observed presentation cadence
on the built-in ProMotion display, complete elapsed-time motion evidence, and fresh qualification after
the Apple tile change. Engine image times, requested opportunities and callback times cannot substitute
for observed presentation. The revised plan explicitly records its witness choice as an assumption for
the owner; it does not silently narrow the request or require an additional approval before building.

## Findings

### S1 — Suggestion, verified: narrow the authorization diagnosis

[The diagnosis](m2-display-event-diagnosis.md), lines 31–38, and
[PLAN.md](../../plans/PLATFORM-RUNTIME-MAC/PLAN.md), lines 24–26, say authorization is not the cause
of the missing display events. The supplied evidence establishes that system-wide kernel tracing
worked and that this particular source was silent. That rules out a blanket refusal of kernel tracing;
it does not establish every source-specific cause. The diagnosis itself correctly says the cause is
unknown in its next paragraph.

Keep the conclusion at that supported scope: there is no demonstrated need to change Developer mode,
and the alternative witness already emitted data under the existing setup. This is a wording correction,
not a request for another recording, a security-setting change or an owner decision. It does not block
the selected mechanism, which no longer depends on repairing the silent display source.

### S2 — Suggestion, verified: reconcile the shared descriptions of the replaced proof

[C12](../../design/CONTRACTS.md#unattended-visual-and-motion-evidence--planned-t2-capture-extension),
lines 609–612, still calls M2.2 a built-in-panel “swap proof”, while the
[shared platform design](../../design/modules/PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation),
lines 87–92, still assigns M2.4 trace-finalization diagnosis. The revised module instead assigns M2.4
the presented-time command and M2.5 full-interval recording, and explicitly leaves kernel swaps unmeasured.

Update those cross-references on the planner's next turn so a reader following the shared documents
reaches the current witness and item responsibilities. The accepted re-plan expressly permits an
alternative witness, and the external OS observation, capture-OFF requirement and ownership invariants
remain intact. This is stale descriptive wording, not a new producer/consumer API conflict or a reason
to restore the retired table requirement.

## Witness assessment

I inspected the retained OFF and ON TSV tables directly. With the diagnosis's stated difference
conversion, `diff(requested_time_raw) * 3 / 125` nanoseconds, they contain:

| Arm | Rows / drawable IDs | Two-refresh intervals |
| --- | --- | --- |
| OFF | 349 / 496–844 | 526→527: 16.665250 ms; 544→545: 16.666625 ms |
| ON | 343 / 505–847 | 558→559: 16.665416 ms |

All retained statuses are 2 and the ordinal ranges contain no holes. Ordinary intervals range from
8.327709 to 8.338958 ms across the two tables. Independently subtracting the `media_seconds` values
for submissions 544 and 545 in the retained OFF surface log gives 15.470792 ms, matching the proposed
known-answer case. These calculations reproduce the fixture observations; they do not validate absolute
presented time, the complete interval, or the original TSV extraction's PID attribution.

The export's “Requested Time” label deserved checking rather than assuming it meant presentation.
Apple's installed Instruments model provides supporting primary evidence. Under
`/Applications/Xcode.app/Contents/Applications/Instruments.app/Contents/Packages/GPU.instrdst/Contents/Extensions/`,
`com.apple.metalsystrace-common.dtac/modeling-rules/rules-0002.clp`, lines 545–552, reads
`CA_CLIENT_METAL_PRESENTED_HANDLER`, including a separate present-time value, drawable ID, status and PID.
Lines 893–916 write that present-time value into `ca-client-presented-handler.at-time` after conversion,
and write the event timestamp separately. This supports using that field instead of handler time or
`delta`; the table name and regular-looking intervals alone would not have done so.

Apple's [presentedTime documentation](https://developer.apple.com/documentation/metal/mtldrawable/presentedtime)
defines an OS-provided onscreen time and a zero result for an unpresented or dropped drawable. Its
[Metal performance guide](https://developer.apple.com/documentation/xcode/analyzing-the-performance-of-your-metal-app)
also distinguishes app activity from display observations. The installed model and the retained values
support the proposed alternative as a design choice. They do not establish a direct measurement of
scan-out or photons, and the revised design correctly excludes those claims.

Absolute conversion and fresh-run attribution remain required implementation proofs. The retained raw
values are not usable as trace-relative nanoseconds directly. M2.4/M2.5 require derived conversion,
marker validation and stated error, instead of simply fitting an offset to make the pan appear covered.
The command must also preserve missing/invalid observations rather than treating ordinal continuity as
proof of presentation. M2.2's statement requires an observed interval for every submission; reporting
unmatched samples is diagnostic disclosure, not permission to qualify an uncovered interval.

## Acceptance-item audit

I compared the done-when definitions with `3e5be4a76019` and read every current item. The changed
acceptance fields are exactly M2.2, M2.4, new M2.5 and M3.2; status/evidence resets are not new criteria.
The previous review's boundaries remain relevant even though it had no findings.

| Item | Design judgement and false proof excluded |
| --- | --- |
| M0.1 | Unchanged historical ownership review. No new implementation claim is inferred from its carried pass. |
| M1.1 | Unchanged same-binary loss/control rows, retained authority and queue audit distinguish recovery from restart or old-device work. |
| M1.2 | Unchanged complete paused-image equality and composited response after replacement distinguish recovery from a frozen matching endpoint. |
| M2.1 | Unchanged complete elapsed-time motion, control/image joins and denied attributed preflights remain independent of panel setup. Fixed-step motion and missing submitted images cannot qualify it. |
| M2.2 | Revised witness remains fresh, release-build-specific and capture OFF, with three matched arms, the full pan and holds, built-in display attribution and investigated long intervals. Callback timing, historical fixtures, ON-arm cadence and merely listing missing rows cannot satisfy its statement. The design retains observer comparisons and host-load reporting. |
| M2.3 | Unchanged actual-window producer correspondence, lease lifetime, pending-readback replacement and disabled direct-path proof remain prerequisites. Offscreen equality alone cannot pass. |
| M2.4 | Offline parser work is judgeable within M2 from retained known-answer tables, source inspection and refusal tests. It must find the three gaps and the surface-log join while rejecting those partial traces as whole-interval qualification. A script that merely prints the expected three answers would fail command inspection and cannot supply M2.5's fresh result. |
| M2.5 | Fresh recording must establish retained coverage of both whole-interval markers, per-submission joins, validated conversion, display identity and recording count. A readable trace directory, presence of markers alone or the old truncated stream cannot pass. |
| M3.1 | Unchanged complete M2.1 rerun on the post-APPLE-TILE-M3 assembly prevents extending an earlier visual verdict to changed output. |
| M3.2 | Revised witness is rerun with all M2.2 arms and M2.3 window proofs on M3.1's build. Full coverage, conversion/error and attribution remain required; earlier traces and functional window logs cannot qualify the changed path. |

M2.4 needs no recording. M2.5 explicitly addresses the observed ring-buffer truncation through a changed
recording configuration whose retained coverage must be demonstrated. The exact successful configuration
is build work with an observable stop condition, not an assumption that the old template now works.
The six-recording allowance is explicit in this scoped re-plan, bounded by recording/finalization/export
limits and the GPU lock; unchanged unsuccessful configurations are not repeated. M3 grants only its two
traced qualification arms, with another re-plan required for diagnostic work.

M2 depends on T2 M0 and does not wait for APPLE-TILE. Final M2.2 follows M2.3/M2.4/M2.5; reusing a qualifying
M2.5 OFF recording requires all OFF-arm conditions. M3 follows Mac M2 and APPLE-TILE M3. No new dependency
cycle, clock owner, submission owner or capture producer is introduced. C02/C06/C07 lifetime and authority
rules remain unchanged; C12's substantive separation of images, device observation and observer cost remains.
The module retains bounded capture resources and investigates hitches without introducing a numerical
performance acceptance threshold.

## Verification and limits

Read both accepted briefs, the project brief/module entry, previous review, full current module design,
plan and ten acceptance items, the revised definition/design diffs, current diagnosis and attended evidence,
relevant shared platform/frame-host and C02/C06/C07/C12 sections, and the project/skill calibration examples.
Read the module log after forming the item assessment. Used prompting-agents guidance for actionable findings.

Direct Python/CSV calculations over the two retained compressed tables and OFF surface log reproduced the
counts and intervals above. Read Apple's installed Instruments event model and current primary documentation.
No runtime image or motion inspection, GPU work, trace recording, permission change or other agent was launched.
This review approves design and proof sufficiency only; all open milestone proofs remain owed.

`./dev check-docs` passed: 677 Markdown files, 3,379 local links, 30 module records, crate policy and
current design hashes. `git diff --check` passed. Only this review file was changed; no captures or milestone
verdicts were written.

DESIGN OK
