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
rules; this module builds the parts of them that do not exist. M1 built the verifier and the gate, and is Done.

M2 narrows the gate's key. The lead's first task for it, of 15 November 2026, asked for a key that "covers
exactly the inputs that can change a registered exact image or the verifier's verdict ... and nothing else".
Three design reviews of that key failed on one finding, MF3. The lead's task of 15 November 2026 that replaces
it, word for word:

> Re-plan BASELINE-GUARD M2, whose design review has failed three times on MF3: a fully derived key can miss
> semantic data inputs in locations it excludes, such as data under docs/.
>
> Take the smaller step. Keep every path the exact-image key in scripts/exact_baselines.py covers today, and
> subtract only workspace crates and examples that are outside the build closure of what the verifier builds
> (v1-game and thrum-cook). Work out that closure from `cargo metadata` when the check runs; don't keep a
> hand-written list. That way a crate re-enters the key the moment the verified game depends on it, and a crate
> the closure can't classify stays in the key, so the check fails closed. Non-crate paths (scripts/, tools/
> except unlinked crates, .cargo/, Cargo.toml, Cargo.lock, the toolchain, the registry, the baselines and their
> evidence) stay covered exactly as now.
>
> Done when: tests show the key changes for an edit to a linked crate, a shader or asset it loads, scripts/, the
> lockfile and a baseline; and stays the same for an edit to an unlinked crate such as thrum-sim2d or
> thrum-network. The fuller derived key is out of scope here; record it as possible later work if re-proving
> remains a bottleneck.

This design answers that task. "Exactly the inputs ... and nothing else" is no longer M2's outcome: the key is
M1's, less one kind of file. [Later work](#later-work) records the fuller key.

## What went wrong, and why a rule alone would not fix it

