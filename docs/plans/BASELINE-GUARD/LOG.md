# BASELINE-GUARD log

- 2026-11-11 Created from the owner's task, quoted in the design: "Say in ARCHITECTURE.md or the affected designs
  how a module's change that alters another module's exact baselines gets caught before landing." The rules are in
  the module list. This module exists because `land` runs only `./dev check` before it pushes, and the exact
  comparison was not in it.
- 2026-11-11 Planner's decision: a separate module with one milestone and no `needs`. Adding the guard to
  `APPLE-TILE` M3 would have put it on the critical path and behind attended items, and changed definitions a
  builder is about to start on. `T2-CAPTURE`'s design gives recipe comparison to `APPLE-TILE`, so it was not put
  there.
- 2026-11-11 The planner did not run the guard's checks: they need a release build and the GPU. Each check was
  parsed as shell and the done-when file validated with the loop's own validator.
- 2026-11-11 [Design review](../../../.orchestrate/evidence/BASELINE-GUARD/design-review-1.md): changes requested, two Must Fix.
  MF1: `land` rebases, checks and pushes without handing control back, so the full matrix could only run after a
  failing push. MF2: the Chrome comparator needs 481 reference frames that exist only in another builder's scratch.
- 2026-11-11 Design revised for both. MF1: the verifier writes a proof keyed by a digest of the covered paths, and
  `./dev check` refuses a covered-path change without a proof for exactly that content, so `land` cannot push it.
  M1.3 and M1.4 are the acceptance items the review asked for. MF2: the retained Chrome manifests already hold a
  SHA-256 for every frame, so four sets (day and night, SDR and scene-linear) are compared by digest from a fresh
  Wasm build, with no bulk reference and no regeneration; ruled in the module list with `APPLE-TILE`.
- 2026-11-11 S1 not applied, because its subject is gone: the small image test inside `./dev check` is no longer
  built, so there is no debug-build reference set to approve or maintain. S2 applied in M1.4: patches, base
  revision, commands and logs are kept, and a failure tells a changed image from an unreadable reference.
- 2026-11-11 The planner did not run the checks, which need the GPU, Chrome and work that does not exist yet. Each
  was parsed as shell and the done-when file validated with the loop's own validator.

- 2026-11-11 M1 builder: untouched main `1d35b9a6` passed both unchanged native matrices (38 + 4
  exact comparisons) and all four Chrome sets (481 records each). Fresh release/Wasm builds and task-owned
  cook only; [preflight records](../../../.orchestrate/evidence/BASELINE-GUARD/M1/preflight/provenance.json) retain hashes.
  Implemented registry, clean-tree verifier, digest mode and content-key gate; proof and integration experiments
  are in progress. Coverage also includes `.cargo/` (design review S3), `tools/` and the registry itself; the
  architecture table records the same wider inputs. No retained reference or owner matrix was changed.

- 2026-11-11 M1.1–M1.3 pass with [target evidence](../../../.orchestrate/evidence/BASELINE-GUARD/M1.md). The final verifier
  passes 42 native comparisons and 1,924 Chrome records at covered key `f6b5c8fa`; all seven focused tests pass.
  Successful owner-published captures are archived incrementally after disk space dropped during shared work.
  Final-version mutant refusals, the third timing and the milestone check remain in progress.

- 2026-11-11 M1.4 passes on the final verifier with incremental capture archiving. The actual loop refuses all
  four scratch cases, with a fixed local remote after each refusal. Native difference metrics, failed manifests,
  patches, base, commands and logs are [retained](../../../.orchestrate/evidence/BASELINE-GUARD/M1/refusals-final/report.json).
  M1.5 and the complete milestone check remain.

- 2026-11-11 M1 claimed with all five items passing. The rebased source includes APPLE-TILE M3 and passes
  42 native comparisons and all 1,924 Chrome records at covered key `7e7c9a3c`. The required milestone check
  passes all three automated commands, including the profile Check. Its explicit `--timeout 3600` follows the
  design's one-hour allowance after the standalone command's 600-second default interrupted an earlier run.
  [M1 evidence](../../../.orchestrate/evidence/BASELINE-GUARD/M1.md) records all four final-version land refusals, three full
  verifier timings with load, three gate timings, preserved references and byte-mode semantics. Lead review remains.

- 2026-11-11 The claim landed as `232d27dc` through the checked integration command. Retained a 7,811-byte
  source patch from published checkpoint `9a29a8c9` to reconstruct the refusal experiment's covered content,
  because its recorded commit was rebased. An isolated index reproduced key `f6b5c8fa` exactly.
  [Recovery record](../../../.orchestrate/evidence/BASELINE-GUARD/M1/refusals-final/base-source.json).

