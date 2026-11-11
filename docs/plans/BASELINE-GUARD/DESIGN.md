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
rules; this module builds the parts of them that do not exist.

## What went wrong, and why a rule alone would not fix it

Read: [APPLE-TILE M2's review](../../evidence/APPLE-TILE/M2-review.md),
[APPLE-PRESENTATION M1's first review](../../evidence/APPLE-PRESENTATION/M1-review-1.md),
[this design's first review](../../evidence/BASELINE-GUARD/design-review.md), `land` in the build skill's
`agent_loop.py`, and `scripts/apple_tile_m0.py`, `apple_tile_m1.py`, `apple_tile_m2.py`,
`apple_tile_m1_compare_browser.py` and `capture_v1_browser.mjs`.

- `APPLE-TILE`'s exact comparisons need a release build, a cooked store and, for Chrome, a browser. Nothing runs
  them automatically.
- A builder merges with `land`. Under one lock it fetches, rebases, runs `./dev check` and pushes, and it does not
  hand control back between the rebase and the push. `./dev check` is the only gate before main, and it did not
  hold the comparison.
- So `APPLE-PRESENTATION` M1's jitter change merged with 316,181 of 800,000 pixels different at one checkpoint.
- `APPLE-TILE` M2 was then claimed on a binary built before its rebase, so its own evidence missed the break too.

A written rule tells a builder what not to do. Only a failing gate stops the push. The first design put a few Mac
checkpoints in `./dev check` and left the full matrix, and all of Chrome, to a rerun the builder was asked to make
after `land`'s rebase. `land` gives no chance to make it: a night, surface-format or Chrome regression would have
been pushed first and found afterwards. This design replaces that.

## Purpose and scope

**Commissioned:**

1. **The verifier.** One command that, from a clean checkout, draws fresh output on the Mac and in Chrome at the
   current commit and compares it with every registered exact baseline. On a full pass it writes a proof.
2. **The gate.** `./dev check` refuses a commit that changes the covered paths unless a proof exists for exactly
   that content. `land` runs `./dev check` on the rebased commit, so nothing unproven is pushed.

**Not commissioned:** changing any baseline, comparison or tolerance; toleranced baselines such as the G7 route's;
visual judgement of any kind; the small image test inside `./dev check` that the first design proposed. The gate
makes it unnecessary, and with it goes the open question of whether a debug build draws the release bytes.

**Owns:** the registry file, the verifier, its proof records and the gate. The retained references, and the right
to refresh them, stay with each baseline's owner module.

## Research

- **Test the commit that will be pushed.** Graydon Hoare's rule for Rust's bors, to keep a repository that always
  passes its tests by testing each change as integrated before the branch advances, and the merge queues built on
  it ([Jane Street's account](https://blog.janestreet.com/making-never-break-the-build-scale/)). `land` already has
  this shape, holding a lock from rebase to push. What it lacks is the expensive test.
- **Reuse a result only for identical inputs.** Build systems such as Bazel key a cached result by a hash of the
  action's inputs ([remote caching](https://bazel.build/remote/caching)). The proof below is keyed the same way,
  so a rebase that changes an input invalidates it and one that does not keeps it.

## Mechanism

### The registry

One file lists each registered baseline: its identity, its owner, its retained reference files, how a fresh
candidate is produced and compared, and the **covered paths**. The table in
[the module list](../ARCHITECTURE.md#exact-baselines) is the same registry for people; the two must agree.

Covered paths err wide: `crates/`, `examples/v1-game/`, `scripts/`, the workspace manifest, lockfile and toolchain
file, and each baseline's own retained files. A planner narrows them only with evidence that a path cannot reach
the game's native or Wasm build.

### The content key and the proof

The **content key** of a commit is a digest of the Git object IDs of the covered paths at that commit. Two commits
with the same key have byte-identical covered files, whatever else differs between them.

A **proof** is the record the verifier writes when every registered comparison passed on a clean tree. It names
the content key and is kept untracked in the worktree. It is the only thing the gate accepts.

### The verifier

One command, from a clean checkout:

1. refuses a tree with uncommitted changes, and reports low disk space as its own named failure;
2. builds the release game with the features the owners' scripts need, builds the Wasm page with
   `examples/v1-game/web/build.sh`, and cooks the showcase valley fresh;
3. runs `APPLE-TILE`'s native matrices (`apple_tile_m1.py`, `apple_tile_m2.py`) unchanged;
4. captures the four Chrome `fixed` runs with `capture_v1_browser.mjs` and compares each with its retained set;
5. exits non-zero on any difference, naming the baseline, its owner and the comparison;
6. on a full pass writes the proof, with the revision, the content key, the native binary's hash, the Wasm
   artefact hashes and the cooked root.

It takes every input from the checkout and its own fresh outputs. It never reads another task's scratch.

### The Chrome references

The first design registered a Chrome baseline whose comparator needs both captures unpacked, and the reference's
481 frames exist only in one builder's scratch. They are not needed. Each retained manifest already holds, for
every one of its 481 records, the frame's identity fields and the SHA-256 of its decoded bytes. Four are committed:

| Set | Retained manifest (under `docs/evidence/render/`) |
|---|---|
| Day, submitted SDR | `apple-tile-m0/chrome-baseline/prechange-chrome-fixed-manifest.json.gz` |
| Day, scene-linear | `apple-tile-m1/chrome/chrome-prechange-linear-3/manifest.json.gz` |
| Night, submitted SDR | `apple-tile-m1/chrome/chrome-prechange-night-sdr/manifest.json.gz` |
| Night, scene-linear | `apple-tile-m1/chrome/chrome-prechange-night-linear/manifest.json.gz` |

The comparison for each set: both captures complete and `fixed`, the same number of records, the identity fields
the owner's comparator already checks equal record by record, and each fresh frame's decoded bytes, hashed again
from the file, equal to the retained digest. Equal SHA-256 is treated as equal bytes. This is exact, uses the
owner's bytes as committed, and regenerates nothing.

It is a reference-by-digest mode of `scripts/apple_tile_m1_compare_browser.py`, added by this module's builder
under [the ruling in the module list](../ARCHITECTURE.md#exact-baselines). The script's byte mode and
`APPLE-TILE`'s items do not change.

### The gate

A step of `./dev check`, with no GPU and no browser:

1. If the commit has no covered-path change that main lacks (the difference from the merge base with the remote
   main branch to the commit, over the covered paths), it passes. Main's own covered content was proved when it
   landed.
2. Otherwise it computes the commit's content key and passes only if a proof names that key. If not, it fails and
   names the verifier command. If the base cannot be resolved, it fails.

There is no flag or environment variable that skips it.

### Check that it can work

- **The sequence the review gave.** A builder proves its branch. Another builder lands a covered-path change.
  `land` rebases the first branch, so its covered content, and its key, change. `./dev check` finds no proof for
  the new key and fails, and `land` does not push. The builder runs the verifier on the rebased commit, which now
  draws the combined result, and lands again. A rebase that brings in only other paths keeps the key and the proof.
- **A fix with no plan** goes through the same `land` and the same gate.
- **Chrome and night are inside it**, because the proof exists only when every registered comparison passed.
- **Would it have caught MF1?** The jitter change moved every rasterised surface, so the native matrix fails and no
  proof is written. M1 shows it with that mutant, a night-only mutant and a Chrome-only mutant.
- **Are the bytes reproducible?** The native matrix matched on two separately rebuilt binaries on 11 November
  ([the MF1 recheck](../../evidence/render/apple-tile-m2/MF1-recheck.md): 38 of 38 and 4 of 4, then all 42 again
  after another module integrated). Chrome's four sets each matched all 481 frames against a fresh build at
  `APPLE-TILE` M1. Chrome has not been rerun since, so the builder's first step is the verifier's comparisons on
  untouched main.
- **Can the builder, the reviewer and the loop reach it?** A Codex agent asks to launch Chrome outside its
  sandbox, and the loop reruns a milestone's checks itself, both as the existing browser checks already do. The
  gate needs neither a GPU nor a browser, so `land` can always run it.

## What it does not stop

- A push of a commit that `./dev check` did not pass: `land --no-check`, or a plain `git push` after a rebase
  with no check. Rule 1 of the module list forbids both for a covered-path change.
- A forged proof. The proof is a local file, as `./dev check` is local code.

Both are caught after the fact by the claim-time item of rule 3, whose command a builder cannot change, and by
the next builder's verifier run, which fails on main.

## Cost

A landing that changes covered paths pays one verifier run on its final content: two builds, a cook, the native
matrices and four Chrome captures of 481 frames. A rebase that brings in another covered-path change pays it
again. Landings that touch only other paths pay the gate's key computation. The times are recorded in M1, not
budgeted. The scripts need 16 GiB free.

## Risks

1. **A macOS, driver or Chrome update changes the bytes**, and no covered-path change can land. The sign is that
   untouched main fails the verifier. The baseline's owner refreshes it under rule 1, with the cause recorded. The
   gate is not disabled.
2. **Chrome's references no longer match main.** Found by the builder's first step. It ends `BLOCKED` with the
   comparison, and the refresh is `APPLE-TILE`'s.
3. **The gate is edited by the builder it stops.** Rule 1 forbids it and a reviewer checks it.
4. **A verifier run nears the loop's one-hour limit on a check.** M1 records the time. The fallback is a
   comparison-only entry to the owners' scripts, proposed to `APPLE-TILE`, never fewer comparisons.
5. **Two builders invalidate each other's proofs.** Each landing forces at most one more run on the other, and
   whichever lands first makes progress.

## Proof and builder handoff

`mac-metal` and `chrome-webgpu`. GPU runs hold the `gpu` lock. Scratch goes in `tmp/baseline-guard/`; selected
evidence in `docs/evidence/BASELINE-GUARD/`. Mutants run in scratch and are never merged. The demonstration of
`land` refusing runs in a scratch clone whose remote is a local bare repository, never against the real remote.

## Open questions

| Question | Settled by |
|---|---|
| Do Chrome's four retained sets still match untouched main? | The builder's first step in M1 |
| How long do one verifier run and one gate check take? | M1 records them |
