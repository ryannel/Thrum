# Repo rules: the shapes held by check, bulk files out of the tree, names and layout

14 November 2026, revised 15 November after [the first design review](../../../.orchestrate/evidence/REPO-RULES/design-review-1.md),
and again that day when WP1's first landing was refused ([log](LOG.md)).
Revised 24 November after [the migration review](../../../.orchestrate/evidence/REPO-RULES/design-review.md).
[Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation) ·
[the code review](../../../.orchestrate/evidence/code-review-2026-11-14/CODE-REVIEW.md) ·
[patterns](../../../.agents/skills/thrum-engineering/references/patterns.md)

## The request

The owner asked for the 14 November code review's fixes to be planned and driven ("can you not just plan and drive
that work?"). The lead's task for this module: add the review's mechanical `repo_check` rules beside the canonical
shapes already written in the patterns reference; plan renames and splits (themes T9 and T10) "as one planned
architecture work package"; plan hygiene (T13) last. On finding H03 the decision is made: "no Git history rewrite …
bulk media and archives leave the tracked tree under docs/ASSET-LIFECYCLE.md's storage rules (untrack at HEAD, keep
selected small evidence), and repo_check stops new bulk binaries being committed. Keep evidence links that reviews
depend on resolvable."

## Purpose and who it is for

The review's root cause is that nothing named or checked the shape of recurring code, so each work package copied the
nearest example and forked a variant. The patterns reference now names the shapes. This module makes `./dev check`
hold them, so a builder who copies an old shape is told at once, by name, with the section to read. Its users are
every builder and reviewer in the repository, and the owner, whose checkouts and worktrees stop carrying gigabytes
of media.

## What was measured while planning

- Tracked evidence at `1a7ee764`: 9,054 files, 3.71 GiB. 730 files are over 2 MiB and hold 2.32 GiB: 586 PNG
  (1.56 GiB), 90 MP4 (393 MiB), 10 MOV (188 MiB), 35 gz (128 MiB), 1 MKV, 8 JSON. The registered native baseline
  folder is 59 MiB.
- `scripts/repo_check.py` takes no arguments and runs every check; `./dev check` and `./dev check-docs` both call
  it. Its link check asks whether the target exists on disk inside the repository, not whether it is tracked. So
  once a file leaves the tree, a link to it fails in a clean checkout and passes wherever someone restored the
  file; the check has to ask the manifest, not the disk.
- Worktrees of this repository share one Git directory (`git rev-parse --git-common-dir` gives
  `/Users/ryannel/Workspace/thrum/.git` from a builder's worktree and from the loop's clean checkout alike) and
  nothing else: a path relative to a checkout names a different folder in each.
- Names that carry a work package or gate code today, under `crates/`, `examples/`, `tools/` and `scripts/`: 28
  files, 19 environment variables (`THRUM_G5_CAPTURE_DIR` and the like), two Cargo descriptions and one bench
  target. The codes in use are `g<n>` with an optional letter (`g2`, `g2r`, `g2s`, `g3b`, `g3s`, `g3t`, `g5`,
  `g6`), `t2`, `a1` and `m<n>` (`m0` to `m12`, `m3b`).
- A design's approval is a hash of its `DESIGN.md`. Editing another module's design, even to fix a link, stops
  that module's builders until it is reviewed again.

## Mechanisms

**1. Shape rules are ratchets.** Each rule counts its violations in each file. A checked-in baseline,
`.config/shape-baseline.json`, is the inventory: for each rule, the count in each file that has any, one entry a
line, sorted by path, so two builders who each lower a count merge cleanly. The check has two halves.

- **The tree.** Every file's count equals its entry, and a file with no entry has none. When a file's count
  differs, the check names the rule, the file and line, the patterns section to read and the shape to use.
  A builder whose change redistributes or reduces the count runs `tighten` to record the current counts.
  The history half decides whether their total is allowed.
- **The history.** The accepted state is what has already landed: for each rule, the lowest total its baseline
  has held at any commit on the first-parent history of `HEAD`, since the rule was registered. The present total
  may not exceed it. So a change that adds a violation and rewrites the baseline to match is refused, whether it
  is uncommitted, committed in a builder's worktree or already merged, because the low mark comes from Git and
  not from the file under review. The comparison needs no remote and no branch name, so it is the same in a
  builder's worktree and in the loop's clean checkout, which share the object store. The refusal names each file
  whose count is above its entry at the low mark.

