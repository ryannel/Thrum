# Asset and build lifecycle

These are manual contributor rules for this repository. The full [storage design](design/STORAGE-LIFECYCLE.md)
describes future admission, publication and collection tooling; that tooling does not exist yet.

For asset generation or optimization, create `tmp/<task>/` with a small record of owner, purpose, expected
peak disk use, selected inputs, intended retained output and expiry/closeout condition. Inspect available
space before bulk work. Keep candidate outputs and temporary build caches there; never scatter them
through runtime asset directories. Do not retain entire chats as evidence.

At selection, keep the chosen editable source/recipe, exact input/tool versions, license/provenance,
selected runtime outputs and compact quality evidence. Give the asset a stable package ID; record exact
dependency hashes. Keep quality tiers, skins, fitting options and collision products distinguishable.
Generated assets still require visual review and any needed collision/material/runtime validation.

Stage and validate a complete delivery before a game changes its exact asset pin. A library promotion
does not silently update all consumers. Preserve reimport overrides and the prior accepted delivery until
the replacement is verified. Keep required source masters in versioned storage or a declared durable
store; ignored local files are not a backup. Record actual recovery status and test restoration before
claiming reproducibility. No remote upload or paid service is implied by these rules.

At campaign closeout retain the chosen patch, exact baseline/candidate inputs, fixture, toolchain,
measurements, limits and rationale. Measure the integrated engine workload, not only the optimized helper.
Discard owned rejected variants, redundant captures and reconstructible build outputs after recording
selection and checking retained roots. Failed alternatives need a concise reason, not an archive of all bytes.

Cleanup is explicit and ownership-scoped. Inspect a dry-run list, active work and shared references first.
Never remove another worker's tree or cache, or the only copy of a selected master. Promotion and retained
root verification precede deletion. Use normal filesystem/Git tools for now; do not pretend the planned
asset CLI, leases, automatic GC or restore guarantees are implemented.

## Bulk review evidence

Evidence has three homes, and `docs/evidence/` is none of them; the merge refuses a new or changed file there.
A record of a run, a review capture or a log goes in the working folder, `.orchestrate/evidence/<MODULE>/`,
which every worktree on this Mac shares and Git does not hold. Data a check reads is test data, tracked in
`tests/fixtures/<module>/` for a script or beside the crate's tests for a Rust test, with the bytes it had;
the V1 game's checks read their baselines from [test fixtures](../examples/v1-game/tests/fixtures/), and the
exact image sets are listed in [the registry](../.config/exact-baselines.json). What left the tree is listed in
one manifest, [`.config/evidence-manifest.json`](../.config/evidence-manifest.json). A module's provenance
record is not evidence: it is tracked at `docs/provenance/<MODULE>.md`. Keep new captures in `tmp/<task>/`, with
a reproduction record naming the command, revision, tool versions and SHA-256 hashes, plus selected small
stills, in the working folder.

When a reviewer needs the actual bytes of a large capture, for example a 60 MB real-time movie, run:

```sh
python3 scripts/evidence_store.py put tmp/my-task/capture.mp4 --path docs/evidence/MODULE/capture.mp4
git add .config/evidence-manifest.json
```

The logical path is a name in the manifest, never a tracked file: the bytes go to the local store and nothing
is written under `docs/evidence/`. Commit the manifest and link the logical evidence path from the record.
The source stays in scratch until recovery is verified. A reviewer on this Mac, from any worktree, runs
`python3 scripts/evidence_store.py restore docs/evidence/MODULE/capture.mp4` and gets an ignored copy at that
path. Restoring verifies size and SHA-256 and refuses to overwrite different local bytes. Use a new logical
path for changed content. `THRUM_EVIDENCE_STORE` can override the store location for both commands.

The default local store is `thrum-evidence-store` beside the main checkout, located through Git's common
directory, so builders and reviewers share it. It has **no remote copy, backup or service**: a lost disk loses
new local evidence. `check` names missing local entries without failing; `restore` fails and says where the
file was originally put. Arrange a separate authorised transfer if a reviewer uses another machine.

Older bulk evidence removed from HEAD is listed as history entries in
[the manifest](../.config/evidence-manifest.json); entries marked `relocated` are past reviews' evidence paths
for files that became test data, and `put` refuses those paths. The records nothing read, which left the tree
on 21 November 2026, are in Git history and in the working folder. The same restore command recovers history
bytes from Git's object store in any full clone, with no history rewrite.
`python3 scripts/evidence_store.py check` verifies all history entries and all available local bytes.
Restored copies are ignored by the root `.gitignore`, and the manifest keeps existing links valid.

## Profiling recordings and temporary Python environments

Treat cleanup as part of the run, including success, failure and cancellation. Before profiling or creating a temporary environment, identify its output/cache locations and ownership, including tool-managed files outside the repository. Bound profiling duration and repetitions; sample disk use during long captures and stop your recording before it exhausts available space.

For Instruments/xctrace, set an explicit output destination and inspect the system temporary directory for scratch created by your run. Raw `instruments*.ktrace` and `xrgpu_aps_*` files can outlive the exported trace. Record the paths associated with your run; a filename pattern, timestamp or absence of an open handle alone does not establish ownership. At closeout, stop and wait for your recorder, verify the selected trace or measurements are readable in the evidence destination, then remove its unneeded raw recordings and GPU scratch. Keep decisive raw evidence needed for retained claims or an unresolved investigation, with a reason and release trigger. Do not stop shared profiling services or delete another run's files.

Reuse a compatible Python environment for related iterations. For a disposable campaign, prefer an explicitly scoped environment and cache, for example `UV_CACHE_DIR` under its scratch directory, rather than accumulating many environments in the global cache. Retain the script, dependency lock/requirements and tool versions; normally the environment itself is rebuildable. Close your workers before removing their environment. Use the package manager's cleanup command with in-use protections enabled; never force through a lock. Do not clear the shared global cache after every run or treat app-managed Python runtimes as disposable task environments. A shared-cache cleanup needs its own checked scope; process command lines, working directories and open files help identify consumers.

Use a `finally`/exit handler in repeatable profiling and environment scripts for owned disposable resources. On the next run, reconcile leftovers from interrupted runs before allocating more; a crash may bypass cleanup. If a retained file or active consumer prevents cleanup, record the exact path and follow-up condition in the existing handoff. Completion includes verifying that the owned scratch is gone or deliberately retained, and reporting significant deferred cleanup. This does not authorize deleting Codex conversations or app databases.
