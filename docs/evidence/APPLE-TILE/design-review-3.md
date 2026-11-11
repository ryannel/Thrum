# APPLE-TILE design review: D0.2 scope correction

11 November 2026. Reviewed integrated revision `019f849d519ba0978f31971d55da1780888023bb` using
`orchestrate:review-milestone` 1.6.2, “Reviewing a design”, and Thrum's engineering review rules.
**No blocking findings.** D0.2 now assigns the native witness to M10 without weakening its exactness or M0's
ordinary-image proof. This is approval of the revised design criteria, not acceptance of M0, its captures, or
any implementation, visual or performance claim. The loop owns approval records and unblock transitions.

## Commission and review scope

I read the previous review of `2514d8240876` first. It contained **no Must Fix findings** and two Suggestions;
both remain open as described below. Its approval did not notice that D0.2's statement required the native
witness before its first implementation in M10. This review explicitly checks that conflict.

The accepted record's `briefs` array contains one task, read in full:

> Resolve APPLE-TILE M0's scope conflict: the builder blocked because done-when item D0.2 requires a native witness that the plan assigns to M10 (see the M0 blocked note and docs/evidence/render/apple-tile-m0/README.md at 03bc878a). Make M0's criteria and M10's consistent so M0 can be claimed on what it can prove, without weakening what V1 needs.

I also read the module brief and relevant project-purpose, architecture and consumer definitions. The required
outcome of this revision is a usable ordinary comparison baseline before renderer changes, followed by a real,
exact native/twin composition proof at the native boundary milestone. It must preserve ordinary scene content,
lighting, mirrors, history and fallback/lifecycle behavior, with separate target and performance qualifications.
It does not commission a new rendering mechanism or an early native provider.

Inspection checklist for this revision:

1. Compare every changed item field against the previously reviewed definitions; check for weakened proof or removed items.
2. Trace M0's retained baseline, identity, present and omission requirements through later ordinary-image comparisons.
3. Trace the excluded native witness to a mandatory later item, including first-fixture feasibility, actual native writes,
   exact history/twin comparisons, negative controls and adverse lifecycle cases.
4. Check the revised plan's ordering and G4 handoff for a missing prerequisite or new dependency cycle.
5. Revisit both previous Suggestions and distinguish design approval from builder evidence and loop state.

A structured comparison of the two done-when files found no added or removed items and exactly one changed item
field: **D0.2.statement**. Its `ask` and every other item's fields are unchanged. M0's state/block reason also
differ from the prior revision; these are loop records, not new proof requirements. I read the entire plan diff,
the relevant unchanged design mechanisms/comparison contract, D0.1–D0.5, M10–M12's retained obligations, the native
boundary decision and G4's consumer sequence. The module design and brief are byte-for-byte unchanged. This is
an incremental design review, not a renewed implementation audit of all intervening source changes. I read the
M0 evidence README as context for the scope conflict; I did not independently validate its claimed results.

## Findings, ranked

### 1. Suggestion, verified: the plan incorrectly exempts changed criteria from design review

**Location:** `docs/design/modules/APPLE-TILE-PLAN.md:6` and `:155–157`.

The plan says no review is owed for the D0.2 correction because the design text is unchanged. The changed
criterion is part of the reviewable design: its old statement required a native witness in M0, and its new
statement explicitly excludes that work. The review skill requires changed items to be checked even when the
mechanism document is unchanged.

Replace that exemption with this review's revision and outcome, leaving milestone acceptance and unblock with
the loop. This is a Suggestion because the loop did commission this independent review and the revised criterion
is sound; the erroneous process sentence has not removed a required behavior or bypassed this actual check.
It should not become precedent for skipping review of later criterion changes.

### 2. Suggestion, verified, carried forward: D12.5 still misnames the native handoff

**Location:** `docs/design/modules/APPLE-TILE-DONE.json:421`; compare
`docs/design/modules/APPLE-PRESENTATION-PLAN.md:90–102`.

D12.5 still asks whether “G4 M11 native handoff” avoids the final cooked-simulator dependency. The consumer
identifies **G3t M11 → G4 M8** as the native handoff; G4 M11 is the final paired proof and deliberately needs
G3t M12. Correct the name without removing that final qualification dependency. As in the previous review,
this is advisory: the actual sequence and producer handoff agree, and D0.2 does not alter them.

### 3. Suggestion, verified, carried forward: the frame-runtime maturity scope retains an obsolete dependency