- 2026-11-11 Claim-time M1.1 failed in the loop checkout. M2 published a completed
  portable route with 14.8 GiB free and immediately ran its next 16 GiB preflight before the polling reaper.
  Reopened M1.1 pending requalification. Added a verifier-owned adapter that calls the owner's run function and
  archives completed captures synchronously after its original report publisher returns. Owner source files,
  comparisons, preflights and return codes are preserved. Nine focused tests pass, including immediate-next-capture
  release and failed-source preservation. [Failure record](../../../.orchestrate/evidence/BASELINE-GUARD/M1/disk-repair/loop-failure.json).

- 2026-11-11 The repaired M1.1 passes all 42 native comparisons and 1,924 Chrome records. Nine focused tests
  pass. The repository Check hit its one-hour timeout after repeated device waits from the per-binary Cargo
  runner used by this builder. Changed that verification runner to one GPU lease around the complete xtask
  Check, with the inner runner removed to prevent recursive locking. Resuming the failed full profile Check.

- 2026-11-11 Reclaimed M1 after the synchronous publication fix. The repaired verifier passes all 42 native
  comparisons and 1,924 Chrome records in 1021.79 seconds. M1.3's nine tests pass, and the full
  repository Check passes after resuming it under one GPU lock. The earlier aggregate timeout and the successful
  component results are retained without rewriting their outcomes. [Requalification](../../../.orchestrate/evidence/BASELINE-GUARD/M1/disk-repair/requalification.json).

- 2026-11-12 M1 Done: [the review](../../../.orchestrate/evidence/BASELINE-GUARD/M1-review.md) passed all five items.

- 2026-11-15 The lead's task, approved by the owner that day: "the proof key covers exactly the inputs that can
  change a registered exact image or the verifier's verdict ... and nothing else. Derive the set rather than
  hand-list crates ... The gate must still fail closed". M2 added. M1's listed key changed at 33 of main's
  commits on the night of 14 to 15 November; ten of them changed a file a verified build or comparison reads
  ([measurements](../../../.orchestrate/evidence/BASELINE-GUARD/M2-plan/README.md)).

