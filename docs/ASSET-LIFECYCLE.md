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

## Profiling recordings and temporary Python environments

Treat cleanup as part of the run, including success, failure and cancellation. Before profiling or creating a temporary environment, identify its output/cache locations and ownership, including tool-managed files outside the repository. Bound profiling duration and repetitions; sample disk use during long captures and stop your recording before it exhausts available space.

For Instruments/xctrace, set an explicit output destination and inspect the system temporary directory for scratch created by your run. Raw `instruments*.ktrace` and `xrgpu_aps_*` files can outlive the exported trace. Record the paths associated with your run; a filename pattern, timestamp or absence of an open handle alone does not establish ownership. At closeout, stop and wait for your recorder, verify the selected trace or measurements are readable in the evidence destination, then remove its unneeded raw recordings and GPU scratch. Keep decisive raw evidence needed for retained claims or an unresolved investigation, with a reason and release trigger. Do not stop shared profiling services or delete another run's files.

Reuse a compatible Python environment for related iterations. For a disposable campaign, prefer an explicitly scoped environment and cache, for example `UV_CACHE_DIR` under its scratch directory, rather than accumulating many environments in the global cache. Retain the script, dependency lock/requirements and tool versions; normally the environment itself is rebuildable. Close your workers before removing their environment. Use the package manager's cleanup command with in-use protections enabled; never force through a lock. Do not clear the shared global cache after every run or treat app-managed Python runtimes as disposable task environments. A shared-cache cleanup needs its own checked scope; process command lines, working directories and open files help identify consumers.

Use a `finally`/exit handler in repeatable profiling and environment scripts for owned disposable resources. On the next run, reconcile leftovers from interrupted runs before allocating more; a crash may bypass cleanup. If a retained file or active consumer prevents cleanup, record the exact path and follow-up condition in the existing handoff. Completion includes verifying that the owned scratch is gone or deliberately retained, and reporting significant deferred cleanup. This does not authorize deleting Codex conversations or app databases.
