# Repo rules: the shapes held by check, bulk files out of the tree, names and layout

14 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation) ·
[the code review](../../evidence/code-review-2026-11-14/CODE-REVIEW.md) ·
[patterns](../../../.agents/skills/thrum-engineering/references/patterns.md)

## The request

The owner asked for the 14 November code review's fixes to be planned and driven ("can you not just plan and drive
that work?"). The lead's task for this module: add the review's mechanical `repo_check` rules beside the canonical
shapes already written in the patterns reference; plan renames and splits (themes T9 and T10) "as one planned
architecture milestone"; plan hygiene (T13) last. On finding H03 the decision is made: "no Git history rewrite …
bulk media and archives leave the tracked tree under docs/ASSET-LIFECYCLE.md's storage rules (untrack at HEAD, keep
selected small evidence), and repo_check stops new bulk binaries being committed. Keep evidence links that reviews
depend on resolvable."

## Purpose and who it is for

The review's root cause is that nothing named or checked the shape of recurring code, so each milestone copied the
nearest example and forked a variant. The patterns reference now names the shapes. This module makes `./dev check`
hold them, so a builder who copies an old shape is told at once, by name, with the section to read. Its users are
every builder and reviewer in the repository, and the owner, whose checkouts and worktrees stop carrying gigabytes
of media.

## What was measured while planning

- Tracked evidence at `1a7ee764`: 9,054 files, 3.71 GiB. 730 files are over 2 MiB and hold 2.32 GiB: 586 PNG
  (1.56 GiB), 90 MP4 (393 MiB), 10 MOV (188 MiB), 35 gz (128 MiB), 1 MKV, 8 JSON. The registered native baseline
  folder is 59 MiB.
- `scripts/repo_check.py` takes no arguments and runs every check; `./dev check` and `./dev check-docs` both call
  it. It already checks Markdown links, so an untracked file that a tracked page links to fails today.
- A design's approval is a hash of its `DESIGN.md`. Editing another module's design, even to fix a link, stops
  that module's builders until it is reviewed again.

## Mechanisms

**1. Shape rules are ratchets.** Each rule counts its violations in each file. A checked-in baseline,
`.config/shape-baseline.json`, holds the count each file had when the rule was added. A file may never hold more
than its baseline, and a file with no entry is held at zero. The baseline only goes down: a `tighten` command
rewrites counts to the present, and the check refuses a baseline entry above the present count, so a fix cannot be
undone quietly. The file holds one entry a line, sorted by path, so two builders who each lower a count merge
cleanly. So no rule needs the codebase cleaned first, no running builder is stopped by code it did not
write, and every fix is kept. A refusal names the rule, the file and line, the patterns section to read, and the
shape to use instead.

**2. The rules** (each is a text rule over Rust sources, exact enough to have a violating and a corrected fixture):

| Rule | Refuses | Theme |
|---|---|---|
| `string-errors` | `Result<_, String>` as the return of a `pub fn` in `crates/` and `examples/` | T3 |
| `package-reach` | A `#[path]`, `include!`, `include_str!` or `include_bytes!` whose target resolves outside the file's own package, and a manifest dependency of any kind from a package under `crates/` on one under `examples/` | T5 |
| `path-attribute` | Any `#[path]` attribute, which shares code outside the module tree | T6 |
| `library-env` | `std::env::var`, `var_os` or `vars` in a library crate's non-test source | T8 |
| `device-request` | A call to `request_adapter` or `request_device` outside the modules [HOST](../HOST/DESIGN.md) and [TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md) register as the owners | T4, T6 |
| `whole-file-gpu-gate` | A file-level `#![cfg(target_os = …)]` on a test file | T6 |
| `history-names` | A new file whose name starts with a milestone or gate code (`g2_`, `g2s_`, `g2r`, `t2_`, `g3`, `g5`, `m3b` and the like) | T9 |
| `evidence-size` | A tracked file under `docs/evidence/` over the cap, unless it is a registered exact-baseline reference. Added in M2, with the store that gives a refused file somewhere to go | T13 |

Other modules add rules through the same engine, each in the milestone that first needs it: `dev-only-crates`
(`TEST-SUPPORT`), `game-names` (`GAME-SPLIT`) and `layering` (mechanism 5). A rule is added only with its fixtures.

**3. One command, two uses.** `python3 scripts/shape_rules.py check` runs every rule against the baseline, and
`repo_check.py` calls it, so `./dev check` and `./dev check-docs` run it with no flag or variable that skips it.
`check --rule <name> --zero <path>…` fails on any violation of that rule under those paths, whatever the baseline
says. Other modules use it to prove a crate is clean (`REVIEW-ENGINE`, `REVIEW-RENDER`, `HOST`, `GAME-SPLIT`), and
the game gate uses it on a games repository.

