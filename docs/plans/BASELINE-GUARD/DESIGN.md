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

The lead's task of 15 November 2026 for the one finding, MF3, of
[the design review](../../evidence/BASELINE-GUARD/design-review.md) that followed:

> MF3 has now been raised twice. Settle it this time: the reviewer accepts either covering optional and
> enumerated script inputs in the key, or making any input the key cannot resolve block the proof from being
> published (fail closed). Pick whichever is simpler to make provably complete, and keep excluding the unrelated
> crates and scripts.

[The covered paths](#the-covered-paths), [the verification root](#the-verification-root),
[the unread sources](#the-unread-sources) and [the gate](#the-gate) answer both. For MF3 the design covers:
every tracked file is a covered path unless one of three rules withholds it, so a script's data is in the key
however the script finds it. Refusal is kept for the one thing it can catch without fail, a step that runs a
script the key left out.

## What went wrong, and why a rule alone would not fix it

Read: [APPLE-TILE M2's review](../../evidence/APPLE-TILE/M2-review.md),
[APPLE-PRESENTATION M1's first review](../../evidence/APPLE-PRESENTATION/M1-review-1.md),
this design's [first](../../evidence/BASELINE-GUARD/design-review-1.md) and
[latest](../../evidence/BASELINE-GUARD/design-review.md) reviews, `land` in the build skill's
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
3. **The derived key.** The covered paths are worked out from the commit: every tracked file that no rule
   withholds. The verifier works where only they exist, and a proof stays good for a later commit that changes
   only withheld files and Rust sources no verified build read.

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
- **Run the work where only the declared inputs exist.** Bazel runs each action "in a working directory that
  only contains known inputs" ([sandboxing](https://bazel.build/docs/sandboxing)), so a read nobody declared
  fails instead of going unseen. It needs no record of what a program opens, and it holds for a script in any
  language. An earlier revision ruled this out because `APPLE-TILE`'s and `T2-CAPTURE`'s scripts hash the whole
  source tree into their records. That was wrong: they hash the files Git lists, and in a folder that is its own
  repository Git lists only what is there
  ([the planner's trial](../../evidence/BASELINE-GUARD/M2-plan/README.md#a-root-that-holds-only-the-covered-paths)).
- **A missing file fails only the read that needs it.** In such a folder a test for an undeclared file answers
  no and a listing leaves it out, so a script that reads a file only when it is there carries on without it.
  Bazel does not lean on the sandbox for this. It takes a folder's listing itself, when it loads the package,
  and every match becomes a declared input ([`glob`](https://bazel.build/reference/be/functions#glob)). An
  optional or listed input has to be among the inputs before the run; the run cannot be trusted to miss it.
- **Watching what a script opens was rejected.** Python and Node can report the files they open, but not what
  Git lists for them, what a Swift launcher or the game reads, or a test for a file that is not there. Each gap
  would need a rule of its own, and those same scripts open every source file to hash it. Nor can the verifier
  watch from outside: on macOS the tools that report another program's file-system calls need root
  ([`fs_usage`](https://keith.github.io/xcode-man-pages/fs_usage.1.html)).
- **So data is covered by where it is.** With no way to see a lookup, the only inputs that are safely in the
  key are the ones put there without asking what any script does. A hand list of what to leave out fails safe:
  a file nobody thought of is covered.

## Mechanism

### The registry

One file, `.config/exact-baselines.json`, lists each registered baseline: its identity, its owner, its retained
reference files, and how a fresh candidate is produced and compared. The table in
[the module list](../ARCHITECTURE.md#exact-baselines) is the same registry for people; the two must agree.

It also names the roots the covered paths are derived from: the packages the verifier builds, and the scripts it
starts. The verifier takes its build commands and its comparison scripts from the same entries, so what is built
and what is keyed cannot drift apart. The registry may list a further path outright, a file or a folder, for a
withheld file a step is meant to read or run. A listed folder covers every tracked file under it. The registry
has no entry that removes a path.

### The covered paths

The **covered paths** of a commit are its tracked files, less those a rule below withholds. They are derived
from the commit with Git and Cargo's resolver, with no build. A file is covered for where it is and what kind
of file it is, never for how a program finds it: no rule asks whether a script names a file, computes its
path, tests for it or lists its folder. A tracked file in a place no rule mentions is covered.

Three rules **withhold** a file:

1. **It belongs to a package Cargo does not link.** Cargo resolves the workspace at that commit with every
   feature on and for every target platform, so no feature or target the verifier picks can link a package the
   derivation missed. A **linked package** is a package the registry says the verifier builds, or one reachable
   from it over normal and build dependencies, whose source is a path. Dev-dependencies are not followed,
   because `cargo build` does not compile them. Every tracked file in a linked package's folder is covered: Rust
   sources, shaders, the web page, routes, presets and the showcase assets the cooker reads. Every file in the
   folder of a workspace package that is not linked is withheld, except its `Cargo.toml`, because Cargo reads
   every member's manifest to load the workspace. Packages from crates.io are pinned by the lockfile.
2. **It is a document:** a file under `docs/`, `.agents/` or `.claude/`, or a Markdown file outside the
   workspace packages.
3. **It is a script nothing reaches.** A script is a Python, JavaScript, shell or Swift source file outside the
   workspace packages and the documents. A script **reaches** a file by importing it or by naming the file's
   path in a quoted string. The verifier's own files and the scripts the registry names are reached, and so is
   every script a reached script reaches. The one script name resolved at run time, the native matrix, comes
   from the registry. A script nothing reaches is a **withheld script**.

A file one of these rules withholds is covered after all when the registry registers it as a reference or
lists it outright, or when a reached script reaches it. A quoted folder brings back nothing.

So the covered paths are the build configuration (the workspace `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, `.cargo/` and every package's manifest), the linked packages whole, the reached scripts,
the registry, the references and the listed paths, and every other tracked file outside the packages and the
documents: a data file beside a script, the files under `.config/`, the Git attribute and ignore files. Eleven
files are covered today only as that last kind, and 83 scripts are withheld
([the planner's count](../../evidence/BASELINE-GUARD/M2-plan/README.md#covering-by-place)).

### The verification root

Covering a file is half of the answer. The other half is that nothing the verifier runs can be swayed by a
file that is not covered. So the verifier does none of its work in the worktree. It fills a **verification
root**: a scratch folder that holds the covered paths, taken from the commit's objects in Git, a tripwire in
place of each withheld script, and no other tracked file. Every build, the cook, every capture and every
comparison runs there, from the root's own copies of the scripts.

What a step finds in the root depends on the kind of file:

- **A covered file is there as the commit has it.** A step reads it, tests for it and finds it in a listing
  as it would in the worktree, however it came by the path, and the key holds its content. This is what covers
  a script's optional and listed inputs. [The three cases the review traced](#check-that-it-can-work) need no
  registry entry.
- **A withheld script is there in name only.** Its **tripwire** is a file at the script's path that does one
  thing when a step runs or imports it: it leaves a mark in the verifier's scratch, outside the root, and
  stops. The verifier writes no proof while a mark is there, so a step cannot run a withheld script and keep
  its result by catching the failure. The key holds the path of every withheld script, so one that appears,
  goes or is renamed changes the key. Its content never does.
- **Any other withheld file is not there:** a file of a package that is not linked, a document and, once the
  builds are done, an [unread source](#the-unread-sources). A step that needs one fails, and no proof is
  written. A step that only tests for one, or lists its folder, goes on without it;
  [what it does not stop](#what-it-does-not-stop) says why that is left.

When a step fails for a withheld file it is meant to read, or trips the tripwire of a script it is meant to
run, the path is listed in the registry, which makes it a covered path.

Five properties make the root work:

- **Its files are copies, never links.** The owners' scripts find the repository from their own resolved
  location, so a link would lead them back to the worktree.
- **Git, asked from inside it, answers for the root alone.** It lists the root's files, reports a clean tree
  and names a commit of the root's own. Their manifests name the root's commit; the verifier's record names
  it beside the repository's revision.
- **Whole-tree hashing describes the root.** The owners' scripts hash every file Git lists under `crates/`,
  `examples/` and `scripts/`, and one hashes every shader under `crates/`, into the records they write
  (`source_files`, `build_source_files`, `shader_hashes` and the digests made from them). In the root those
  hashes cover the root's files, tripwires included. They are provenance, not inputs: no comparison reads
  them, so they change no verdict. A script that did compare one with a retained record would fail in the root
  on untouched main, where M2.6 would show it.
- **Cargo can load it.** Cargo refuses a workspace when a member's manifest is missing, or when a target of any
  member has no source file. Every member's manifest is a covered path. Wherever Cargo names a target's root
  file that the root does not hold, an empty file stands in for it. The stand-ins are gone before the cook.
- **It is kept between runs and brought up to date in place,** so Cargo rebuilds only what changed.

The part of the verifier that runs in the worktree checks that the tree is clean, derives the covered paths,
fills the root, looks for marks and writes the records and the proof. It runs no build, script or comparison
there. The gate makes no root.

### The unread sources

Inside a linked package, a Rust file may be compiled only for tests, benches, examples or another configuration.
Cargo records which files the compilers read: beside each artefact it writes a dependency record that lists
every source file and every file an `include_str!` or `include_bytes!` pulled in.

After its builds, the verifier reads the records of the artefacts it built: the native game, the cooker and the
Wasm module. An **unread source** is a covered `.rs` file, in a linked package that has no build script, that
none of those records lists and the registry does not list outright. Nothing else can be unread:

- A file that is not Rust stays covered, read or not. The cooker, the page build, the scripts and the game at
  run time read such files and leave no record.
- A package with a build script stays covered whole. The script can read any file in its package without
  declaring it, and Cargo itself treats the whole package as the script's input in that case.

A compiler cannot read a tracked file that is not a covered path, because the root does not hold it: the build
fails. The verifier names the cause and writes no proof when a record is missing, when one lists a file outside
the root and its build folder, or when a linked package's source is a path outside the repository.

Once the unread sources are known the verifier deletes them from the root, so no later step can read one as
data. It then runs the page build again: Cargo has nothing to compile, and the page is assembled from what is left. From
the cook on, the root holds exactly what the content key names, the files it covers and a tripwire at each
withheld script's path, and the verifier checks that from disk.

### The content key and the proof

The **content key** of a commit, under a list of unread sources, is a digest of the path and Git object ID of
every covered path at that commit that is not on the list, and of the path alone of every withheld script.
With an empty list it covers every covered path.

A **proof** is the record the verifier writes when every registered comparison passed on a clean tree. It holds
its unread sources and names the content key of its commit under them. It is kept untracked in the worktree, and
it is the only thing the gate accepts.

### The verifier

One command, from a clean checkout:

1. refuses a tree with uncommitted changes, and reports low disk space as its own named failure;
2. derives the covered paths and fills the verification root from the commit, tripwires included. Every later
   step runs in the root;
3. builds the release game with the features the owners' scripts need, and builds the Wasm page with
   `examples/v1-game/web/build.sh`;
4. reads the dependency records of those builds and works out the unread sources. It deletes them from the
   root, runs the page build again, removes the stand-ins, and checks that the root's tracked content, hashed
   from disk, gives the content key;
5. cooks the showcase valley fresh and runs `APPLE-TILE`'s native matrices (`apple_tile_m1.py`,
   `apple_tile_m2.py`) unchanged;
6. captures the four Chrome `fixed` runs with `capture_v1_browser.mjs` and compares each with its retained set;
7. exits non-zero on any difference, naming the baseline, its owner and the comparison;
8. on a full pass hashes the root's content again and looks for tripwire marks, and writes the proof only if
   the content still gives the content key and there is no mark. The proof holds the revision, the content key,
   the unread sources, the native binary's hash, the Wasm artefact hashes and the cooked root.

It takes every input from the root and its own fresh outputs. It never reads another task's scratch. The
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
2. If the content key with no source left out is the same at the commit and at its merge base with the remote
   main branch, it passes: no file covered at either commit differs, and no withheld script was added, removed
   or renamed. Main's own covered content was proved when it landed.
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
- **A rebase that brings in nothing a build reads keeps the proof.** A change inside a package Cargo does not
  link, to the content of a script nothing reaches or to a document outside the linked packages and the listed
  paths changes no covered path. A change to an unread source is
  left out of the key on both sides. The key is equal and the proof stands.
- **Why an equal key means equal builds.** Equal keys under one unread list mean the same build configuration,
  the same manifests and so the same linked packages, and every covered file identical except files on the list.
  No covered file was added, because a new file is on no list, and none off the list was removed. A compiler reads a file only as a target's root or
  because a file it already read names it. Roots come from the manifests, which are equal, and from files Cargo
  finds by name; a newly found file is a new file, and changes the key. Every file read before is identical, so
  each names the same files as before. The builds therefore read the same bytes with the same configuration.
- **Why an equal key means an equal verdict in the root.** The only tracked files outside the key that any
  step can reach are the unread sources, and only while Cargo builds; the page is assembled again once they are
  gone. From the cook on, the root holds the key's files, a tripwire at each path the key names for a withheld
  script, and no other tracked file, checked from disk before the cook and after the last comparison. Two
  commits with equal keys give the cook, the game, the captures and the comparisons the same files at the same
  paths, the same listing of every folder and the same answer to any test for a file. A step that ran a
  tripwire left a mark, and no proof exists. What is left to differ is outside the repository.
- **Why the root's verdict is the worktree's.** A step could come out differently in the worktree only by
  finding there a tracked file the root lacks. Every such file is withheld, and there are four kinds. A
  withheld script cannot be run without leaving a mark. A file of a package that is not linked, a document and
  an unread source are absent, so a step that needs one fails. One case is left: a step that takes input from
  one of those three without needing it, or reads a withheld script as text. It is an entry under
  [what it does not stop](#what-it-does-not-stop). It cannot arise for any other file, because every other
  tracked file is in the root.
- **The three cases the review traced.** A registered comparison script reads a tracked JSON beside itself:
  always, through `Path(__file__).with_suffix('.json')`; only if it exists; or by listing a folder of them. In
  each case the JSON is outside the packages and the documents and is not a script, so no rule withholds it.
  The root holds it, the script reads it there as it does in the worktree, and changing it changes the key,
  with no registry entry. A JSON added to the listed folder is covered from the commit that adds it.
- **The same script, importing a module beside itself by a name it computes.** If nothing else reaches the
  module it is a withheld script. The import trips its tripwire, and no proof is written until the registry
  lists it. A module of that name added later is a new withheld script, which changes the key. A tripwire left
  its mark under a Python, a Node and a shell step, each written to swallow every error
  ([trial](../../evidence/BASELINE-GUARD/M2-plan/README.md#covering-by-place)).
- **The root was tried.** At main `1630751e` a folder holding only the covered paths, with an empty file at
  each of the 56 target roots of the 14 packages that are not linked, gave Cargo the same resolved graph and
  the same targets as the worktree, and built the cooker. As its own Git repository it held 577 files in
  67 MB. The matrices, the native adapter and the Chrome comparator loaded from it, the cooker cooked the
  showcase in it, and Git listed only its files. No GPU run was made there; M2.6 makes it
  ([trial](../../evidence/BASELINE-GUARD/M2-plan/README.md#a-root-that-holds-only-the-covered-paths)).
  That folder was filled before files were covered by place, which adds eleven small files and the tripwires.
- **The numbers.** Cargo links nine packages into the two the verifier builds, and resolves the graph in 0.12 s.
  The dependency records of one verifier build list 251 repository files, every one inside those nine packages.
  176 of their 375 Rust files are unread. Imports and quoted paths reach 23 of the 107 files under `scripts/`.
  Replayed over the night of 14 to 15 November, the key changes at 10 commits where the listed key changed at 33
  ([measurements](../../evidence/BASELINE-GUARD/M2-plan/README.md)). By a rougher estimate, covering by place
  and keying the names of withheld scripts adds one commit to that night's count and six in the 493 commits
  since 11 November. Covering `scripts/` whole would add 28
  ([estimate](../../evidence/BASELINE-GUARD/M2-plan/README.md#covering-by-place)).
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
  each is covered, the withheld scripts and, when a proof covers the commit, the unread sources.
- `content_key(root)`: the commit's content key under the proof that covers it. With no such proof, the key
  with nothing left out.
- `proof_path(root)`: the file of the proof that covers the commit. With no such proof, a path that does not
  exist.
- A proof's fields `passed`, `content_key`, `baselines`, `record` and `revision`, and the same `content_key` in
  the record it points to. After a rebase the proof still covers, `revision` is the commit the verifier ran on,
  not the current one.
- The registry's listed paths. A module may add a path there for a script it owns, when the script reads a
  withheld file or runs a withheld script, and says so in its log. Nobody removes one without a ruling in the
  module list. An owner's script run by the verifier runs in the root: the manifests it writes there name the
  root's commit, and the verifier's record gives the repository's revision. Run directly, it behaves as before.
- Where a comparison's data may live. A tracked data file beside a script, in a linked package or anywhere
  else outside the documents is covered without being declared. A file under `docs/` is covered only when the
  registry registers or lists it, or a reached script names its path.

The ruling on [a frame that varies under load](../ARCHITECTURE.md#a-frame-that-varies-under-load) compares the
content keys of a failed run and a passing one. Both records hold it, since each run finished its builds.
`GAME-SPLIT` M4 has the verifier hand `THRUM_GAMES` to the commands it starts, and M5 renames the game's
package; M5's change to the registry is the package's name and path, and the covered paths follow from it.

## What it does not stop

- A push of a commit that `./dev check` did not pass: `land --no-check`, or a plain `git push` after a rebase
  with no check. Rule 1 of the module list forbids both for a covered-path change.
- A forged proof. The proof is a local file, as `./dev check` is local code.
- A step that leaves the root on purpose: an absolute path into the worktree or another checkout, a path taken
  from the environment, or Git asked about another repository. The root contains reads made from a script's
  own location, the working folder and what Git lists there, which is how every script reads today.
- A step that takes input, without needing it, from a file the root leaves out: a file of a package Cargo does
  not link, a document the registry does not list, or an unread source. A test for such a file answers no and
  a listing leaves it out, so the step goes on as if the file had never existed, and a later change to the
  file keeps the proof. The same holds for a step that reads a withheld script as text without running it: it
  reads the tripwire. Nothing an unprivileged program can do on macOS shows that a step looked, so the only
  cure is to cover the file. The task rules that out for the unrelated crates and scripts, and covering
  `docs/`, which holds 9,163 evidence files, would lapse every proof at nearly every commit. A comparison's
  data belongs beside its script, in a linked package or in the registry, where it is covered.
- A Cargo build script that reads, as data and without declaring it to Cargo, a Rust file of another linked
  package that no compiler reads. Build scripts run before the unread sources are known, and Cargo does not
  run one again unless its own package changes. One linked package has a build script today, `thrum-app`,
  and it reads one C file of its own package. Closing this means building every package with a build script
  from nothing on each run.
- A build step that acts on whether a target file exists in a package that is not linked. The empty stand-ins
  are in the root while Cargo runs.
- A change outside the repository: macOS, the GPU driver, Chrome, Node, `wasm-bindgen`, or the user's Cargo
  configuration and registry cache. See risk 1.

The first two are caught after the fact by the claim-time item of rule 3, whose command a builder cannot
change, and by the next builder's verifier run, which fails on main. The next four would change an image only
through code a reviewer of that change reads; nothing catches them automatically.

## Cost

A landing that changes a covered file a build or comparison reads pays one verifier run on its final content:
two builds, a cook, the native matrices and four Chrome captures of 481 frames, 1,022 seconds when M1 measured
it. A rebase that brings in another such change pays it again. A landing or a rebase that touches only other
files pays the gate, which adds one Cargo resolve of about a tenth of a second to its Git queries. A change to
the workspace manifest, a package's manifest or the lockfile lapses every proof, a new workspace member
included. The times are recorded, not budgeted. The scripts need 16 GiB free.

Covering by place lapses a proof for a few changes no build or comparison reads: to `.gitignore` or
`.gitattributes`, the CI workflow, `clippy.toml`, the `dev` scripts, the shape baseline or a data file beside
an unrelated script, and when a script nothing reaches is added, removed or renamed. That is the price of not
asking what a script reads; the planner's estimate is six commits in 493.

The root adds its files (67 MB in the trial), a second page build that compiles nothing, and whatever Cargo
rebuilds because its sources moved. Disk is short on this machine, so the root's builds write to the worktree's
own build folder unless that breaks a build, and the dependencies are not built or stored twice. M2 records
the time and disk the root adds beside M1's figures.

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
8. **An owner's script cannot run in the root.** The trial loaded each one and ran the cook, but drew nothing.
   If a script needs a tracked file, the registry lists it. If it needs something the root cannot give, such
   as the repository's history, the builder ends `BLOCKED` with the cause and the proposal goes to the owner.
   No step moves back to the worktree.
9. **A game built in the root draws different bytes.** The sources and flags are the same and only paths
   differ. The sign is that untouched main passes M1's verifier and fails in the root; the builder ends
   `BLOCKED` with the comparison.
10. **A tripwire trips under a step that means no harm**, such as one that imports every module in its folder.
   The proof is refused until the registry lists what the step runs. If an owner's script runs every script
   there, the builder ends `BLOCKED` with the cause and the proposal goes to the owner.

## Limits on builders

- Never refresh, regenerate or edit a file under a registered baseline. If a baseline does not match on untouched
  main, end `BLOCKED` with the comparison; the refresh is its owner's.
- No tolerance, nearest-frame search, retry inside a run or skipped record, for any reason.
- Do not change `scripts/apple_tile_m0.py`, `apple_tile_m1.py` or `apple_tile_m2.py`, the byte mode of the Chrome
  comparator, `scripts/capture_v1_browser.mjs` or `examples/v1-game/web/build.sh`. They belong to other
  modules.
- When unsure whether a file is an input, cover it. Leave a file out only on the evidence this design names.
- Withhold a file only by the three rules. A fourth rule, another document folder or a wider meaning of
  "script" is a change to this design.
- When a step fails in the root for a file it does not hold, or trips a tripwire, list the path in the
  registry if the step is meant to read or run it. Never run a step in the worktree, link a root file to the
  worktree, add a file to the root that is not a covered path, or clear a mark, to get past the failure.
- Mutants, scratch commits and any demonstration of `land` never push to the real remote.
- Chrome needs a browser launch outside the Codex sandbox. If a launch is refused, leave the item open and say
  so; do not substitute a Mac result.

## Proof and builder handoff

`mac-metal` and `chrome-webgpu`. GPU runs hold the `gpu` lock. Scratch goes in `tmp/baseline-guard/`; selected
evidence in `docs/evidence/BASELINE-GUARD/`. Mutants run in scratch and are never merged. A demonstration of
`land` or of the gate across a rebase runs in a scratch clone whose remote is a local bare repository, never
against the real remote. The gate's tests build small Cargo workspaces of their own, offline, and need no GPU.
The root's tests make a root for such a workspace and run fixture scripts in it, through the code the verifier
uses, also with no GPU.

## Open questions

| Question | Settled by |
|---|---|
| Does Cargo rewrite an artefact's dependency record on a build that compiles nothing? | M2's builder, before relying on the records' dates. The design needs only that the record is right for the sources built |
