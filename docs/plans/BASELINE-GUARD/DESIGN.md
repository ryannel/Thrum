# Baseline guard: a change that alters a registered exact image is refused before it reaches main

11 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#exact-baselines) · [the brief](../BRIEF.md). C12 in
[CONTRACTS](../../design/CONTRACTS.md) governs evidence.

## The request

The owner's planning task of 11 November 2026, word for word:

> Plan the modules CULLING and EFFECT-GRAPH from docs/plans/ARCHITECTURE.md: write each one's DESIGN, PLAN and
> done-when file. Both depend on APPLE-TILE M2, which is in its second review now, so builders can start them once
> it passes. Everything else is waiting on APPLE-TILE M2 or M3, so these are the next work. Keep each first
> milestone small enough to start right after APPLE-TILE M2 lands. Also: APPLE-PRESENTATION M1's jitter change broke
> APPLE-TILE's exact baseline comparisons, and nothing caught it before it merged
> (docs/evidence/APPLE-TILE/M2-review.md MF1). Say in ARCHITECTURE.md or the affected designs how a module's change
> that alters another module's exact baselines gets caught before landing.

This module answers the last two sentences. [The module list](../ARCHITECTURE.md#exact-baselines) states the
rules; this module builds the one part of them that did not exist.

## What went wrong, and why a rule alone would not fix it

Read: [APPLE-TILE M2's review](../../evidence/APPLE-TILE/M2-review.md),
[APPLE-PRESENTATION M1's first review](../../evidence/APPLE-PRESENTATION/M1-review-1.md), the build skill's `land`
command, and `scripts/apple_tile_m0.py`, `apple_tile_m1.py` and `apple_tile_m2.py`.

- `APPLE-TILE`'s exact comparison against its M0 baseline lives in two scripts. They need a release build and a
  cooked store, and nothing runs them automatically.
- A builder merges with `land`, which rebases, runs `./dev check` and pushes. `./dev check` is the only gate before
  main, and it did not hold the comparison.
- So `APPLE-PRESENTATION` M1's jitter change merged with 316,181 of 800,000 pixels different at one checkpoint.
  The tests in `./dev check` that did notice (paused frames stopped being identical) were edited by the same
  builder.
- `APPLE-TILE` M2 was then claimed on a binary built before its rebase, so its own evidence missed the break too.

A written rule tells a builder what not to do. Only a failing gate stops the merge.

## Purpose and scope

**Commissioned:**

1. **The guard.** On the Mac, `./dev check` draws the game's default output and compares it, byte for byte, with
   each registered exact baseline.
2. **The verifier.** One command runs every registered baseline's full verify matrix from a clean checkout, for
   builders before a claim, the loop at a claim and reviewers.

**Not commissioned:** changing any baseline, comparison or tolerance; a guard for Chrome inside `./dev check`
(no browser runs there; rule 3 of the module list covers Chrome at the claim); toleranced baselines such as the G7
route's; visual judgement of any kind.

**Owns:** the guard's tests, the registry file they read and the verifier. The retained bytes, and the right to
refresh them, stay with each baseline's owner module.

## Mechanism

### The guard

- **It runs the game's own default path.** The guard builds its frames through `examples/v1-game`'s host, scene
  source, route and capture tap, the same path the baseline was captured through, with the capture settings the
  baseline records and nothing else. That is the code the jitter change edited. A guard that set up its own view
  would have passed that day.
- **It compares exactly.** Submitted SDR bytes and scene-linear bytes, for the portable and the Apple tile recipe,
  against the retained files. No tolerance, no search for the nearest of several frames, no retry, and no skip: on
  a host without Metal the guard is not compiled, like the other Metal tests, and where it is compiled a missing
  device fails it.
- **It is small.** It guards a subset of the baseline's checkpoints, chosen so that both of the comparisons the
  review's MF1 failed on (`mirror-light` and `approach`) would fail again. The full matrix stays the authority and
  runs through the verifier.
- **It says what to do.** A failure names the baseline, its owner, the checkpoint and the number of differing
  pixels, and points at [the rules](../ARCHITECTURE.md#exact-baselines).
- **It reads a registry.** One file lists each guarded baseline: its identity, owner, retained files and guarded
  checkpoints. A second baseline is a new entry and its bytes, with no new test code. The table in the module list
  is the same registry for people; the two must agree.

### The verifier

One command builds the release game with the features the owner's scripts need, cooks the showcase valley fresh,
runs each registered baseline's full verify command, and exits non-zero on any difference. It refuses a dirty tree,
builds the binary itself so the binary is the commit's, and writes the revision, the clean-diff hash and the
binary hash into its record. It adds no comparison of its own; it wraps the owners' scripts.

### Check that it can work

- **Would it have caught MF1?** The jitter change moved every rasterised surface by a sub-pixel offset, so any
  checkpoint with geometry differs in hundreds of thousands of pixels. M1 proves it with that mutant.
- **Are the bytes reproducible?** The M0 matrix matched exactly on two separately rebuilt binaries on 11 November
  ([the MF1 recheck](../../evidence/render/apple-tile-m2/MF1-recheck.md): 38 of 38 and 4 of 4 comparisons, then all
  42 again after another module integrated), so the output is deterministic on this Mac.
- **Does the gate's debug build draw the same bytes as the baseline's release build?** Expected, because the GPU
  work is identical and Rust does not reorder floating-point arithmetic between profiles, but not yet shown. The
  builder tests this first. If the bytes differ, the cause is diagnosed, and only then may the guard keep its own
  retained set, captured by the gate's build at a commit where the verifier passes, with that derivation recorded
  in the registry. Loosening the comparison is not an option.
- **Can the test get a cooked valley?** `./dev check` already cooks content inside its Metal tests.

## Cost

The guard adds a cook of the showcase valley and a few frames at the route's render extent to every `./dev check`
on the Mac. The added time is recorded, not budgeted. If it becomes the slowest GPU test in the gate, the answer is
fewer checkpoints, never a weaker comparison.

## Risks

1. **A macOS or driver update changes the bytes**, and every merge fails. The sign is that untouched main fails.
   The baseline's owner then refreshes it under rule 1, with the cause recorded; the guard is not disabled.
2. **The guard is edited by the builder it stops.** Rule 1 forbids it, a reviewer checks it, and the claim-time
   matrix, whose command a builder cannot change, still runs.
3. **The subset misses a change the full matrix catches.** Rule 3 covers it at the claim.
4. **Disk.** The owners' scripts refuse to capture with under 16 GiB free. The verifier reports that as its own
   named failure, so it is not mistaken for a changed image.

## Proof and builder handoff

`mac-metal` only. GPU runs hold the `gpu` lock. Scratch goes in `tmp/baseline-guard/`; selected evidence in
`docs/evidence/BASELINE-GUARD/`. The mutants are run in scratch and never merged. After this milestone the registry
table in the module list names the verifier as each baseline's full verify command.

## Open questions

| Question | Settled by |
|---|---|
| Do the gate's debug build and the baseline's release build draw identical bytes? | The builder's first experiment in M1 |
| How long does the guard add to `./dev check`? | M1 records it |