**Location:** `docs/design/maturity.json:528`; compare its G3t scope at `:588` and
`docs/design/modules/APPLE-TILE-PLAN.md:20–33`.

The frame-runtime scope still describes Mac M3 as transitively blocked on T2 “Chrome M1/simulator M2”. The
active G3t chain uses delivered T2 M0/M1 and diagnostic simulator function before M12, without T2 simulator M2.
Align the summary with the active dependency chain while preserving COOKED-DRAW's final cooked-simulator
ownership and T2 M2's stronger capture qualification. This remains an advisory status inconsistency, not a new
cycle or a missing prerequisite introduced by this revision.

## Assessment of the correction

**M0 remains judgeable and discriminating.** D0.2 at lines 25–29 retains byte-identical same-target portable
repeats and full-resolution pre-change scene-linear/submitted SDR baselines. Its unchanged question still requires
build/compiler/shader/content/sample/exposure/format identity, H=1, both SDR/sRGB present controls, non-1:1 extents
and letterboxing. Phase mismatch, missing contribution, a nonzero weak light in an overflowing cluster, mirror
slot 5060 and missing night stars must each fail exact comparison with native-size differences. A coarse-grid
comparison, silently refreshed baseline, widened tolerance or CPU float oracle cannot satisfy it. D0.1 still
requires actual route/cut/variant coverage and separate realtime host-time motion. Removing the native witness
from M0 does not remove any of these ordinary-path requirements.

**The native proof is deferred to its existing owner, not deleted.** D10.1 still requires the small same-submit
native copy/select fixture versus the prepared twin as the first implementation step, before route integration,
and returns failures to boundary design review. D10.2 still requires exact RGBA16F output/history bytes through
route, reset and replacement; distinct preinitialized histories; submission-selected state; and actual native
writes. No-op native work, concealed WGSL production, stale source/history and omitted reset must fail. D10.3 and
D10.4 retain completion, discard/loss and iOS lease/background proofs. D11.2 retains exact diagnostic recovery
against its twin, and D12.3 retains the separately labelled final diagnostic. A build that never implements the
native witness can finish M0, but cannot finish M10 or the module.

**Ordinary and diagnostic comparisons remain separate throughout.** The unchanged design at
`APPLE-TILE.md:244–270` requires later ordinary paths to match both paired recipes and the retained pre-change
portable baseline; diagnostic output matches its own stateful twin. D10.2 explicitly disables the diagnostic in
ordinary runs. The correction therefore cannot legitimize a new ordinary baseline or an omitted scene contribution.
It also preserves the prior review's resolution of the unsupported CPU/GPU arithmetic premise.

**Ordering is consistent.** The plan retains M0 → M1 → M2 → M3 → M10 → M11 → M12, with M0 creating the
ordinary controls, M10 first building the native fixture and M11 completing its provider handoff. G4's native
consumer still waits for G3t M11. M0 no longer needs a capability that its own successors must build. No new
reverse dependency, native-provider requirement before M10, or loss of final qualification follows from the edit.
The existing M0 `blocked` reason describes the now-corrected conflict; changing that loop state is outside this review.

The plan's proposal to reuse retained evidence does not itself accept it. The builder must establish applicable
source/input identity and rerun required claiming checks; the milestone reviewer must independently judge the
proof under the review skill. This design verdict supplies no inherited pass for those observations.

## Revision and verification record

| Artifact | SHA-256 |
| --- | --- |
| APPLE-TILE.md | `d90de51acfddeb2fda00e9b763dc5b13be3d0f5ddedff520a78f24e069bc1e4f` |
| APPLE-TILE-BRIEF.md | `f8b69b720d1d1729627eda41600a29eac58a866d726e8664f741116fef11a8a7` |
| APPLE-TILE-PLAN.md | `9fc8365c1c705786c820bffc7c4e3f5299694cbecdb1d346061af80445e1eb68` |
| APPLE-TILE-DONE.json | `04867f495f07c2f62856d3e1529eca94a13caf4a217d15fad803632b9e86782a` |
| APPLE-TILE-ACCEPTED.json | `a18e52ff1e7bae4eb153d5886bbf9603348ae463b04f9ef8d5810b66e373a7cc` |

`RUSTC_WRAPPER= ./dev check-docs` passed before and after writing the report. The wrapper override avoids the
external compiler cache; it skips no repository check. No GPU, browser, simulator, OS-window, motion or performance
proof was run, and no images or video were judged. No agents were launched. Only this review file was changed;
no captures were needed, no definitions/status records were edited, and no commit was made.

DESIGN OK