Four cases are not growth, and the design names them so that no one needs a hand-made exception:

- **A rule is registered.** A rule is registered by the commit that brings it to the main branch. Its history
  starts there, at the first commit whose baseline lists it, with whatever counts the code on main has then. The
  baseline records each rule's version, which is part of the rule's definition. A change to what a rule matches
  is a new version, in the commit that changes the matcher, and starts its history again.
- **Main moves before a rule lands.** Until a rule is on main nobody else can run it, so a violation that
  reaches main first is code that was already there: it enters the baseline, and the rule holds it from then
  on. The check reads only Git history and cannot tell an unmerged branch from main, so the rule's builder lands
  a branch on which the rule is first listed in a commit that holds the counts of the main it lands on. When
  main gains or loses a violation while the rule waits, that commit is made again on the new main. Counts the
  branch held earlier were never on main and are not accepted state; a later commit that raises the baseline is
  still refused, as any raise is.
- **A file moves.** The ratchet is on a rule's total, so a move, a rename or a split carries a file's allowance
  to wherever its lines go and grants none. The same holds for a change that removes one violation and adds
  another: the repository is no worse, and it passes.
- **A violation slipped onto main past a registered rule.** The low mark is unchanged, so every check fails
  until the violation is removed, and then passes. Nothing has to be waived or rewritten.

In a tree with no Git history, such as the game gate's detached copy, only the tree half runs, and the check says
so. So no rule needs the codebase cleaned first, no running builder is stopped by code it did not write, and
every fix is kept.

**2. The rules** (each is a text rule, exact enough to have a violating and a corrected fixture):

| Rule | Refuses | Theme | Added in |
|---|---|---|---|
| `string-errors` | `Result<_, String>` as the return of a `pub fn` in `crates/` and `examples/` | T3 | WP1 |
| `package-reach` | A `#[path]`, `include!`, `include_str!` or `include_bytes!` whose target resolves outside the file's own package, and a manifest dependency of any kind from a package under `crates/` on one under `examples/` | T5 | WP1 |
| `path-attribute` | Any `#[path]` attribute, which shares code outside the module tree | T6 | WP1 |
| `library-env` | `std::env::var`, `var_os` or `vars` in a library crate's non-test source. One exemption: a package registered in `.config/architecture.toml` with `test_role = "support"`, which is a process edge ([TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md#mechanisms)'s GPU gate reads `THRUM_REQUIRE_GPU`). No other package and no other role is exempt | T8 | WP1 |
| `device-request` | A call to `request_adapter` or `request_device` outside the modules [HOST](../HOST/DESIGN.md) and [TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md) register as the owners | T4, T6 | WP1 |
| `whole-file-gpu-gate` | A file-level `#![cfg(target_os = …)]` on a test file | T6 | WP1 |
| `history-names` | A work package or gate code in a path segment under `crates/`, `examples/`, `tools/` or `scripts/`, an environment variable name in source, a Cargo target name or a Cargo description. The case-insensitive families are `g<n>` and `m<n>` with an optional letter, `t2`, `a1` and, from WP3, `wp<n>` (one or more digits). A code is bounded by the start or end of the name or a non-alphanumeric separator, including underscores | T9 | WP1, extended in WP3 |
| `evidence-size` | A tracked file under `docs/evidence/` over the cap, unless it is a registered exact-baseline reference. Added with the store that gives a refused file somewhere to go | T13 | WP2 |
| `layering` | In `thrum-render`, an import from the scene renderer or the atmosphere in the graph or timing, and a new one in the frame runtime or the presenter (mechanism 5) | T10 | WP3 |

WP1 proves the rules marked WP1. WP3 raises the version of `history-names` when it adds the current work package
code family and records the new counts. WP3 proves zero on every naming surface with the pins in mechanism 5
honored. The normal ratchet inventory still counts pinned names.

Other modules add rules through the same engine, each in the work package that first needs it: `test-crate-edges`
(`TEST-SUPPORT`) and `game-names` (`GAME-SPLIT`). A rule is added only with its fixtures.

