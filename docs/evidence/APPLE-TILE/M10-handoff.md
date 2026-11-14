# APPLE-TILE M10 continuation

Resume this worktree. M10 remains `in_progress`; D10.1 (native boundary) and D10.3
(retention/discard/reset) pass. D10.2 (T2 diagnostic output), D10.4 (simulator race)
and attended D10.5 remain open. Checkpoint `1b00d4ee` retains the passing target
proofs. The [builder record](M10/README.md) links their complete compact evidence.
No merge or complete milestone acceptance is claimed.

The latest engine source is `fe809c7f`. It ends the native-only qualification hold
after both backgrounding and lease revocation become visible; ordinary stalls
retain their previous behavior. The simulator release app and target Clippy pass.
The source in `07282556` passes the exact baseline campaign (38 ordinary native
comparisons, four surface/letterbox comparisons, 1,924 Chrome frame records) and
the official M10 check with full `./dev check`, outside the sandbox under the GPU
lock. This includes all four integrated Metal native tests, graph restrictions,
fixture API separation and iOS lease/input/world-pause unit tests. No pixel
tolerance or retained reference changed, and no speed/MetalFX/panel claim follows.

The earlier small same-submit fixture passed at `cdede389` before route integration.
The earlier coordinator attempt stopped on capture accounting, before pixel
comparison: its three deliberate candidate discards were incorrectly expected to
produce zero omissions. `342151fc` requires exactly three. The corrected
coordinator test now passes exact SDR/scene/output/history comparisons and rejects
all five omission/stale/reset controls in the full gate.

The configured script is
`/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.2.2/skills/orchestration/build/scripts/agent_loop.py`.
Background job `20261114-063318-66523` finished with exit 0 (baselines and full
official M10 check). Background job `20261114-072259-82489` is still queued for
the shared GPU. Use the script's `wait <id> --worktree .` command. It runs the
corrected coordinator test, `scripts/apple_tile_m1_sim.py --native-witness`, then
`scripts/apple_tile_m10.py`; results go to `tmp/g3t-m10/simulator` and
`tmp/g3t-m10/diagnostic`. At the last observation the Mac runtime builder's
repository gate actively held the GPU, after CULLING completed its oracle.
No M10 deadlock was observed. Do not launch duplicates or change/rebase engine
source while the diagnostic campaign is active.

After that job finishes, diagnose any failure, inspect its actual source frames
and transition samples against `tmp/g3t-m10/inspection-plan.md`, retain compact
provenance/results and selected original images in `docs/evidence/APPLE-TILE/M10`,
and mark only proved items pass. The simulator proves diagnostic function only;
do not claim submitted temporal output or speed. Its app build completed in job
`20261114-080040-90655`. The native release executable hash is recorded with the
baseline proof. The cooked store is `tmp/g7-showcase/runs/moving-20261114-033128/store`.

Integrate through the official `merge --worktree .`, inspect `upstream_files`,
and repeat proofs affected by upstream changes. The existing
`tmp/apple-tile-m3-resume/check-or-merge.py` wrapper serializes Cargo GPU tests
after taking the integration lock; do not take an outer GPU lock around merge.
The last fetch showed no upstream engine, M10-runner or baseline-registry changes,
but check again at integration. Leave M10 `in_progress`.

D10.5 needs the owner's unlocked active Mac session and an existing Screen
Recording grant for the OS-window observer. This background session cannot obtain
that setup; do not automate grants. Complete every other item, then report
`BLOCKED needs_access` naming that setup, with this handoff committed.

Scratch ownership is in `tmp/g3t-m10/OWNERSHIP.md`. Preserve selected M10 sources
through review and failed captures needed for diagnosis. Earlier M3 bulk cleanup
is documented by `M10/m3-expired-bulk-release.json.gz`; selected evidence and newer
integrated sources were preserved. No Instruments recording, temporary Python
environment or grant change was made. Do not delete another builder's artifacts.
