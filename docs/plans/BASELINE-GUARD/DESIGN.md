# Baseline guard: a change to another module's exact reference images is found on main and traced to its commit

11 November 2026, planned again 15 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#exact-baselines) · [the brief](../VISION.md). C12 in
[CONTRACTS](../../design/CONTRACTS.md) governs evidence.

## The requests

The owner's planning task of 11 November 2026 ended:

> APPLE-PRESENTATION WP1's jitter change broke APPLE-TILE's exact baseline comparisons, and nothing caught it
> before it merged (.orchestrate/evidence/APPLE-TILE/WP2-review.md MF1). Say in ARCHITECTURE.md or the affected designs
> how a module's change that alters another module's exact baselines gets caught before landing.

WP1 answered it with a verifier and a gate that refused any covered change without a proof of its exact content.
That gate stalled the build ([the report](../../../.orchestrate/evidence/BASELINE-GUARD/WP2-replan/proof-regime.md)), and on
15 November 2026 the owner approved the report's regime:

> Ok, let's get the script and skill changes done then once we've got orchestrate to where it needs to be, let's
> get the lead on Thrum working.

> We want all these fixes in place before I head to bed tonight.

In the lead's session that day the owner said "Go ahead" to a gate that only warns, an hourly verifier on main,
`gpu: 2, chrome: 1`, this module planned again, and an unintended image change sitting on main for about two
hours. So "caught before landing" now reads: a moved image is found on main after the merge, within about two
hours, and traced to its commit; what is still refused before a merge is a change to the references themselves.

The lead's task for this plan, of 15 November 2026:

> The script now does the sentinel's shared run, retry, bisect and naming the culprit. Re-plan BASELINE-GUARD
> down to what remains Thrum's own (for example the verify command and the CPU ownership guard), and decide what
> happens to its in-flight WP2 on branch build/baseline-guard.

## Purpose and scope

