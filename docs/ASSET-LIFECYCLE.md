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
