# Cu source CPU cost and exact work removal

3 November2026. **Isolated CPU candidates, not adopted or live.** Baseline is current **c3e0ebee9680337c**; [exact identity](baseline-c3e0-identity.json). No source physics, dt=.25, weather, admission, native control-grid pitch, optical payload, shader or lighting change is selected here. Canonical design/maturity and actual browser/GPU work belong to root.

The ZIP retains exact original/publication/frame source, copied evaluators, all raw outcomes/profiles, selectors, finite native-point/packed-pose/bounds checks, the rejected first cache, two-file patch, and original reports/handoffs. Mutable code is copied; native R16 payloads are not duplicated. [Restore specification](restore-spec.json) pins the archive and every member. Four fields reference the [component proof](../cloud-component-proof-2026-11-03/README.md), and material noise references the [geometry proof](../cloud-geometry-operators-2026-11-03/README.md), by encoded and decoded SHA. Their original CC0 provenance remains in the referenced packages.

## Recover and inspect

```
python3 docs/evidence/cloud-source-cost-2026-11-03/verify.py
python3 docs/evidence/cloud-source-cost-2026-11-03/restore.py --output tmp/cloud-source-cost-review
python3 docs/evidence/cloud-source-cost-2026-11-03/verify.py --restored tmp/cloud-source-cost-review
```

Restoration refuses an existing destination, safely extracts694 small members, verifies every byte, decodes each unique external payload once and hardlinks its destinations. It reconstructs the full baseline c3e0 served identity and both sibling evaluator layouts. No ignored tmp source or original worktree is needed. [Recovery receipt](RECOVERY.json) records a fresh relocation/hash verification and both restored receipt validators; **no model replay, benchmark, browser or GPU was run during packaging**. Existing tests/results are retained, not relabeled as newly executed.

After recovery, read `cloud-system-cpu-cost/REPORT.md`, `cloud-system-cpu-fast-proof/PUBLICATION-RESULT.md` and `FRAME-RESULT.md`. `combined-source.patch` reconstructs exact final `frame/src/source/model.js` +`math.js`; final hashes are e21f9f5163f0a183136ce02a2d2cc9d6d5f6459943acfc5fa3254ba8ec8ae70e and7f85fe25b759b7e1d37770b3e4bcea13199ef49dbb66362b0d79c621556bd862. Baseline model675edc8..., publication-only087c84f8..., clockee3033f... are distinct retained identities. The copied main lab uses the publication-only candidate; final combined CPU source is under `frame/`, never claimed browser-loaded.

## Findings and limits

| Study | Saved outcomes | Result |
|---|---:|---|
| Original cost |27:18complete,6stopped,3diagnostics | Dry publication repeats slot-invariant work; wet bounds allocate repeatedly |
| Publication-only |66:55complete,11stopped | At16copies dry first publication8.4–10.0× faster; wet steady no consistent material win |
| Frame vs publication |66:56complete,10stopped | At16copies wet presentation1.3–1.4× faster; dry1× cadence arithmetic saves12.8–15.4%; dry16× total has no consistent gain |

Each matched matrix uses separate fresh Node workers, alternating pair order, one tick-only and two normal repetitions per1/2/16copies, plus adverse64 cases. Every slower repetition and stop remains. Source copies are isolated kernel workloads with repeated IDs/cells, **not a valid combined sky**. Host reports Node24.16.0/darwin arm64/AppleM3Max; no M2/device admission. Parent coordinated quiet CPU windows. Source steps, node counts, GC, RSS, process CPU and wall distributions remain separate in raw receipts. Profiling is separate and sampled, not exact allocation/residency. Do not compound gains across the two different matched windows into an end-to-end number.

Publication hoists exact per-slot retained-ID/terminal selection and bulk-converts F64→F32; raw source truthiness preserves positive-underflow allZero semantics. Frame caching registers only deeply frozen owned birthQ→the existing exact numerical dual, with fresh identity per birth/reset; mutable/arbitrary Q falls back. Public descriptors retain separate frozen Q arrays. Bounds keep all eight corners and operand order; no row-extrema reassociation. Per-stage differential covers48selectors,19000steps/57005tuples,102births/72retirements/66slotreuses. Native1600points/466positive and packed poseF32/bounds match exactly; source8/render8 pass perstage. [Root's independent publication rerun](independent-publication-differential.json) has the same exact counts/source hashes and is distinct from the author's receipt. [Independent final CPU code review](independent-review.md) found no issue; its small no-write checks and residual limits are separate from the author's suites. These finite tests do not certify a continuous field or actual GPU execution.

Persistent controls remain943296CPU B/copy; hypothetical GPU controls+poses314432+768B/copy plus shared64B. Actual residency/driver/fullsky costs are not measured. Cadence arithmetic uses N(4tick+4first+56same) at1× and N(64tick+60first) at16×, not measured playback/frame pacing. All64 normal cases stop; occasional64 tick-only completions do not qualify source+presentation capacity.

Early224MiB/hard256MiB are sampled process-pair guards. Original timing max241.281MiB includes a disclosed late warmup check; publication225.234MiB and frame226.984MiB peaks remain adverse outcomes. The first pose-keyed cache failed its224MiB differential guard; source/log retained, exact triggering RSS/source time and hard-limit outcome unknown. The revised immutable-birth-Q cache passed without raising limits. Copied inherited test outputs inside the rejected tree are explicitly labeled ancestry, not passes for rejected bytes.

[GPU query review](GPU-QUERY-REVIEW.md) is a separate read-only review of e6456b622215cdd7, with no actionable finding,13existing Node tests independently rerun and14mocked restoration cases. It is not authorship or hardware proof of that GPU candidate and excludes the reviewer's authored baseline field. Subsequent root hardware checks failed the strict bit gate for both e645 and c2 despite passing the broader CPU envelope. [Both exact failed GPU screens](../cloud-query-screen-2026-11-03/README.md) remain retained; root deferred GPU adoption and did not run a quiet speed benchmark. These outcomes do not relabel the earlier read-only review as hardware qualification. Actual common consumers,1×/16× experience, integrated CPU cost, fullsky and production acceptance remain separate gates.

Future correctness/benchmark commands are retained in the recovered READMEs. They overwrite result files and can consume CPU; preserve this package, choose a fresh copy, and coordinate timing before running them. Root reviews the final patch separately. No cleanup of active tmp work or Git operation was performed.