[The module list](../ARCHITECTURE.md#exact-baselines) states the rules. This module builds the parts of them
that are Thrum's own.

**Built:**

1. **The verifier** (WP1): `python3 scripts/exact_baselines.py verify --full`. From a clean checkout it draws
   fresh output on the Mac and in Chrome and compares it with every registered baseline, byte for byte.
2. **The answer for a commit** (15 November, with the ruling): `python3 scripts/exact_baselines.py verify`. It
   draws nothing. It asks the main-branch check `exact-images` whether a passing run contains the commit.
3. **The gate as a notice** (15 November, with the ruling): `./dev check` prints a warning for a covered change
   that no proof names, and never refuses.

**Commissioned (WP2):**

4. **The reference guard.** `./dev check` refuses a commit that changes a registered reference, its comparison
   or its registration unless the same commit records the refresh in the registry. It needs no GPU.
5. **One proof.** The run on main is the only proof of the exact images, so the local proof file and the gate
   that read it are removed.
6. **Scratch that stays bounded** in the checkout the run on main uses.

**Not commissioned:** changing any baseline, comparison or tolerance; toleranced baselines such as the G7
route's; visual judgement. Scheduling the run on main, its single flight, its retry, the bisect and naming the
first failing commit are the orchestrate script's, and nothing here repeats them.

**Owns:** the registry file, the verifier and the reference guard. The retained references, and the right to
refresh them, stay with each baseline's owner module.

## Research

- **Slow tests run after the merge, shared, with culprit finding.** Google's continuous testing runs the
  expensive suites on the trunk at intervals and bisects a failure to the change that caused it, because testing
  every change before submission does not converge at that rate of change (Memon and others, "Taming
  Google-Scale Continuous Testing", ICSE-SEIP 2017). Chromium's tree works the same way, with a revert as the
  first response. Cheap tests stay before the merge, as in Rust's bors and merge queues.
- **A pinned file changes only with its recorded digest.** Lockfiles (`Cargo.lock`, `go.sum`) and Bazel's
  `sha256` on a fetched archive make a change to pinned bytes fail until the recorded digest changes in the same
  commit, which turns a silent change into a reviewed one. `cargo build --locked` also refuses a lockfile that
  has lost an entry the manifest still needs, so dropping a pin is no way round it. The reference guard is
  both, for the references and for the set of baselines.
- **A narrower proof key was designed and is set aside.** [Later work](#later-work) has the record.

## Mechanism

### The registry

One file, `.config/exact-baselines.json`, lists each registered baseline: its identity, its owner, its retained
reference files, how a fresh candidate is produced and compared, and the files that make the comparison. The
table in
[the module list](../ARCHITECTURE.md#exact-baselines) is the same registry for people; the two must agree. Its
`covered_paths` are the paths whose change the notice reports.

### The run on main

`ORCHESTRATE.md` declares the main-branch check `exact-images`, whose command is `verify --full`. The script
runs it on main's tip in a checkout of its own, kept between runs so the builds stay warm, holding the `gpu` and
`chrome` locks. The verifier:

1. refuses a tree with uncommitted changes, and reports low disk space as its own named failure;
2. builds the release game, the cooker and the Wasm page;
3. cooks the showcase valley fresh and runs `APPLE-TILE`'s native matrices unchanged;
4. captures the four Chrome `fixed` runs and compares each with its retained manifest, record by record, by the
   SHA-256 of each frame's decoded bytes;
5. exits non-zero on any difference, naming the baseline, its owner and the comparison;
6. writes a record of the run, pass or fail: the revision, the content key of the covered paths, the binary and
   Wasm hashes and the cooked root.

It takes every input from the checkout and its own fresh outputs. One run takes about 16 minutes
([58 passing records](../../../.orchestrate/evidence/BASELINE-GUARD/WP2-replan/proof-regime.md#measured-this-session): native
matrices 410 s and 369 s, the four Chrome sets about 140 s).

### The answer for a commit

`verify` hands the question to the script, `main-check exact-images`, and returns its exit status. It passes
once a passing run exists on a main commit that contains HEAD, waiting for the run under way or starting one.
It fails when main fails at or before HEAD, and on a branch that main does not contain. It exits 75 when the
run has not finished inside the wait the script allows a check. It is the `check` of an item in most modules'
done-when files, with no `lock`: an item lock held while it waits would take a `gpu` slot from the run it waits
for.

### The reference guard

With the gate only a notice, one hole opens. A builder whose change moves an image can make the run on main pass
again without fixing it, in three ways: regenerate the reference, loosen the comparison, or take the baseline
out of the registry. Rule 1 forbids each, but only a reviewer would find it, later. The guard makes each fail at
the merge.

**What is protected.** For each baseline the registry lists its **comparison files**: the files that choose
which comparisons run, read the reference and decide whether a candidate equals it. Today they are:

| Baseline | Comparison files |
|---|---|
| Every one | `scripts/exact_baselines.py`: the verifier, which reads the registry, runs each comparison and turns the results into the exit status |
| `apple-tile-native` | `scripts/exact_baselines_native.py`; the matrices `scripts/apple_tile_m1.py` and `scripts/apple_tile_m2.py`; `scripts/apple_tile_m0.py`, which both are built from; and `scripts/compare_v1_pixels.py`, whose `compare` is the byte equality both call |
| The four Chrome baselines | `scripts/apple_tile_m1_compare_browser.py` |

The list is written out in the registry, not traced from imports. A comparison file that gains an import has
itself changed, so the change arrives with a refresh, and that refresh is where the list gains the new file.

A baseline's **protected files** are every tracked file of its reference and every one of its comparison files.
Its **definition** is every field of its registry entry but the pin and the refresh: its identity, owner,
reference path, candidate, comparison, record count and list of comparison files.

**The pin and the refresh.** For each baseline the registry holds:

- a **pin**: a SHA-256 over the definition and over the path and Git object ID of every protected file, at the
  commit;
- a **refresh**: the ruling in the module list that allowed the last change, by its anchor, and the module and
  milestone that made it.

A baseline that a ruling ends leaves the list of baselines and enters the registry's **retired** list, as its
identity, its owner and a refresh.

**What the guard checks.** `python3 scripts/exact_baselines.py guard`, a step of `./dev check`, reads the
commit's tree, its registry and the registry at the merge base with main. It fails, naming the baseline, its
owner and rule 1, when:

1. a pin differs from the one computed at the commit. It names each protected file and each definition field
   that differs from the merge base;
2. a protected file has an uncommitted change;
3. a pin differs from the merge base's and its refresh does not;
4. a baseline is new since the merge base and has no refresh;
5. a baseline of the merge base is no longer among the baselines and is not retired;
6. a refresh, a retirement's included, names an anchor the module list does not hold.

So the registered set changes by four recorded routes and no other: a protected file changes, a definition
changes, a baseline is added, a baseline is retired. Each is a commit that edits `BASELINE-GUARD`'s registry and
names a ruling, which is one place for a reviewer to look and an edit rule 1 allows only under that ruling.
`covered_paths` is outside the pins: it selects no comparison, and only keys the verifier's record.

**Check that it can work.** [A model of the pins](../../../.orchestrate/evidence/BASELINE-GUARD/WP2-replan/guard-scope.json) at
`81138094` has the figures; `guard_scope.py` beside it reproduces them.

- *The culprit that regenerates.* Main fails, and the named module regenerates the reference to pass.
  `./dev check` fails at its merge: the pin differs. To merge it must write a refresh that names a ruling. If
  there is none, it has no anchor to name. Writing one into the module list is a builder changing a planner's
  file, in plain sight of the milestone's reviewer.
- *The culprit that unregisters.* It removes `chrome-night-scene-linear` and leaves the rest alone. Every
  remaining pin is still right, so checks 1 to 4 pass. Check 5 fails and names the baseline: the merge base
  registers it and the commit neither registers nor retires it.
- *The culprit that redefines.* It lowers a Chrome baseline's record count from 481 to 480, or points a
  reference at another file. The definition is in the pin, so that baseline's pin differs and check 1 fails.
- *The culprit that loosens the comparison.* It edits the equality in `scripts/compare_v1_pixels.py`: the native
  pin differs. It makes the verifier skip an entry: all five pins differ.
- *A planned refresh* (`ATMOSPHERE` WP9, `CULLING` WP3). The ruling exists. The builder regenerates, runs
  `verify --full` on the candidate, updates the pin and the refresh, and merges all of it in one landing.
- *A comparison file edited by its owner* changes the pin, so the owner updates it in the same commit and names
  the ruling or the rule that makes the file its own. The verifier is a comparison file of all five baselines,
  so `BASELINE-GUARD` pays five pins for an edit of its own. The files listed above changed in ten commits in
  the first two weeks of November, five of them the verifier's.
- *What it reads.* Git object IDs from the commit's tree and two versions of one registry file: no build, GPU
  or browser. The tree and all five pins take under a tenth of a second, over 57 protected files for the native
  baseline and 3 for each Chrome one.
- *The founding case.* The jitter change edited exact tests in crates, not a registered reference. The guard
  does not see that; the run on main does, because every pixel moved.

### Scratch

Each run leaves its record and logs under `tmp/baseline-guard/` in the run's checkout, and a failed run keeps
its captures for diagnosis. With up to 48 runs a day and about a quarter failing before any retry, that grows
without limit, and the verifier itself refuses to start under 16 GiB free. So at its start the verifier keeps
the records of its latest runs and the captures of its latest failed run, and removes older ones. The numbers
kept are the builder's, set from the measured size of a failed run.

## What other modules use

- `python3 scripts/exact_baselines.py verify`: the answer for a commit. The item text in their done-when files
  does not change.
- `python3 scripts/exact_baselines.py verify --full`, under the `gpu` lock: a planned refresh's run on its
  candidate. `GAME-SPLIT` WP4 runs it with `THRUM_GAMES` pointing at nothing.
- A verifier record's `content_key`, `revision` and hashes, which the rulings on a frame that varies under load
  and a fixed capture that stalls compare.
- The registry's pins and refreshes, which a planned refresh edits for its own baseline under its ruling, and
  which an owner edits when it changes one of its comparison files. Its retired list, for a baseline a ruling
  ends.
- After WP2, `gate`, `proof_path` and the proof file are gone. The scripts that called them
  (`.orchestrate/evidence/WATERS/finish_m1.py`, `.orchestrate/evidence/SEA/qualify_m2_baselines.py` and others like them) are
  retained evidence of a procedure the ruling ended, and nothing runs them.

## What it does not stop

- An exact test in a crate changed to absorb another module's output. Rule 1 forbids it, a reviewer finds it,
  and the run on main still catches the moved image.
- A builder that writes a refresh naming a ruling that does not allow its change. The guard checks that the
  anchor exists, not what it says.
- A helper that a comparison file calls from a file other modules edit daily. Today that is one function:
  `read_png` in `scripts/run_v1_showcase.py`, which decodes the ten 8-bit native references before
  `compare_v1_pixels.py` compares them. That file is `SHOWCASE`'s and changed in 19 commits in the first two
  weeks of November. Pinning it would ask a refresh of each, and a refresh written by reflex guards nothing. A
  decoder changed to return other pixels is a breach of rule 1 that a reviewer finds.
- A candidate that is not drawn. The files that only produce a candidate (the game, the cooker, the capture
  scripts) are what the run on main tests, and no pin covers them. One changed to emit the reference instead of
  drawing is a reviewer's to find.
- A change outside the repository: macOS, the GPU driver, Chrome, Node or `wasm-bindgen`. Main then fails for a
  cause no commit holds, and the script cannot tell. Its bisect trusts the last recorded pass without running it
  again and halves the commits after it, so when every one of them now fails it names the first commit after
  that pass. The name is a lead, and the lead reads the failure log before acting on it. When the cause is
  outside, the baseline's owner refreshes under rule 1, cause recorded.
- An unintended image change on main for up to about two hours, which the owner accepted.

## Cost

One verifier run of about 16 minutes on main per interval, shared by every builder, at most one start every
30 minutes: about 4 to 6 hours of one `gpu` slot and of the `chrome` slot on a busy day. A landing pays the
guard in `./dev check`, a tree read and two reads of the registry. The times are recorded, not budgeted.

## Risks

1. **The run on main fails for a cause that is not a commit** (a flake that holds twice, low disk, a Chrome
   update). Every milestone's verifier item then fails at its claim until a run passes. The lead reads the
   run's log first; the script retries once and the scratch rule keeps disk from being the cause.
2. **Two regressions overlap while main is red.** The second shows only after the first is fixed, on the next
   run.
3. **Holding `chrome` for the whole run** makes Chrome items wait up to 16 minutes. If `flow` shows a queue,
   [later work](#later-work) narrows it.
4. **A claim that arrives just after a run starts** waits for the next one, up to 30 minutes and its 16. A
   retry on top can pass the 55 minutes the script waits, and the claim's check then fails as pending; the next
   attempt finds the result.

## Later work

- **A two-minute tier of the native matrix** for routine runs on main, with the full matrix daily. It would cut
  a run by about two thirds and needs a ruling with `APPLE-TILE`, whose sampling it is.
- **`chrome` held only for the Chrome captures**, taken by the verifier around those steps, with the settings
  line reduced to `lock gpu`.
- **A run that skips unchanged content.** The record already holds the content key; a run on a tip whose key
  equals the last pass's could answer at once.
- **The PNG decoder inside the pins.** Moving `read_png` into the comparison's own file would close the first
  helper listed under [what it does not stop](#what-it-does-not-stop). It changes `APPLE-TILE`'s comparison and
  `SHOWCASE`'s script, so it needs a ruling with both.
- **The narrower key.** WP2 as first planned took the files of crates the game does not link out of the proof's
  key, and nine commits of it are on the branch `build/baseline-guard`, unmerged. It spared 18 of 204 lapses.
  With no proof asked before a merge it saves nothing, so it is not merged. It would matter again only for the
  skip above. Its design is this file at `4101a7cf`, and the fuller derived key it withdrew is recorded there
  with [the planner's measurements](../../../.orchestrate/evidence/BASELINE-GUARD/WP2-plan/README.md).

## Limits on builders

- Never refresh, regenerate or edit a file under a registered baseline, except under a ruling in the module list
  that names your milestone.
- No tolerance, nearest-frame search, retry inside a run or skipped record, for any reason.
- Do not change `scripts/apple_tile_m0.py`, `apple_tile_m1.py` or `apple_tile_m2.py`,
  `scripts/compare_v1_pixels.py`, the byte mode of the Chrome comparator, `scripts/capture_v1_browser.mjs` or
  `examples/v1-game/web/build.sh`. They belong to other modules.
- In the registry, add to the existing baselines only their comparison files, pins and refreshes. Their other
  fields, and which baselines are registered, stay as they are.
- Do not change what `verify` without `--full` answers or the item text other modules carry, and do not edit
  `ORCHESTRATE.md`: the settings line is the planner's.
- Tests of the guard and any demonstration of a merge run against a scratch repository whose remote is a local
  bare repository, never the real remote.

## Proof and builder handoff

`mac-metal` and `chrome-webgpu`. A `verify --full` run a builder makes holds the `gpu` lock. Scratch goes in
`tmp/baseline-guard/`; selected evidence in `.orchestrate/evidence/BASELINE-GUARD/`. The guard's tests build small Git
repositories of their own, so they run offline and need no GPU.
