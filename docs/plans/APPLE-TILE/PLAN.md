# APPLE-TILE plan

The Apple tile recipe and backend dataflow (gate G3t), governed by
[the boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) and C04, C06, C07, C08 and C12.
[Design](DESIGN.md) · [brief](BRIEF.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

Milestones run in the order M0, M1, M2, M3, M10, M11, M12; the IDs are kept for consumers, and M4–M9 were removed
as deferred work. Read [Sequence and dependencies](DESIGN.md#sequence-and-dependencies) and
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) before starting any milestone.

<a id="m0-proof-route-counters-and-capture-probe-pending"></a>

## M0 A repeatable offscreen valley proves the comparison path: done
[review](../../evidence/APPLE-TILE/M0-review.md)

## M1 The valley selects a recipe and stores only actual future inputs

Criteria: D1.1–D1.5 in the done-when file.

- [x] Add truthful registry/override, per-variant storage and ready fallback; preserve COOKED-DRAW facts/selection. Prove Mac/Chrome exact bytes and simulator diagnostic function.
- [x] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## M2 The existing frame lifecycle runs through one split graph

Criteria: D2.1–D2.5 in the done-when file.

- [x] Merge graph and late surface binding; preserve refusal/loss/background/history rules. Complete attended OS-window matched-state reference comparisons and functional recovery.
- [x] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## M3 G4 can replace the pre-tone linear input without changing SDR output

Criteria: D3.1–D3.5 in the done-when file.

- [ ] Expose one colour/exposure input with exact identity substitution; keep tone/intermediate/present arithmetic. Migrate pass-name timing and prove attended window function.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## M10 A stateful native segment composes inside one submission

Criteria: D10.1–D10.4 in the done-when file.

- [ ] Start with exact same-submit copy/select fixture and twin. Then prove route/history, held lifetime, discard/reset and diagnostic simulator background race.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## M11 Native providers survive replacement and expose truthful timing

Criteria: D11.1–D11.3 in the done-when file.

- [ ] Use bounded extent fixtures and prepared twins for replacement/rejection/errors, complete generation retention and explicit clock-domain timing.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## M12 An integrated record states qualified wins and remaining targets

Criteria: D12.1–D12.6 in the done-when file.

- [ ] Rerun retained mechanisms and full workload, actual final Mac window, exact Chrome baseline, and COOKED-DRAW-dependent simulator function; record sustained costs and owed qualifications.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

Stop rules, for every milestone:
- D2.5, D3.5 and D12.3 need attended setup: an unlocked active Mac session and Screen Recording permission for the
  OS capture process. Missing setup leaves those rows open while unblocked work proceeds; never automate a grant.
- M12's cooked simulator scene waits on `COOKED-DRAW`; never build a substitute.
- Never silently refresh a baseline, and never widen a comparison metric in place of exact bytes.