- 2026-11-15 Planner's decisions for M2. Covered paths are derived from Cargo's resolved graph and the scripts'
  references. A proof also sets aside the Rust files no build read, by Cargo's dependency records, and the gate
  accepts it for a later commit that differs only in those. Rejected: a naming rule for test files (a list by
  another name); watching or denying file reads (the owners' scripts hash the whole tree into their records);
  a key over the resolved graph in place of the manifest and lockfile files (spares one commit of 33).

- 2026-11-15 Survey of other proofs keyed to a source identity, as the task asked
  ([findings](../../../.orchestrate/evidence/BASELINE-GUARD/M2-plan/README.md#other-proofs-keyed-to-a-source-identity)).
  `SIM-2D`'s cost records are drawn wider than their inputs: a hand list of the right five packages, with the
  whole workspace manifest, the lockfile, test files and untracked files. `CULLING` M3's campaign names M1's
  listed paths and narrows them by hand, with the same three exclusions M2 derives. Nothing else compares a
  retained record with the sources. No other module's file was changed.

- 2026-11-15 M1's plan notes moved: its stop rules that still hold are in the design's "Limits on builders".
  The module list's exact-baseline rule 2 and table now describe the derived key. Tried with the loop's
  `check --items-only`: M2.1 to M2.4 and M2.7 fail before the work, none errors. M2.6's check, the verifier, was
  not run: the `gpu` lock had builders queued for over an hour, and the command is M1.1's, unchanged.

- 2026-11-15 [Design review](../../../.orchestrate/evidence/BASELINE-GUARD/design-review.md) of the M2 revision: changes
  requested, one Must Fix. MF3: a script's read through a computed path, a folder it lists or a Rust source
  read as data escapes a key built from imports and quoted paths, so a proof could outlive a changed input.
  S4: M2.1's "a document" should not invite a builder to drop a linked package's documents from the key.

- 2026-11-15 Design revised for MF3. The verifier does every build, the cook, every capture and every
  comparison in a verification root, a scratch tree that holds only the covered paths, so an uncovered file
  does not exist for anything it runs and a read of one fails before a proof is written. Unread sources are
  deleted from the root once known, and the root's content is hashed from disk against the key. Every
  workspace package's manifest is now covered, because Cargo needs them all there. M2.8 is the fixture the
  review asked for; M2.3 and M2.6 changed to match. S4 taken in M2.1 and M2.5.

- 2026-11-15 Rejected for MF3: reporting opened files from Python and Node (misses what Git lists, native
  readers and tests for absent files); builds in the worktree with only comparisons in the root (leaves the
  page build's reads open); a sparse checkout (Git lists absent files, which one capture script then reads).
  Left open, as an assumption in the plan: a Cargo build script reading another package's unread source.

- 2026-11-15 The root was tried without the GPU
  ([trial](../../../.orchestrate/evidence/BASELINE-GUARD/M2-plan/README.md#a-root-that-holds-only-the-covered-paths)): Cargo
  loads it with empty files at 56 target roots and resolves the same graph, the cooker builds and cooks there,
  and the owners' scripts load. No matrix or capture has drawn from a root.

- 2026-11-15 M2's checks tried by running each command: the five test-class checks, M2.8's included, and
  `covered` fail before the work and none errors. M2.6's check, the verifier, was not run: it needs the `gpu`
  lock and is M1.1's command, unchanged.

- 2026-11-15 [Design review](../../../.orchestrate/evidence/BASELINE-GUARD/design-review.md) of the verification root: changes
  requested, MF3 again and nothing else. The root fails a read a script needs, but a script that reads a file
  only if it is there, or finds it by listing a folder, carries on without it, and a later change to the file
  keeps the proof. The review has no Suggestions.

- 2026-11-15 The lead's task: "MF3 has now been raised twice. Settle it this time: the reviewer accepts either
  covering optional and enumerated script inputs in the key, or making any input the key cannot resolve block
  the proof from being published (fail closed). Pick whichever is simpler to make provably complete, and keep
  excluding the unrelated crates and scripts."

- 2026-11-15 Planner's decision: cover. Every tracked file is a covered path unless it is in a package Cargo
  does not link, is a document or is a script nothing reaches, so no rule depends on how a script finds a file.
  Imports and quoted paths now decide only which scripts are reached. M2.1 is written again for that rule, M2.9
  is new and holds the review's three cases, and M2.8, M2.7 and M2.3 follow. Refusing was not taken as the main
  rule because nothing shows the verifier that a step looked for a file: a missing file answers "no", macOS
  reports lookups only to root, a denied read is swallowed by an ordinary `except OSError`, and access times
  move on the first read only.

- 2026-11-15 Refusal is kept where it cannot be argued around. A tripwire stands in the root for each withheld
  script, leaves a mark when a step runs or imports it, and the key holds the script's path. Without it a
  module imported by a computed name would be the same hole in another file type.

- 2026-11-15 Left open, as assumptions in the plan and an entry in the design's "What it does not stop": a step
  that takes input, without needing it, from an unlinked package, an unlisted document or an unread source.
  Closing it means covering those files, which the task excludes for crates and which for `docs/` would lapse
  every proof. Also considered and not taken: `scripts/` covered whole (28 more lapses in 493 commits against
  6), a folder rule for a reference's siblings in `docs/`, and the non-Rust files of unlinked packages
  ([estimate](../../../.orchestrate/evidence/BASELINE-GUARD/M2-plan/README.md#covering-by-place)).

- 2026-11-15 M2's checks tried by running each command: the six test-class checks, the new `ScriptInputTests`
  among them, and `covered` fail before the work and none errors. The loop's `check --items-only` waited over
  ten minutes on the `gpu` lock for M2.6, with builders holding it, and was stopped; M2.6's check is M1.1's
  command, unchanged. `./dev check-docs` reports only the 18 missing links in `APPLE-PRESENTATION`'s log.

- 2026-11-15 [Design review](../../../.orchestrate/evidence/BASELINE-GUARD/design-review.md) of covering by place (commit
  `76daae92`): changes requested, MF3 a third time and nothing else. Data a script reads only if present, or by
  listing a folder, is still lost when it lies in a place the rules leave out, such as `.orchestrate/evidence/`.

- 2026-11-15 The lead's task, a re-plan of M2: "Take the smaller step. Keep every path the exact-image key in
  scripts/exact_baselines.py covers today, and subtract only workspace crates and examples that are outside the
  build closure of what the verifier builds (v1-game and thrum-cook). Work out that closure from `cargo metadata`
  when the check runs; don't keep a hand-written list. ... a crate the closure can't classify stays in the key,
  so the check fails closed. ... The fuller derived key is out of scope here; record it as possible later work
  if re-proving remains a bottleneck." The design quotes it whole.

- 2026-11-15 M2 planned again from that task. The key is M1's listed paths, less the files of workspace
  packages Cargo links into neither built package; their manifests stay. The verification root, unread
  sources, document and script rules and tripwires are withdrawn, and the design's "Later work" records them.
  All nine items were written again: M2.1 to M2.8 now.

- 2026-11-15 Planner's decisions. The gate uses the commit's own graph only: the linked packages follow from
  files that are covered, so no graph of the merge base is needed. Added beyond the task: the verifier checks
  Cargo's dependency records after its builds and writes no proof if a build read a file outside the key,
  because a linked crate can include a file from an unlinked crate's folder with no dependency. Left open, as
  an assumption: a script or the game reading such a file as data at run time.

- 2026-11-15 Measured at `5dbc2c10` ([README](../../../.orchestrate/evidence/BASELINE-GUARD/M2-plan/README.md#the-smaller-step)):
  nine of 23 packages linked; 73 of 738 listed files leave the key. Since 11 November M1's key changed at 204
  of main's commits and the smaller key at 186, so it spares 18, about one lapse in eleven. A fixture
  workspace and the records of two real verifier builds bear out the build-read check.

- 2026-11-15 M2's checks tried by running each command: the five test-class checks and `covered` fail before
  the work and none errors. The loop's `check --items-only` waited seven minutes on the `gpu` lock for M2.7,
  behind queued builders, and was stopped; M2.7's check is M1.1's command, unchanged. `./dev check-docs` passes.

- 2026-11-15 Owner, on the proof regime
  ([report](../../../.orchestrate/evidence/BASELINE-GUARD/M2-replan/proof-regime.md)): "Ok, let's get the script and skill
  changes done then once we've got orchestrate to where it needs to be, let's get the lead on Thrum working."
  and "We want all these fixes in place before I head to bed tonight." In the lead's session the owner said "Go
  ahead" to a gate that only warns, an hourly verifier on main, `gpu: 2, chrome: 1`, this module planned again,
  and an unintended image change sitting on main for about two hours.

- 2026-11-15 Planned again on the lead's task (quoted in the design). Built by the planner with the ruling
  [proofs run once, on main](../ARCHITECTURE.md#proofs-run-once-on-main): `gate` warns and never refuses;
  `verify` asks the main-branch check `exact-images` and draws nothing; `verify --full` is the verifier, which
  that check runs on main. `scripts/test_exact_baselines.py` passes with the gate's tests rewritten and
  `MainCheckTests` added. The full verifier's code was not changed and was not run by the planner.

- 2026-11-15 M2 replaced. The earlier M2 (a proof key without unlinked crates) is withdrawn: no proof is asked
  before a merge, so nothing lapses. Its nine commits stay on `build/baseline-guard`, unmerged, and its design
  is this folder's `DESIGN.md` at `4101a7cf`. The new M2 is the reference guard, the removal of the proof file
  and gate, and bounded scratch for the run on main. Items M2.1 to M2.6 are all new.

- 2026-11-15 Planner's decisions, each an assumption in the plan: the check may start every 30 minutes, not
  hourly, so a claim does not end pending; it holds `chrome` as well as `gpu`; `simulator` is exclusive. In
  other modules only `lock` fields changed: 114 verifier items lost theirs, 72 Chrome items moved to `chrome`
  and 15 simulator items to `simulator`. `GAME-SPLIT` M4.7's check gained `--full`, since it needs a real run.

- 2026-11-15 Planner's mistake, recorded: running `verify` in the planner's checkout started the main-branch
  check's runner on main's tip, which does not yet know `--full`. It was stopped after three minutes while
  still waiting for the `gpu` lock, and no run was recorded.

- 2026-11-16 [Design review](../../../.orchestrate/evidence/BASELINE-GUARD/design-review.md) of the reference guard at
  `f819b733`: changes requested, two Must Fix. MF4: removing a registry entry, or editing its fields, changes no
  pin, so a comparison could be dropped unseen. MF5: the native byte equality is in
  `scripts/compare_v1_pixels.py`, which no pin covered.
- 2026-11-16 Design revised for both. MF4: the pin now covers the entry's definition, and the guard reads the
  registry at the merge base: a baseline registered there must still be registered or be in the new retired
  list with a refresh, and a new baseline needs a refresh. New item M2.7 is its acceptance. MF5: each entry
  lists its comparison files, written out in the design: the verifier for all five, and for the native baseline
  the wrapper, `apple_tile_m0.py`, `m1`, `m2` and `compare_v1_pixels.py`. M2.1 and M2.2 are written again for
  it. [The model](../../../.orchestrate/evidence/BASELINE-GUARD/M2-replan/guard-scope.json) shows each of the review's two
  edits, a redefinition and a verifier edit now move a pin or fail the set check.
- 2026-11-16 Planner's decision, an assumption in the plan: `read_png` in `scripts/run_v1_showcase.py`, which
  decodes the ten 8-bit native references, stays outside the pins. `SHOWCASE`'s file changed in 19 commits since
  1 November, against 10 for all the listed comparison files together. Moving the decoder is later work.
- 2026-11-16 S6 taken: the design no longer says the bisect names no commit for a cause outside the repository.
  It names the first commit after the last recorded pass, so the name is a lead and the lead reads the log. Two
  assumptions in the plan are reworded to match. M2.5 gained an `ask`, from the review's note that a green run
  alone does not show M2 moved no retained image. M2.6 now covers adding, redefining and retiring a baseline.
- 2026-11-16 M2's checks tried by running each command by hand: `guard`, `GuardTests`, `RegisteredSetTests`,
  `ScratchTests` and the `proof_path` search each fail before the work, and none errors. M2.5's `verify` was
  not run: its command is unchanged, and it would wait on or start the shared run on main. `./dev check-docs`
  passes.

- 2026-11-16 M2 builder implemented committed-tree reference/definition pins, refresh-anchor validation,
  merge-base registration/retirement checks and CPU guard integration; local proof publication is removed.
  Retention keeps 48 completed records and the newest failed capture set, estimated 1.69 GiB from measured
  runs; incomplete runs stay for diagnosis. Focused tests and retained-reference identity checks pass.
  [M2 evidence](../../../.orchestrate/evidence/BASELINE-GUARD/M2.md) separates those checks from the shared exact-image run
  that the lead performs on main after the merge. No retained reference or comparison behavior changed.

- 2026-11-16 Ruling with `T2-CAPTURE`: its M3 builder changes `scripts/exact_baselines.py` and
  `scripts/exact_baselines_native.py` so a run whose Mac capture ended incomplete exits 75, "could not judge",
  and is recorded as that. No item of this module changes.
  [Ruling](../ARCHITECTURE.md#ruling-on-a-capture-that-drops-frames).
- 2026-11-18 The timing rule of the 15 November regime changed, on the lead's task for `SCENE-VIEWS` M3: no
  last milestone pauses the other builders for a timing, and quiet figures wait for one batched window if
  they are still wanted ([settings](../../../ORCHESTRATE.md#targets)). The assumption in this plan was
  written again to say so. No item of this module changed.
- 2026-11-20 [The module list](../ARCHITECTURE.md#ruling-on-relocated-references) gained a ruling on where
  retained references are kept, after `SHOWCASE` M9 moved them to test fixtures on the lead's ruling. In the
  registry a relocation changes `reference`, the matching `covered_paths` and, through the paths, each `pin`
  and `refresh`. The guard and the verifier are as they were. No item of this module changed.

- 2026-11-25 Three reviews of `GAME-SPLIT` WP3 found, outside its items, that a passing verifier record names a
  Wasm file no capture ran. Since `c33723d47` a capture given no web root builds and serves the proving bundle,
  and the verifier still builds and hashes the normal one.
  [The third review](../../../.orchestrate/evidence/GAME-SPLIT/WP3-review-3.md) measured `5f62545d…` against
  `710c305c…` instantiated. No image moved and no reference is stale.
- 2026-11-25 Planned as WP3 on the lead's task, with
  [a ruling](../ARCHITECTURE.md#ruling-on-the-wasm-a-chrome-capture-runs) in the module list. The verifier
  builds the proving bundle once, hands it to each Chrome capture, and fails a run whose capture did not
  instantiate the Wasm file it recorded. No reference or definition changes.
  [Planning notes](../../../.orchestrate/evidence/BASELINE-GUARD/WP3-plan/README.md)
- 2026-11-25 Planner's decisions: the real run is the builder's, once, before the merge, and WP3.2 checks its
  kept record; the capture script and `scripts/wasm_receipt.mjs` stay outside the pins; two assumptions were
  added to the plan; no owner's ruling was needed. The planning notes give the reasons.
- 2026-11-25 WP2's section in the plan shrank to its heading and review link, as a Done work package's does.
