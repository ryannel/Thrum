# PLATFORM-RUNTIME-MAC M2 handoff, 12 November 2026

Work is stopped at the owner's direction for the orchestrate upgrade and lead replacement. The next lead
runs `unblock` and then `run` using the upgraded orchestrate script before a builder resumes. This session
did no build work, changed no milestone claims and started no reviewer or merge. It began on branch
`build/mac-m2` at `3b070bc2abf1ee095cec6dc4969453510be15872`, with a clean worktree. This handoff is the only
new file; its containing commit is the handoff checkpoint. No background job was launched by this session.
The sandbox refused `ps`, so this session did not establish whether any earlier session's process remains;
there is no evidence here of a stuck holder and no process was stopped.

## Finished work and the controlling review

The [module log](../../plans/PLATFORM-RUNTIME-MAC/LOG.md) records the prior implementation, witness tooling,
window proofs and historical motion/cadence collection. The local M2 claim was rebased to
`b4d96b98f801e2cec280e6a568053c292656fff6`. The
[integration stop record](../render/mac-m2-presented-20261111/integration-stop-20261112.json) confirms that
its full `./dev check` failed in `cooked_material_gpu`; the GPU-locked focused rerun passed eight tests.
That focused success neither replaces the full check nor establishes the original failure's cause.
The stop record says `not_integrated` for that attempt; it does not establish current main ancestry.

The later [independent review](M2-review.md), on
`53137ce1506a37ec27c04082701d0e9ac1061bd9`, is FAIL: M2.1, M2.2 and M2.3 could not be judged.
M2.4's reproducible witness/refusal tooling and M2.5's full-interval recording method pass that review.
The review built and cooked the current assembly, but runtime and full-check requests waited for GPU
without starting; the observed panel was locked/asleep. Those requests are recorded in
[review run status](M2-review/run-status.json), not as completed proofs.

The confirmed identity mismatch is decisive: the retained recordings used executable SHA-256
`aa1591522962933b55ec1f347cc538880f5eb14b7ea227bf4adcb7f00b585507`, reconstructed from `10287802`.
The review built `3a23303c64f776260f01e92818feee9c81fbf1933cb4aecd28e6bff054ee13cd` and found 96 of
406 recorded source files changed, including render/present/capture paths. See
[build identity](M2-review/build-identity.json). The old recordings prove their recorded binary only.
The log and integration-stop statements that attended proof was complete and no new recording was owed
are superseded by this review. **An attended re-record of the current build, with the GPU free of other
builders, is still owed.** There is no confirmed current renderer stutter or diagnosed renderer defect.

`DONE.json` currently leaves M2 in progress and M2.1–M2.3 open; M2.4/M2.5 retain pass. Do not promote them
from old evidence or treat this stop as completion. Current-build live-window qualification and the full
mandated check remain owed alongside fresh motion and matched cadence arms.

## Tried approaches and limits to preserve

The log and [presented-time evidence](../render/mac-m2-presented-20261111/README.md) retain the trials.
Game Performance raw swap/VSync and derived displayed-surface tables were empty even after restoring OS
logs. Developer mode as their cause was unverified; no security change is justified by that suspicion.
A ring-window configuration exceeded its deadline; the smaller three-instrument ring-window configuration
completed but retained only the last 1.075 seconds. Neither qualifies the full marked interval. The successful
configuration is non-windowed Deferred, Blank plus Metal Application, Display and Points of Interest,
with presented-handler `at-time` rows and validated same-process conversion. Callback times, delta columns,
movie PTS, engine pixels and GPU completion do not substitute for OS-reported presented intervals or photons.

Preserve `tmp/mac-m2/presented-commission-20261111.json`: four of six recordings were consumed, leaving two,
not a new allowance. The older `tmp/mac-m2/trace-commission.json` is exhausted separate history. The review
consumed no recording and granted none. Disk preflight and missing-binary refusals did not consume recordings.
Keep source masters, trace bundles, manifests, cooked content, executable pins and both counters. Use fresh
output directories for new evidence, without resetting the counter. Do not retry an unchanged failed hypothesis.

