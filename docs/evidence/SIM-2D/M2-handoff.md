# M2 recovery note, 17 November 2026

Finish SIM-2D M2 with every builder result passing, merge through `agent_loop.py
merge --worktree .`, and leave the milestone `in_progress`. The lead runs the
merged checks and starts review. This note protects an active run from another
session restart; work is continuing.

The branch is `build/sim-2d`. Source checkpoint `119bd20ab` includes upstream
`c5737c700`, the core world-lane change that otherwise invalidates the cost source
key. The three repair commits are above main. They keep compilation outside the
Chrome lock, run all nine conformance requests in one browser session, omit unused
returned timelines, and reject debugger disconnects. No engine or retained trace
changed. Every M2 builder result is already `pass`; the current cost records must
be refreshed before finishing because the harness and upstream core source changed.

The final real Chrome conformance passed at `69395002b`; its receipt is committed
at `docs/evidence/SIM-2D/M2/chrome-webgpu.json`. It covers the profile vectors and
all eight Crowd/Mixed size and wire routes. After rebasing, the native and Wasm
artifacts still have exactly the hashes in
[the repair notes](M2/chrome-queue-repair.md). Eleven focused Python tests and the
JavaScript syntax check pass. `./dev check` passed before the rebase. The final
merge runs the gate on integrated work. The lead reported the other item checks
passing; do not repeat them merely because of the rebase.

Two current background jobs measure the integrated source at `119bd20ab`:

- `20261117-010823-22751` writes `tmp/sim2d/M2-costs-mac-metal.json`.
- `20261117-010823-22761` writes `tmp/sim2d/M2-costs-chrome-webgpu.json`.

Use the build script's `wait <job> --timeout 50 --lines 5` or `jobs` to collect
them. The script is
`/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.3.6/skills/orchestration/build/scripts/agent_loop.py`.
The jobs take their own device locks. Earlier direct-tool measurements were
stopped after a harness change or the core rebase; none is accepted evidence.

When both jobs pass, retain their complete records at the matching
`docs/evidence/SIM-2D/M2/costs-<target>.json` paths. The `costs --target <target>`
validator must pass for both, with receipts retained at the existing validation
paths. Refresh the current measurements in `M2/performance.md` and `M2.md`, keeping
the historical investigation and every tripwire. Add at most eight lines to the
module log. Commit the evidence and merge through the script. The milestone stays
`in_progress`; end with `DONE` only after `MERGED`.

The first large browser batch at `7ee85d580` used about nineteen minutes of
renderer CPU, then its connection was absent while the harness waited. A large
reply causing that disconnect is only an inference. Reply reduction plus explicit
disconnect failure passed the complete subsequent target run. Details and limits
are in the repair notes. The abandoned profile `tmp/sim2d/chrome-lDXcy3` was removed
after confirming that no process used it. Preserve other scratch and other
builders' worktrees.