Read: [APPLE-TILE M2's review](../../evidence/APPLE-TILE/M2-review.md),
[APPLE-PRESENTATION M1's first review](../../evidence/APPLE-PRESENTATION/M1-review-1.md),
this design's [first review](../../evidence/BASELINE-GUARD/design-review-1.md), `land` in the build skill's
`agent_loop.py`, and `scripts/apple_tile_m0.py`, `apple_tile_m1.py`, `apple_tile_m2.py`,
`apple_tile_m1_compare_browser.py` and `capture_v1_browser.mjs`.

- `APPLE-TILE`'s exact comparisons need a release build, a cooked store and, for Chrome, a browser. Nothing ran
  them automatically.
- A builder merges with `land`. Under one lock it fetches, rebases, runs `./dev check` and pushes, and it does not
  hand control back between the rebase and the push. `./dev check` is the only gate before main, and it did not
  hold the comparison.
- So `APPLE-PRESENTATION` M1's jitter change merged with 316,181 of 800,000 pixels different at one checkpoint.
- `APPLE-TILE` M2 was then claimed on a binary built before its rebase, so its own evidence missed the break too.

A written rule tells a builder what not to do. Only a failing gate stops the push. M1 built that gate: a proof
from a full verifier pass, keyed to the content being pushed.

**What M1's key costs.** The key covers paths listed by hand: all of `crates/`, `scripts/` and `tools/`, the
game, the build configuration and the references. The verifier builds two packages, and Cargo links nine of the
workspace's 23 into them. So a change to a crate the game never links lapses every open proof, and each builder
runs the verifier again after its rebase. `SIM-2D`'s log records one such run forced by a change to
`thrum-cloud-model`.

## Purpose and scope

**Commissioned:**

1. **The verifier.** One command that, from a clean checkout, draws fresh output on the Mac and in Chrome at the
   current commit and compares it with every registered exact baseline. On a full pass it writes a proof.
2. **The gate.** `./dev check` refuses a commit that changes the covered paths unless a proof exists for exactly
   that content. `land` runs `./dev check` on the rebased commit, so nothing unproven is pushed.
3. **The narrower key (M2).** The covered paths are M1's listed paths, less the files of workspace packages
   that Cargo links into nothing the verifier builds. Cargo works out which those are each time the key is
   computed.

**Not commissioned:** changing any baseline, comparison or tolerance; toleranced baselines such as the G7 route's;
visual judgement of any kind; [the fuller derived key](#later-work), which would also leave out scripts the
verifier never runs, test sources and other files no verified build reads.

**Owns:** the registry file, the verifier, its proof records and the gate. The retained references, and the right
to refresh them, stay with each baseline's owner module.

## Research

- **Test the commit that will be pushed.** Graydon Hoare's rule for Rust's bors, to keep a repository that always
  passes its tests by testing each change as integrated before the branch advances, and the merge queues built on
  it ([Jane Street's account](https://blog.janestreet.com/making-never-break-the-build-scale/)). `land` already has
  this shape, holding a lock from rebase to push.
- **Reuse a result only for identical inputs.** Build systems such as Bazel key a cached result by a hash of the
  action's inputs ([remote caching](https://bazel.build/remote/caching)). The proof is keyed the same way, so a
  rebase that changes an input invalidates it and one that does not keeps it.
- **Take the affected packages from the package graph, and treat what the graph cannot place as affecting
  everything.** Tools that decide which packages a change touches read the dependency graph, not a list.
  [guppy's determinator](https://docs.rs/determinator) does it for Cargo workspaces, and its rule for a changed
  file outside every package is to assume that everything needs to be rebuilt. Nx and Turborepo do the same for
  JavaScript. `cargo metadata` gives Cargo's own graph without building.
- **Check the graph against what the compiler read.** Make and Ninja learn a target's inputs from the dependency
  file the compiler writes ([Ninja's depfiles](https://ninja-build.org/manual.html#ref_headers)). Cargo writes
  one beside each artefact: the sources rustc read, the files `include_str!` and `#[path]` pulled in, the
  sources of path dependencies and the files a build script declared
  ([`output_depinfo`](https://doc.rust-lang.org/nightly/nightly-rustc/cargo/compiler/output_depinfo/fn.output_depinfo.html)).
  A package graph says what Cargo links, and the record says what was read; a file can be read from a folder
  whose package is not linked.
- **Leaving out files by what a script reads was tried and withdrawn.** Three revisions of this design left out
  documents, scripts nothing reaches and unread sources, and each review found a script input the rule
  missed. [Later work](#later-work) has the record.

## Mechanism

### The registry

One file, `.config/exact-baselines.json`, lists each registered baseline: its identity, its owner, its retained
reference files, and how a fresh candidate is produced and compared. The table in
[the module list](../ARCHITECTURE.md#exact-baselines) is the same registry for people; the two must agree.

It holds two things the key is made from:

- **The listed paths,** as M1 wrote them: `crates/`, `examples/v1-game/`, `scripts/`, `tools/`, `.cargo/`, the
  workspace `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, the registry itself and the retained references.
  M2 removes none.
- **The built packages,** new in M2: the names of the packages the verifier builds, `v1-game` and `thrum-cook`
  today. They are the only package names the guard holds.

### The linked packages

The **linked packages** of a commit are the built packages and every package reachable from them over normal
and build dependencies whose source is a path. Cargo works them out, through `cargo metadata`, each time the
gate or the verifier computes the key. Nothing stores the result, and no file lists crates.

- Every declared dependency is followed: optional ones, and ones for any target platform. So no feature or
  target a build selects can link a package the derivation left out. The native game is built with extra
  features and the page for `wasm32-unknown-unknown`; both graphs are inside this one.
- Dev-dependencies are not followed, because `cargo build` does not compile them. `v1-game` has one today, on
  `asset-loop`.

An **unlinked package** is a workspace member, as Cargo reports the members, that is not linked. Today nine of
the 23 members are linked. Of the 14 unlinked, six lie under the listed paths (`thrum-cloud-model`,
`thrum-network`, `thrum-sea`, `thrum-sea-model`, `thrum-sim2d` and `thrum-water-model`) and so does `xtask`
under `tools/`; the other seven are examples the listed paths never took in.

### The covered paths

The **covered paths** of a commit are its tracked files under a listed path or in a linked package's folder,
less the files in the folder of an unlinked package. Three kinds of file in an unlinked package's folder stay
covered:

- the package's own `Cargo.toml`. Cargo reads every member's manifest to load the workspace, and keeping them
  costs nothing measured: no commit since 11 November changed one and no other covered path;
- a file that is also inside a linked package's folder, when packages nest;
- a file at or under a listed path that itself lies inside that package's folder. This is how the registry
  brings back a file a build or a comparison reads from an unlinked package.

That is the only rule that removes a file. Everything else under the listed paths stays covered exactly as in
M1, read or not: every file under `scripts/`, the files under `tools/` outside a package, `.cargo/`, the
workspace manifest, the lockfile, the toolchain file, the registry and the references.

**A file Cargo cannot place stays covered.** A file is removed only when Cargo reports its folder as that of a
workspace member and the member is not linked. A folder under `crates/` with no package, a file beside the
packages, and a package Cargo does not list as a member all stay in the key.

**A linked package is covered wherever it is.** If a built package comes to depend on a package outside the
listed paths, that package's tracked files join the key in the commit that adds the dependency.

### The content key and the proof

The **content key** of a commit is a digest of the path and Git object ID of every covered path at that commit.

A **proof** is the record the verifier writes when every registered comparison passed on a clean tree. It names
the content key of its commit. It is kept untracked in the worktree, and it is the only thing the gate accepts.

### The verifier

One command, from a clean checkout:

1. refuses a tree with uncommitted changes, and reports low disk space as its own named failure;
2. builds the release game with the features the owners' scripts need, builds the cooker, and builds the Wasm
   page with `examples/v1-game/web/build.sh`;
3. makes [the build-read check](#the-build-read-check) on the records of those three builds;
4. cooks the showcase valley fresh and runs `APPLE-TILE`'s native matrices (`apple_tile_m1.py`,
   `apple_tile_m2.py`) unchanged;
5. captures the four Chrome `fixed` runs with `capture_v1_browser.mjs` and compares each with its retained set;
6. exits non-zero on any difference, naming the baseline, its owner and the comparison;
7. on a full pass checks that the tree is still clean and the content key unchanged, and writes the proof: the
   revision, the content key, the linked packages, the native binary's hash, the Wasm artefact hashes and the
   cooked root.

It takes every input from the checkout and its own fresh outputs. It never reads another task's scratch. Every
run's record names the content key, pass or fail.

### The build-read check

The package graph says which packages Cargo links. It does not say which files a compiler opens: a linked
package can pull in a file from another package's folder with `include_str!`, `include_bytes!` or `#[path]`,
with no dependency between them. A test in `thrum-render` already includes sources of the game that way
(`tests/visibility_buckets.rs`). Such a file in an unlinked package's folder would change the image and not the key.

So after its builds the verifier reads the dependency record Cargo wrote beside each artefact it built: the
native game, the cooker and the Wasm module. Every file a record lists that lies inside the repository and
outside the build folder must be a covered path at the commit. If one is not, the verifier names the file and
the record and writes no proof. It does the same when a record is missing or cannot be read.

The cure is the registry: the file is listed, which makes it a covered path. Cargo writes the record again on
every build, one that compiles nothing included, so the records are those of this run's builds.

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

A step of `./dev check`, with no GPU, no browser and no build. It asks Cargo for the worktree's package graph
and works out the covered paths. Then:

1. If a covered path has an uncommitted change, an untracked file included, it fails. Past this step the
   worktree's manifests are the commit's, so the graph is the commit's.
2. If no covered path differs between the commit and its merge base with the remote main branch, it passes.
   Main's own covered content was proved when it landed.
3. Otherwise it passes only if the worktree holds a proof that names the commit's content key. If none does, it
   fails and names the verifier command.

It fails, naming the cause, if the base cannot be resolved, if the registry cannot be read, if Cargo cannot load
the workspace, if the registry names a built package the workspace does not hold, or if a linked package's
folder lies outside the repository. There is no flag or environment variable that skips it or narrows the
covered paths.

### Check that it can work

- **The sequence the first review gave.** A builder proves its branch. Another builder lands a change to a
  covered file. `land` rebases the first branch, so its content key changes under every proof it holds.
  `./dev check` finds no proof and fails, and `land` does not push. The builder runs the verifier on the rebased
  commit, which now draws the combined result, and lands again.
- **A rebase that brings in only an unlinked package's files keeps the proof.** Those files are in the key at
  neither commit, so the key is equal and the proof stands. That is all M2 adds.
- **A package re-enters the key the moment the game depends on it.** The dependency is added in a linked
  package's manifest, which is a covered path, so that commit's key differs for the manifest alone. From that
  commit Cargo reports the package as linked, and every file in it is covered.
- **Why the commit's own graph is enough.** The gate never needs the graph of the merge base. The linked
  packages follow from the registry, the workspace manifest and the linked packages' manifests, and all of
  those are covered at the commit. If none differs from the base, the base has the same linked packages. A
  package cannot leave or join the graph without one of them changing, and that change is a covered
  difference, which sends the gate to step 3.
- **Why an equal key means equal builds.** Equal keys mean the same build configuration, the same manifests and
  so the same linked packages, and every file in those packages identical, with none added or removed. A
  compiler reads a file only as a target's root or because a file it already read names it. Roots come from
  the manifests and from files Cargo finds by name inside a linked package; a new one changes the key. Every
  file read at the proof was a covered path, by the build-read check, and is identical, so each names the same
  files as before.
- **What M2 can newly miss is one thing:** a file in an unlinked package's folder that something other than a
  compiler reads. [What it does not stop](#what-it-does-not-stop) has it, with what reads there today.
- **Tried on a workspace made for it** ([trial](../../evidence/BASELINE-GUARD/M2-plan/README.md#the-smaller-step)):
  `cargo metadata --no-deps` gives each member's path dependencies with their kinds in 0.02 to 0.08 s. A linked
  package that includes a file from an unlinked package's folder has that file in its artefact's record, as
  `src/../../other/data.txt`, so a record's paths are normalised before they are compared. Cargo wrote the
  record again on a build that compiled nothing.
- **Tried on real builds.** The three records of a verifier build, in each of the loop's `CLOUDS` and `SIM-2D`
  checkouts, list 25 to 227 files. Every one is tracked and inside one of the nine linked packages, so the
  check passes on today's tree.
- **The numbers** ([measurements](../../evidence/BASELINE-GUARD/M2-plan/README.md#the-smaller-step)). The
  graph from `cargo metadata`, with and without every feature on, and the graph from the manifests' declared
  dependencies give the same nine packages. 73 of the 738 files under M1's listed paths leave the key: 67 Rust
  sources, two shaders of `thrum-sea` and four other files. Replayed over main since 11 November, each commit
  judged by its own manifests, M1's key changed at 204 commits and this one changes at 186. The 18 it spares
  changed only `thrum-sea` (9), `thrum-cloud-model` (6), `thrum-water-model` (2) or `thrum-sea-model` (1). Of
  the 186, 46 changed nothing covered but files under `scripts/`, and 97 nothing but linked packages.
- **A fix with no plan** goes through the same `land` and the same gate.
- **Chrome and night are inside it**, because the proof exists only when every registered comparison passed.
- **Would it have caught MF1?** The jitter change edited sources of the game and the renderer, which are linked
  packages. M1 showed the refusal with that mutant, a night-only mutant and a Chrome-only mutant.
- **Are the bytes reproducible?** The native matrix matched on two separately rebuilt binaries on 11 November
  ([the MF1 recheck](../../evidence/render/apple-tile-m2/MF1-recheck.md): 38 of 38 and 4 of 4, then all 42 again
  after another module integrated). Chrome's four sets each matched all 481 frames against a fresh build at
  `APPLE-TILE` M1 and again on untouched main in this module's M1.
- **Can the builder, the reviewer and the loop reach it?** A Codex agent asks to launch Chrome outside its
  sandbox, and the loop reruns a milestone's checks itself, both as the existing browser checks already do. The
  gate needs Git and Cargo and neither a GPU nor a browser, so `land` can always run it.

## What other modules use

Every module's builder runs these, and some call them from their own scripts
(`docs/evidence/WATERS/finish_m1.py`, `docs/evidence/SEA/qualify_m2_baselines.py`). They keep these meanings:

- `python3 scripts/exact_baselines.py verify`, under the `gpu` lock: the verifier. It is the `check` of an item
  in most modules' done-when files, and the command does not change.
- `python3 scripts/exact_baselines.py gate`: the gate, as `./dev check` runs it. In Python, `gate(root)` raises
  `ValueError` when it refuses.
- `python3 scripts/exact_baselines.py covered`, new in M2: prints the linked packages, the unlinked packages
  whose files are left out of the key, and the number of covered paths at the current commit.
- `content_key(root)`: the commit's content key. `proof_path(root)`: the file a proof for that key is kept in.
- A proof's fields `passed`, `content_key`, `baselines`, `record` and `revision`, and the same `content_key` in
  the record it points to. After a rebase the proof still covers, `revision` is the commit the verifier ran on,
  not the current one.
- The registry's listed paths. A module may add a path there for a file inside an unlinked package's folder
  that one of its scripts or builds reads, and says so in its log. Nobody removes a listed path without a
  ruling in the module list.
- Where a comparison's data may live. A tracked file under the listed paths is covered without being declared,
  unless it is in an unlinked package's folder. Data a comparison, the cooker or the game reads does not live in
  an unlinked package's folder unless the registry lists it.

The ruling on [a frame that varies under load](../ARCHITECTURE.md#a-frame-that-varies-under-load) compares the
content keys of a failed run and a passing one; every run's record holds it. `GAME-SPLIT` M4 has the verifier
hand `THRUM_GAMES` to the commands it starts, and M5 renames the game's package; M5's change to the registry is
the package's name among the built packages and its folder among the listed paths.

## What it does not stop

- A push of a commit that `./dev check` did not pass: `land --no-check`, or a plain `git push` after a rebase
  with no check. Rule 1 of the module list forbids both for a covered-path change.
- A forged proof. The proof is a local file, as `./dev check` is local code.
- **A script, the cooker or the game at run time that reads, as data, a file in an unlinked package's folder
  the registry does not list.** A change to that file keeps the proof. Nothing an unprivileged program can do
  on macOS shows the verifier that a step looked there, so this is closed only by listing the file. The task
  accepts it by removing those packages from the key. Today four of the scripts the verifier runs
  (`apple_tile_m0.py`, `apple_tile_m1.py`, `capture_v1_runtime.py` and `capture_v1_browser.mjs`) hash every
  file under `crates/` into the records they write, `thrum-sea`'s two shaders among them. No script compares
  those hashes with anything, and none of the verifier's scripts names an unlinked package. M1's key has the same limit
  for every path outside its list, such as data under `docs/` that is not a registered reference.
- A Cargo build script that reads, without declaring it to Cargo, a file in an unlinked package's folder. One
  linked package has a build script today, `thrum-app`, and it reads one C file of its own package.
- A change outside the repository: macOS, the GPU driver, Chrome, Node, `wasm-bindgen`, or the user's Cargo
  configuration and registry cache. See risk 1.

The first two are caught after the fact by the claim-time item of rule 3, whose command a builder cannot
change, and by the next builder's verifier run, which fails on main. The next two would change an image only
through code a reviewer of that change reads; nothing catches them automatically.

## Cost

A landing that changes a covered path pays one verifier run on its final content: two builds, a cook, the
native matrices and four Chrome captures of 481 frames, 1,022 seconds when M1 measured it. A rebase that brings
in another covered change pays it again. A landing or a rebase that touches only other files pays the gate,
which adds one `cargo metadata` call, 0.02 to 0.2 s here, to its Git queries. The build-read check adds the
reading of three small files to a verifier run. The times are recorded, not budgeted. The scripts need 16 GiB
free.

By the replay above, M2 removes about one lapse in eleven. A change to any file under `scripts/`, to a test in
a linked package, or to any workspace package's manifest still lapses every open proof.

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
6. **The landing that changes the key lapses every open proof once**, because the verifier itself is a covered
   path. Each builder with covered changes runs the verifier once more.
7. **Cargo cannot load the workspace where the gate runs**, so `./dev check` fails for a cause that is not a
   baseline. The gate names Cargo's error. `./dev check` already needs Cargo to load the same workspace.
8. **A record lists a file the check cannot place**, such as a generated source outside the build folder. The
   verifier writes no proof and names it; the builder ends `BLOCKED` with the record, and no file is waved
   through.

## Later work

**The fuller derived key** is not planned. It would also leave out the scripts the verifier never runs, the
Rust sources no verified build reads and other files nothing reads. By the replay, 46 of the 186 commits that
still lapse proofs touched nothing covered but `scripts/`, and by the earlier measurements over half of the
commits that changed linked packages on the night of 14 to 15 November changed only sources no verified build
reads. Take it up if re-proving remains a bottleneck after M2.

Three revisions of this design tried it on 15 November, and each design review returned MF3: once a key leaves
out data or scripts by where they are or by what refers to them, a script that reads a file only if it is
there, or finds it by listing a folder, can take input the key does not hold. The revisions reviewed are at
commits `df6b07f5`, `9b44ba81` and `76daae92`, their reviews are in
[the evidence folder](../../evidence/BASELINE-GUARD/), and
[the planner's measurements](../../evidence/BASELINE-GUARD/M2-plan/README.md) hold what was tried: a scratch
tree holding only the covered paths, Cargo's records of unread sources, and a tripwire for scripts left out. A
new attempt starts from that finding. It needs either a way to see what a step reads, or the owner's decision
that a named kind of file may be left out unseen.

## Limits on builders

- Never refresh, regenerate or edit a file under a registered baseline. If a baseline does not match on untouched
  main, end `BLOCKED` with the comparison; the refresh is its owner's.
- No tolerance, nearest-frame search, retry inside a run or skipped record, for any reason.
- Do not change `scripts/apple_tile_m0.py`, `apple_tile_m1.py` or `apple_tile_m2.py`, the byte mode of the Chrome
  comparator, `scripts/capture_v1_browser.mjs` or `examples/v1-game/web/build.sh`. They belong to other
  modules.
- Remove a file from the key only by the one rule: Cargo reports its folder as an unlinked workspace member's.
  A second rule, for documents, scripts, tests or anything else, is the later work and a change to this design.
- Keep no list of crates, linked or unlinked, in the script, the registry or a test of this repository's tree.
  The registry names the built packages and nothing more.
- When Cargo's answer about a file or a package is missing or unclear, cover the file or refuse. Never fall
  back to M1's list silently, and never to a narrower one.
- Mutants, scratch commits and any demonstration of `land` never push to the real remote.
- Chrome needs a browser launch outside the Codex sandbox. If a launch is refused, leave the item open and say
  so; do not substitute a Mac result.

## Proof and builder handoff

`mac-metal` and `chrome-webgpu`. GPU runs hold the `gpu` lock. Scratch goes in `tmp/baseline-guard/`; selected
evidence in `docs/evidence/BASELINE-GUARD/`. Mutants run in scratch and are never merged. A demonstration of
`land` or of the gate across a rebase runs in a scratch clone whose remote is a local bare repository, never
against the real remote. The gate's tests build small Cargo workspaces of their own, with no dependency from
crates.io, so they run offline and need no GPU.