**3. One command, two uses.** `python3 scripts/shape_rules.py check` runs every rule against the baseline, and
`repo_check.py` calls it, so `./dev check` and `./dev check-docs` run it with no flag or variable that skips it.
`check --rule <name> --zero <path>…` fails on any violation of that rule under those paths, whatever the baseline
says, and fails for a path that does not exist. Other modules use it to prove a crate is clean (`REVIEW-ENGINE`, `REVIEW-RENDER`, `HOST`, `GAME-SPLIT`), and
the game gate uses it on a games repository.

**4. Bulk files leave the tree and stay reachable.** No history is rewritten. Git history is the durable store
for evidence that was committed. Since DEAD-TOOLS WP3, the sole live manifest for the link check and all store
commands is the tracked [`.config/evidence-manifest.json`](../../../.config/evidence-manifest.json).
`.orchestrate/evidence/BULK.json` and the original paths in WP2's proof describe the historical WP2 landing.
See the [storage rules](../../ASSET-LIFECYCLE.md#bulk-review-evidence) for the current layout.

- A file under `docs/evidence/` over the cap is removed from the tree at HEAD. A **history** entry records its
  logical path, byte count, SHA-256, Git blob ID and a commit that held it. Historical paths of evidence that
  became test data are marked `relocated`. WP5 uses history entries for the three remaining tracked records.
- `python3 scripts/evidence_store.py restore <path>` writes the file back to its path, as an untracked, ignored
  file. A history entry comes from its blob in a full clone.
- The link check accepts a link to a path the manifest lists, whether or not a restored copy is on disk, and
  still refuses a link to an evidence path that is neither tracked nor listed. Existing links to the removed
  records keep their targets through the manifest.
- Registered exact-baseline references never leave: the registry in `.config/exact-baselines.json` is the
  exemption list, and the verifier still passes.
- New bulk output stays out of Git. A builder keeps captures in `tmp/<task>/` and the reproduction record
  and selected stills in the working folder. When a reviewer needs the bytes, `evidence_store.py put` stores
  them locally and adds a **local** entry with the logical path, byte count, SHA-256, date and store location.
  The builder tracks the manifest entry.
- **The local store is one folder for the whole repository**, `thrum-evidence-store` beside the main checkout,
  found through Git's common directory and so the same folder from every worktree: a builder's `put` is restored
  by the reviewer's clean checkout on this Mac. `THRUM_EVIDENCE_STORE` overrides the place. Files are stored
  under their SHA-256, so two worktrees that put the same bytes agree. It has no remote and no service.
- `evidence_store.py check` verifies every history entry against the object store, and fails for one whose blob
  is gone or whose hash differs. For a local entry it verifies the hash when the store holds the file. A local
  entry the store does not hold is reported by name as not on this machine: `check` still passes, because another
  clone cannot have it, and `restore` of that path fails and says where the file was put.

The cap is 2 MiB a file, the review's figure. At that cap 2.32 GiB of 3.71 GiB leaves.

**5. Names and layout, once.** Renames spread conflicts across every builder, so they land in one work package, at a
point where the review's other fixes are in. It is a move with no change of behaviour: the exact baselines and
every test hold it.

- `thrum-render`'s `synthetic` module becomes `scene`, with its types, shader files, labels and docs, because it
  is the production scene renderer.
- Files, benches, scripts, environment variables and Cargo descriptions named for a work package or gate take the
  name of what they do. `history-names` is what proves it, over every place it counts, contents included: a
  renamed test file that still reads `THRUM_G5_CAPTURE_DIR` is not done.
- A path or Cargo name that an open item's `check`, `./dev` or the baseline registry uses still works after the
  rename. For example, a Cargo `[[test]]` entry keeps the target name, or a script keeps a forwarding stub.
  `--zero` discovers and reports these pins from `docs/plans/*/DONE.json`, `tools/xtask` and
  `.config/exact-baselines.json`. An item's name pin expires when its work package is Done.
- The APPLE-TILE comparison registered in `comparison_files` at `scripts/apple_tile_m0.py` sets
  `THRUM_G3T_OVERLAY_CONTROL` and `THRUM_G3T_OMIT_PLACEMENT`. WP3 keeps the comparison bytes and the V1 game's
  capture reads and meanings of those exact keys. `--zero` reports their compatibility pins with the registered
  comparison as the source. The pins apply only to occurrences in that comparison and its V1 capture
  configuration consumers, and only while the file is registered and uses the key. The rule still checks
  other code-bearing names in those files and the same keys in unrelated code.
- God files are split by responsibility as pure moves: `synthetic.rs`, `graph.rs`, `runtime.rs` in render, and
  `runtime.rs`, `registered_world.rs` in core.
- Inside `thrum-render` the `layering` rule holds what moves and the other modules' fixes can deliver, and no
  more. The graph imports nothing from the scene renderer or the atmosphere today, and timing stops once
  [REVIEW-RENDER](../REVIEW-RENDER/DESIGN.md) WP3 gives passes their own timing categories: for those two the rule
  is absolute. The frame runtime and the presenter are different. The frame host is created from a scene, holds
  the prepared scene renderer and its offscreen target, and its capture holds the renderer's readback; the
  presenter binds that target. A rename and a split leave those dependencies where they are, and moving
  scene code under a neutral file name would only hide them. For those two the rule is a ratchet: their imports
  are counted when the rule is registered and may not grow.
- Taking the runtime and the presenter to zero needs the frame host to hold its scene through an interface. No
  module plans that: `RENDER-2D` WP1 lets a host be created for a 2D or a 3D scene, and the host still names both.
  It is not built here, and it is open for the human in the plan.
- `thrum-render` is not split into crates, for the same reason: a crate boundary between the frame runtime and
  the scene renderer cannot be drawn while one names the other's types.

No other module's `DESIGN.md` is edited by any work package here. Stale paths in approved designs are covered by the
"where things are" table in `thrum-engineering`, which the rename work package updates.

**6. Hygiene** (T13, last): `scripts/` keeps at its top level only what `./dev`, an open check or the baseline
registry names, with everything else in named subfolders and an index; superseded documents under `docs/design/`
are marked and listed, with their paths unchanged; shared dependency pins move to `[workspace.dependencies]`;
orphan directories go; feature-gated code is compiled by `./dev check`.

**7. The migrated plans pass the repository gate.** Since the 3.0 migration, both shape-rule readers use
`work_packages` in the done-when files. The caller checks read open and Done work packages. The naming
proof reads pins from open work packages. WP5 restores the readers under the existing rule versions and
baseline and removes the three legacy records under mechanism 4. The remaining ATMOSPHERE link is an
owning planner's plan-only correction before WP5's checked merge. A plan-only merge runs independently of
the repository gate, and the resulting ATMOSPHERE design review applies to its own builders. WP5 is the
first landing that requires the repaired gate to pass. It has no dependency on WP3 or WP4.

## Interfaces

| Interface | For | Defined here |
|---|---|---|
| `shape_rules.py check [--rule R] [--zero PATH…]`, the baseline file, and how a module adds a rule with fixtures | Every module that proves a crate clean | Mechanisms 1 to 3 |
| `evidence_store.py check`, `restore`, `put`; tracked `.config/evidence-manifest.json` with its history and local entries; the one local store beside the main checkout | Every builder and reviewer | Mechanism 4 |

## What it must not do

- Change what a rule matches to make a crate pass, or let a registered rule's total rise.
- Remove a registered baseline file, or any file under `docs/plans/`.
- Edit another module's `DESIGN.md`, items or record.
- Rename what an open check names.

## Not built here

Fixes to the code the rules count: those are `REVIEW-ENGINE`, `REVIEW-RENDER`, `HOST`, `TEST-SUPPORT` and
`GAME-SPLIT`. Shader composition (T7), typed host configuration (T8) and the duplicated mechanisms of T11 get a
ratchet or nothing here; the review's order of attack does not schedule their fixes.

## Proof

Rules are proved by fixtures: a violating file the rule refuses and the corrected file it passes. The ratchet is
proved in a scratch Git repository, where a commit that adds a violation and rewrites the baseline to match is
refused before and after it is committed, and a move, a new rule and a new rule version pass. A rule built on a
branch whose base then gains a violation is refused while the branch still holds the lower count, and passes once
the rule is first listed with the new base's counts. Bulk removal is
proved by the manifest check, a restore that reproduces hashes, a file put from one worktree and restored in
another, the link check and the exact-baseline verifier. The rename is proved by `./dev check`, the verifier,
`history-names` at zero and a comparison of test names before and after.