The owner's words are recorded in [ACCEPTED.json](../../plans/PLATFORM-RUNTIME-MAC/ACCEPTED.json),
`milestones.M2.human_feedback`: M2.2 at `2026-11-12T06:57:44Z`, then M2.5 at
`2026-11-12T06:57:53Z`, each says exactly "The screen is unlocked if you need it". This is the owner's
recorded feedback, not a current panel observation or acceptance of the older executable. Attended setup
needs the Mac unlocked and built-in panel awake throughout; no Developer mode change, Instruments
authorization, Screen Recording grant or play is needed. This background session cannot show approval prompts.
M2.1's offscreen console observations remain witness-only and must not become a lock admission gate.

## Resume work and proofs still owed

After the next lead unblocks and runs the upgraded loop, first reconcile the current build/content identity
with the collector. `scripts/qualify_mac_m2.py` currently pins presented arms to
`tmp/mac-m2/final-window-20261110/identity.json`; merely passing a new `--binary` is refused. Replace that
historical pinning mechanism with current same-build provenance while preserving historical evidence and the
recording counter. This fix was identified, not implemented in this handoff session. Rebuild and cook the
assembly being qualified, then rerun M2.1's denied attributed permission-preflight offscreen motion workflow.
Its commands and evidence are in [10 November evidence](../render/mac-m2-20261110/README.md) and the
[review reproduction record](M2-review/reproduce.md). Retain exact source/build/content identity for every arm.

For M2.3 run the real-window grid/valley replacement and OFF scenarios, inspect the active window and native
originals, package sources with `python3 scripts/package_mac_m2_window.py --output <fresh-arm>`, and establish
same-build pattern correspondence, submission/lease joins, pending retirement, stale refusal and recovery.
The [historical window runbook](../render/mac-m2-20261110/RUNBOOK.md) describes the scenarios; its old binary,
old trace path and `land` instruction are historical and must not be copied as current qualification.

For M2.2 collect fresh matched ON/untraced, ON/traced and OFF/traced arms on M2.1's current build, content,
recipe and extent. Inspect complete host-time 1× ON movies and native sources; qualify cadence only with OFF.
Investigate every missed refresh and retain completeness, conversion error, load and observer-cost evidence.
After fixing provenance, the current collector's command shape is below; it is not runnable proof yet:

```sh
LOOP=/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.0.0/skills/orchestration/build/scripts/agent_loop.py
python3 "$LOOP" lock gpu -- python3 scripts/qualify_mac_m2.py --binary <current-pinned-binary> --store <matching-store> --presented-minimal --presented-deferred --presented-arm <on-untraced|on-traced|off-traced> --output <fresh-output>
python3 scripts/mac_m2_presented.py --input <fresh-arm>/presented.xml --arm <fresh-arm> --output <fresh-arm>/witness.json
python3 -m unittest discover -s scripts -p 'test_mac_m2*.py' -v
python3 "$LOOP" lock gpu -- ./dev check
python3 "$LOOP" lock gpu -- python3 "$LOOP" check --module PLATFORM-RUNTIME-MAC --milestone M2 --worktree .
python3 "$LOOP" lock gpu -- python3 "$LOOP" merge --worktree .
```

Resolve placeholders against the new assembly; do not execute them literally. Use the upgraded script path
if the upgrade replaces 2.0.0. The [presented runbook](../render/mac-m2-presented-20261111/RUNBOOK.md) supplies
bounded recording/export settings and witness/source/cost commands, but its claim that no new recording is
needed is superseded. All GPU captures, timing runs, real-Metal checks and native playback must hold `gpu`;
normal lock waiting is not a failure. Schedule the attended re-record when other builders have released it.
Preserve the two remaining recordings for the traced arms; do not infer further diagnostic authorization.

Only after the fresh proofs pass should the builder update item status/evidence and M2's state, commit,
and obtain `MERGED` from the mandated merge command. Any output-affecting changes brought by integration
require affected proofs again on the integrated assembly. The lead starts the independent review.