**4. Bulk files leave the tree and stay reachable.** No history is rewritten, so every blob that was ever
committed stays in the object store. That makes Git history the durable store for what is already tracked:

- A file under `docs/evidence/` over the cap is removed from the tree at HEAD. Its path, byte count, SHA-256, Git
  blob ID and the last commit that held it go in a manifest, `docs/evidence/BULK.json`.
- `python3 scripts/evidence_store.py restore <path>` writes the file back to its path from the blob, as an
  untracked, ignored file. `check` verifies the manifest against the object store.
- The link check accepts a link to a path the manifest lists. So no review, record or design is edited, and no
  design's hash moves.
- Registered exact-baseline references never leave: the registry in `.config/exact-baselines.json` is the
  exemption list, and the verifier still passes.
- New bulk output stays out of Git. A builder keeps captures in `tmp/<task>/` as the storage rules say, commits
  the record that reproduces them (command, revision, hashes) with small selected stills, and, when a reviewer
  must see the bytes themselves, puts the file in the local store with `evidence_store.py put`, which adds it to
  the manifest with its hash. The local store is a folder beside the repository, `../thrum-evidence-store`, with
  no remote and no service. `restore` reads history first and the store second.

The cap is 2 MiB a file, the review's figure. At that cap 2.32 GiB of 3.71 GiB leaves.

**5. Names and layout, once.** Renames spread conflicts across every builder, so they land in one milestone, at a
point where the review's other fixes are in. It is a move with no change of behaviour: the exact baselines and
every test hold it.

- `thrum-render`'s `synthetic` module becomes `scene`, with its types, shader files, labels and docs, because it
  is the production scene renderer.
- Files, benches, scripts, environment variables and Cargo descriptions named for a milestone or gate take the
  name of what they do.
- A name that an open item's `check`, `./dev` or the baseline registry uses does not change. Where a file behind
  such a name moves, the old name still works (a Cargo `[[test]]` entry keeps the target name, a script keeps a
  forwarding stub), because a builder cannot edit a pinned check.
- God files are split by responsibility as pure moves: `synthetic.rs`, `graph.rs`, `runtime.rs` in render, and
  `runtime.rs`, `registered_world.rs` in core.
- Inside `thrum-render` the layers point one way: the frame runtime, graph, presenter and timing do not import the
  scene renderer or the atmosphere. A `layering` rule holds that at zero.
- `thrum-render` is not split into crates. The frame runtime still holds the scene renderer's types, and
  `RENDER-2D` M1 builds the seam that lets a second scene kind stand in; a crate split before that seam would be
  drawn in the wrong place. The layering rule gives the dependency direction now.

No other module's `DESIGN.md` is edited by any milestone here. Stale paths in approved designs are covered by the
"where things are" table in `thrum-engineering`, which the rename milestone updates.

**6. Hygiene** (T13, last): `scripts/` keeps at its top level only what `./dev`, an open check or the baseline
registry names, with everything else in named subfolders and an index; superseded documents under `docs/design/`
are marked and listed, with their paths unchanged; shared dependency pins move to `[workspace.dependencies]`;
orphan directories go; feature-gated code is compiled by `./dev check`.

## Interfaces

| Interface | For | Defined here |
|---|---|---|
| `shape_rules.py check [--rule R] [--zero PATH…]`, the baseline file, and how a module adds a rule with fixtures | Every module that proves a crate clean | Mechanisms 1 to 3 |
| `evidence_store.py check`, `restore`, `put`; `docs/evidence/BULK.json` | Every builder and reviewer | Mechanism 4 |

## What it must not do

- Change a rule's meaning to make a crate pass, or raise a baseline count.
- Remove a registered baseline file, or any file under `docs/plans/`.
- Edit another module's `DESIGN.md`, items or record.
- Rename what an open check names.

## Not built here

Fixes to the code the rules count: those are `REVIEW-ENGINE`, `REVIEW-RENDER`, `HOST`, `TEST-SUPPORT` and
`GAME-SPLIT`. Shader composition (T7), typed host configuration (T8) and the duplicated mechanisms of T11 get a
ratchet or nothing here; the review's order of attack does not schedule their fixes.

## Proof

Rules are proved by fixtures: a violating file the rule refuses and the corrected file it passes, plus growth and
tightening against a scratch baseline. Bulk removal is proved by the manifest check, a restore that reproduces
hashes, the link check and the exact-baseline verifier. The rename is proved by `./dev check`, the verifier and a
comparison of test names before and after.
