# CREATOR log

- 2026-11-14 Repeated both native exact-image matrices and all four Chrome sets after the SEA
  rebase: exact in 942.78 seconds at `2b4d6589`. The full gate then found one unit-test Clippy
  assertion style error. The equivalent diagnostic assertion at `480a6436` passes full workspace
  Clippy and all four profile-window tests. No runtime code changed.
  [M2 evidence](../../evidence/CREATOR/M2.md) retains the latest records.

- 2026-11-14 Passed both native exact-image matrices and all four Chrome sets at `1e0d687b`
  in 973.39 seconds. Rebase comparison found no changes to M2's measured producers or consumers.
  The next gate found two upstream M10 handoff links into unmerged evidence; their worktree scope
  is now explicit. [M2 evidence](../../evidence/CREATOR/M2.md) retains the verifier record and the
  runner for the unchanged final M2 checks under one GPU reservation.

- 2026-11-14 Completed M2.2 and M2.4's hardware Chrome proofs. Added geometry after frame 656
  raised forward GPU time from 0.393 to 13.369 ms; unsupported, hidden and lost-device rows
  showed reasons. The off arm stopped timestamp work while a new presented submission completed,
  and on cleared pre-switch numbers. [M2 evidence](../../evidence/CREATOR/M2.md) retains both complete
  passing runs. Exact-image verification and integration remain pending.

- 2026-11-14 Inspected and retained M2.7's hardware Chrome pages at default width, 480 px and off.
  Their 20 ordered profiler rows match the native starter and remain readable; all off rows state
  their reason. The parent run later failed an incorrect automatic-recovery assertion, so its
  complete check is not claimed. [M2 evidence](../../evidence/CREATOR/M2.md) records the corrected
  proof and the two browser checks still queued on the shared GPU.

- 2026-11-14 Resumed M2 and retained the earlier session's real Metal timing-switch test and native
  panel measurements. Three world-pause and material-edit pairs exceed their unchanged spreads;
  the four profile-window tests pass after rebasing. Browser and final integration checks remain
  pending. [M2 evidence](../../evidence/CREATOR/M2.md) distinguishes coupled changes and measured cost.

- 2026-11-14 Implemented M2's shared table and profile window, shell bindings and the frame-host
  timing-demand switch. CPU unit tests and native/Wasm compilation and linting passed; device,
  browser and panel proofs remain unverified at this checkpoint. [Builder evidence](../../evidence/CREATOR/M2.md).

- 2026-11-10 Planned from [the module list](../ARCHITECTURE.md). The earlier G6 slices were built before the loop and
  are recorded in the G6 decision and its evidence; this plan covers only what closes the gate.
- 2026-11-10 [Design review 1 at `4bbe44a4`](../../evidence/CREATOR/design-review-1.md): DESIGN CHANGES, one Must
  Fix (F1). Review 2 found it unresolved, because no revision had been made between them.
- 2026-11-11 Revised after design review 2. M1 is unchanged. M2 and M3 items changed.
  [Full entry](../../evidence/CREATOR/log-entries.md#entry-1).

- 2026-11-11 Built and claimed M1. `./dev starter --web` and `./dev scene <folder> --web` build the shared
  40-line scene, cook and watch its assets, and serve the kit page. Shared model setters drive its DOM controls.
  Hardware Chrome 157 proofs cover submitted Sun edits, mesh and texture publications, refused generation
  retention, the same running page/scene, parameter refusals, tab visibility and a 480 px window. The builder
  inspected all six page states. [M1 evidence](../../evidence/CREATOR/M1.md) records the source and artifact hashes.
  The final milestone gate has four passes, including `./dev check`. One earlier V1 allocation-suite failure
  cleared in focused exact-binary runs and full reruns, with its cause unconfirmed and retained in the evidence.
  Mac and Wasm linting include the native test gate and the kit's Wasm test target. Independent review is the lead's.

- 2026-11-11 Reclaimed M1 after [review 1](../../evidence/CREATOR/M1-review.md). F1 is reproduced by the strengthened
  edit case before the fix. Refusals now expire when a later generation draws, while transport errors retain their own
  lifetime. Generation 5 restores the pre-refusal pixels on the same running page.
  [Full entry](../../evidence/CREATOR/log-entries.md#entry-2).

- 2026-11-11 Plan reshaped for orchestrate 1.7.0: status lines removed, `Ownership` moved to the design,
  the review 1 work list moved to [evidence](../../evidence/CREATOR/M1-review-1-work.md), and the open
  assumptions moved from this log to the plan's `Open for the owner`. No item or scope changed.
