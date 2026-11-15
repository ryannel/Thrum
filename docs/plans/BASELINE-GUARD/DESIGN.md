# Baseline guard: a change that alters a registered exact image is refused before it reaches main

11 November 2026, revised 15 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#exact-baselines) · [the brief](../BRIEF.md). C12 in
[CONTRACTS](../../design/CONTRACTS.md) governs evidence.

## The requests

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

The lead's task of 15 November 2026, which the owner approved that day:

> Outcome: the proof key covers exactly the inputs that can change a registered exact image or the verifier's
> verdict (the build closures of what the verifier builds, the scripts and tools it runs, toolchain and lockfile,
> the cooked showcase assets, the registry and the baselines) and nothing else. Derive the set rather than
> hand-list crates, so a new dependency of v1-game is covered automatically. The gate must still fail closed:
> anything that can change a verified image forces a re-proof.

[The covered paths](#the-covered-paths), [the unread sources](#the-unread-sources) and
[the gate](#the-gate) answer it.

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

**What the first key cost.** The gate was first built with a key over paths listed by hand: all of `crates/`,
`scripts/` and `tools/`, the game, the build configuration and the references. The verifier builds two packages,
and Cargo links nine of the workspace's 23 into them. So a change to a crate the game never links, or to another
module's check script, lapsed every open proof, and each builder ran the verifier again after its rebase. On the
night of 14 to 15 November, 33 of main's commits changed that key. Ten of them changed a file a verified build or
comparison reads ([the planner's measurements](../../evidence/BASELINE-GUARD/M2-plan/README.md)).

## Purpose and scope

**Commissioned:**

1. **The verifier.** One command that, from a clean checkout, draws fresh output on the Mac and in Chrome at the
   current commit and compares it with every registered exact baseline. On a full pass it writes a proof.
2. **The gate.** `./dev check` refuses a commit that changes the covered paths unless a proof exists for exactly
   that content. `land` runs `./dev check` on the rebased commit, so nothing unproven is pushed.
3. **The derived key.** The covered paths are worked out from the commit, and a proof stays good for a later
   commit exactly when nothing a verified build or comparison reads has changed.

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
  action's inputs ([remote caching](https://bazel.build/remote/caching)). The proof is keyed the same way, so a
  rebase that changes an input invalidates it and one that does not keeps it.
- **Take the inputs from the package graph.** Tools that decide which tests a change needs read the dependency
  graph, not a list: [guppy's determinator](https://docs.rs/determinator) does it for Cargo workspaces, with rules
  for files outside any package, and Nx and Turborepo do it for JavaScript. `cargo metadata` gives the resolved
  graph without building.
- **Take the files from the compiler.** Make and Ninja learn a target's headers from the dependency file the
  compiler writes ([Ninja's depfiles](https://ninja-build.org/manual.html#ref_headers)), because no list kept by
  hand stays right. Cargo writes the same record for each artefact, and its own rebuild check trusts it.
- **Bazel enforces declared inputs with a sandbox** that hides every other file. That is ruled out here:
  `APPLE-TILE`'s and `T2-CAPTURE`'s scripts hash the whole source tree into their records, so a run that could
  not read an uncovered file would fail, and a trace of what a run opens would list everything.

## Mechanism

### The registry

One file, `.config/exact-baselines.json`, lists each registered baseline: its identity, its owner, its retained
reference files, and how a fresh candidate is produced and compared. The table in
[the module list](../ARCHITECTURE.md#exact-baselines) is the same registry for people; the two must agree.

It also names the roots the covered paths are derived from: the packages the verifier builds, and the scripts it
starts. The verifier takes its build commands and its comparison scripts from the same entries, so what is built
and what is keyed cannot drift apart. The registry may list a further path outright, for an input no rule below
reaches. It has no entry that removes a path.

### The covered paths

The **covered paths** of a commit are the tracked files a verified build or comparison can read. They are
derived from the commit with Git and Cargo's resolver, with no build, in four parts:

1. **The build configuration:** the workspace `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` and `.cargo/`,
   whole.
2. **The linked packages.** Cargo resolves the workspace at that commit with every feature on and for every
   target platform, so no feature or target the verifier picks can link a package the derivation missed. A
   **linked package** is a package the registry says the verifier builds, or one reachable from it over normal
   and build dependencies, whose source is a path. Dev-dependencies are not followed, because `cargo build` does
   not compile them. Every tracked file in a linked package's folder is covered: Rust sources, shaders, the web
   page, routes, presets and the showcase assets the cooker reads. Packages from the registry are pinned by the
   lockfile.
3. **The verifier's scripts:** the verifier's own files, each script the registry names, and every tracked file
   one of those reaches. A script reaches a file by importing it, in Python or JavaScript, or by naming the
   file's path in a quoted string. The one script name resolved at run time, the native matrix, comes from the
   registry. A quoted folder reaches nothing: the owners' scripts name `crates`, `examples` and `scripts` only to
   hash them into their records, and the folders they read from are linked packages or references.
4. **The registry and every registered reference file.**

A package Cargo does not link, a script nothing reaches, `tools/`, documents, plans and evidence other than the
references are not covered.

### The unread sources

Inside a linked package, a Rust file may be compiled only for tests, benches, examples or another configuration.
Cargo records which files the compilers read: beside each artefact it writes a dependency record that lists
every source file and every file an `include_str!` or `include_bytes!` pulled in.

After its builds, the verifier reads the records of the artefacts it built: the native game, the cooker and the
Wasm module. An **unread source** is a covered `.rs` file, in a linked package that has no build script, that
none of those records lists. Nothing else can be unread:

- A file that is not Rust stays covered, read or not. The cooker, the page build, the scripts and the game at
  run time read such files and leave no record.
- A package with a build script stays covered whole. The script can read any file in its package without
  declaring it, and Cargo itself treats the whole package as the script's input in that case.

The records also check the derivation. If one lists a tracked file that is not a covered path, a linked package
reads outside the linked packages: the verifier names the file and writes no proof. It does the same when a
record is missing, or when a linked package's source is a path outside the repository.

### The content key and the proof

The **content key** of a commit, under a list of unread sources, is a digest of the path and Git object ID of
every covered path at that commit that is not on the list. With an empty list it covers every covered path.

A **proof** is the record the verifier writes when every registered comparison passed on a clean tree. It holds
its unread sources and names the content key of its commit under them. It is kept untracked in the worktree, and
it is the only thing the gate accepts.

### The verifier

One command, from a clean checkout:

1. refuses a tree with uncommitted changes, and reports low disk space as its own named failure;
2. builds the release game with the features the owners' scripts need, builds the Wasm page with
   `examples/v1-game/web/build.sh`, and cooks the showcase valley fresh;
3. reads the dependency records of those builds, checks them against the covered paths and works out the unread
   sources;
4. runs `APPLE-TILE`'s native matrices (`apple_tile_m1.py`, `apple_tile_m2.py`) unchanged;
5. captures the four Chrome `fixed` runs with `capture_v1_browser.mjs` and compares each with its retained set;
6. exits non-zero on any difference, naming the baseline, its owner and the comparison;
7. on a full pass writes the proof, with the revision, the content key, the unread sources, the native binary's
   hash, the Wasm artefact hashes and the cooked root.

It takes every input from the checkout and its own fresh outputs. It never reads another task's scratch. The
record of every run that finished its builds names the content key, pass or fail, so two runs on the same content
can be told to be so.

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

A step of `./dev check`, with no GPU, no browser and no build:

1. If a covered path has an uncommitted change, it fails.
2. If no covered path differs between the commit and its merge base with the remote main branch, it passes.
   A path counts if it is covered at either commit. Main's own covered content was proved when it landed.
3. Otherwise it passes only if the worktree holds a proof whose content key equals the commit's content key
   under that proof's own unread sources. If none does, it fails and names the verifier command.

It fails if the base cannot be resolved, if Cargo cannot resolve the workspace, or if the registry cannot be
read. There is no flag or environment variable that skips it, widens a proof's unread sources or narrows the
covered paths.

### Check that it can work

- **The sequence the review gave.** A builder proves its branch. Another builder lands a change to a covered file
  a build reads. `land` rebases the first branch, so its content key changes under every proof it holds.
  `./dev check` finds no proof and fails, and `land` does not push. The builder runs the verifier on the rebased
  commit, which now draws the combined result, and lands again.
- **A rebase that brings in nothing a build reads keeps the proof.** A change to a package Cargo does not link,
  to a script nothing reaches or to a document is outside the covered paths. A change to an unread source is
  left out of the key on both sides. The key is equal and the proof stands.
- **Why an equal key means equal builds.** Equal keys under one unread list mean the same build configuration,
  the same manifests and so the same linked packages, and every covered file identical except files on the list.
  No covered file was added, because a new file is on no list, and none off the list was removed. A compiler reads a file only as a target's root or
  because a file it already read names it. Roots come from the manifests, which are equal, and from files Cargo
  finds by name; a newly found file is a new file, and changes the key. Every file read before is identical, so
  each names the same files as before. The builds therefore read the same bytes with the same configuration, and
  everything else the verifier reads is covered and identical.
- **The numbers.** Cargo links nine packages into the two the verifier builds, and resolves the graph in 0.12 s.
  The dependency records of one verifier build list 251 repository files, every one inside those nine packages.
  176 of their 375 Rust files are unread. Imports and quoted paths reach 23 of the 107 files under `scripts/`.
  Replayed over the night of 14 to 15 November, the key changes at 10 commits where the listed key changed at 33
  ([measurements](../../evidence/BASELINE-GUARD/M2-plan/README.md)).
- **A fix with no plan** goes through the same `land` and the same gate.
- **Chrome and night are inside it**, because the proof exists only when every registered comparison passed.
- **Would it have caught MF1?** The jitter change moved every rasterised surface, so the native matrix fails and no
  proof is written. M1 showed it with that mutant, a night-only mutant and a Chrome-only mutant. The jitter change
  edited compiled sources of the game and the renderer, which the derived key covers.
- **Are the bytes reproducible?** The native matrix matched on two separately rebuilt binaries on 11 November
  ([the MF1 recheck](../../evidence/render/apple-tile-m2/MF1-recheck.md): 38 of 38 and 4 of 4, then all 42 again
  after another module integrated). Chrome's four sets each matched all 481 frames against a fresh build at
  `APPLE-TILE` M1 and again on untouched main in this module's M1.
- **Can the builder, the reviewer and the loop reach it?** A Codex agent asks to launch Chrome outside its
  sandbox, and the loop reruns a milestone's checks itself, both as the existing browser checks already do. The
  gate needs neither a GPU nor a browser, so `land` can always run it.

## What other modules use

Every module's builder runs these, and some call them from their own scripts
(`docs/evidence/WATERS/finish_m1.py`, `docs/evidence/SEA/qualify_m2_baselines.py`). They keep these meanings:

- `python3 scripts/exact_baselines.py verify`, under the `gpu` lock: the verifier. It is the `check` of an item
  in most modules' done-when files, and the command does not change.
- `python3 scripts/exact_baselines.py gate`: the gate, as `./dev check` runs it. In Python, `gate(root)` raises
  `ValueError` when it refuses.
- `python3 scripts/exact_baselines.py covered`: prints the covered paths of the current commit with the reason
  each is covered and, when a proof covers the commit, the unread sources.
- `content_key(root)`: the commit's content key under the proof that covers it. With no such proof, the key
  with nothing left out.
- `proof_path(root)`: the file of the proof that covers the commit. With no such proof, a path that does not
  exist.
- A proof's fields `passed`, `content_key`, `baselines`, `record` and `revision`, and the same `content_key` in
  the record it points to. After a rebase the proof still covers, `revision` is the commit the verifier ran on,
  not the current one.

The ruling on [a frame that varies under load](../ARCHITECTURE.md#a-frame-that-varies-under-load) compares the
content keys of a failed run and a passing one. Both records hold it, since each run finished its builds.
`GAME-SPLIT` M4 has the verifier hand `THRUM_GAMES` to the commands it starts, and M5 renames the game's
package; M5's change to the registry is the package's name and path, and the covered paths follow from it.

## What it does not stop

- A push of a commit that `./dev check` did not pass: `land --no-check`, or a plain `git push` after a rebase
  with no check. Rule 1 of the module list forbids both for a covered-path change.
- A forged proof. The proof is a local file, as `./dev check` is local code.
- A covered script that reads an uncovered tracked file through a path it computes or a folder it lists, or
  reads an unread Rust source as data. Nothing records what a script reads, and the owners' scripts open every
  source file to hash it. The registry lists such a path outright when one is found.
- A change outside the repository: macOS, the GPU driver, Chrome, Node, `wasm-bindgen`, or the user's Cargo
  configuration and registry cache. See risk 1.

The first three are caught after the fact by the claim-time item of rule 3, whose command a builder cannot
change, and by the next builder's verifier run, which fails on main.

## Cost

A landing that changes a covered file a build or comparison reads pays one verifier run on its final content:
two builds, a cook, the native matrices and four Chrome captures of 481 frames, 1,022 seconds when M1 measured
it. A rebase that brings in another such change pays it again. A landing or a rebase that touches only other
files pays the gate, which adds one Cargo resolve of about a tenth of a second to its Git queries. A change to
the workspace manifest or the lockfile lapses every proof, a new workspace member included. The times are
recorded, not budgeted. The scripts need 16 GiB free.

## Risks

1. **A macOS, driver or Chrome update changes the bytes**, and no covered-path change can land. The sign is that
   untouched main fails the verifier. The baseline's owner refreshes it under rule 1, with the cause recorded. The
   gate is not disabled.
2. **Chrome's references no longer match main.** The verifier fails on untouched main. The builder ends `BLOCKED`
   with the comparison, and the refresh is `APPLE-TILE`'s.
3. **The gate is edited by the builder it stops.** Rule 1 forbids it and a reviewer checks it.
4. **A verifier run nears the loop's one-hour limit on a check.** M1 recorded 17 minutes. The fallback is a
   comparison-only entry to the owners' scripts, proposed to `APPLE-TILE`, never fewer comparisons.
5. **Two builders invalidate each other's proofs.** Each landing forces at most one more run on the other, and
   whichever lands first makes progress.
6. **A dependency record is stale or incomplete**, so a compiled file is taken as unread. Cargo keeps the records
   for its own rebuild decisions, and the verifier refuses a missing one. M2 has a reviewer sample the unread
   sources of a real proof against the code, and shows a compiled-source change refused on this repository.
7. **The landing that changes the key lapses every open proof once**, because the verifier itself is a covered
   path. Each builder with covered changes runs the verifier once more.

## Limits on builders

- Never refresh, regenerate or edit a file under a registered baseline. If a baseline does not match on untouched
  main, end `BLOCKED` with the comparison; the refresh is its owner's.
- No tolerance, nearest-frame search, retry inside a run or skipped record, for any reason.
- Do not change `scripts/apple_tile_m0.py`, `apple_tile_m1.py` or `apple_tile_m2.py`, the byte mode of the Chrome
  comparator, `scripts/capture_v1_browser.mjs` or `examples/v1-game/web/build.sh`. They belong to other
  modules.
- When unsure whether a file is an input, cover it. Leave a file out only on the evidence this design names.
- Mutants, scratch commits and any demonstration of `land` never push to the real remote.
- Chrome needs a browser launch outside the Codex sandbox. If a launch is refused, leave the item open and say
  so; do not substitute a Mac result.

## Proof and builder handoff

`mac-metal` and `chrome-webgpu`. GPU runs hold the `gpu` lock. Scratch goes in `tmp/baseline-guard/`; selected
evidence in `docs/evidence/BASELINE-GUARD/`. Mutants run in scratch and are never merged. A demonstration of
`land` or of the gate across a rebase runs in a scratch clone whose remote is a local bare repository, never
against the real remote. The gate's tests build small Cargo workspaces of their own, offline, and need no GPU.

## Open questions

| Question | Settled by |
|---|---|
| Does Cargo rewrite an artefact's dependency record on a build that compiles nothing? | M2's builder, before relying on the records' dates. The design needs only that the record is right for the sources built |
