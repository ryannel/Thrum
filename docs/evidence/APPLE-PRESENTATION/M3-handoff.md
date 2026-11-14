# APPLE-PRESENTATION M3 continuation

The task is to qualify M3 on its real Mac/Chrome targets, merge through
`agent_loop.py merge --worktree .`, and leave M3 `in_progress`. This continuation
found a visible reveal-edge defect after the earlier still/cut passes. The current
reciprocal-depth checkpoint is an experiment, not a qualified replacement. All M3
items are open until current target evidence exists. No planner field or bound
was changed, and no merge or milestone reviewer was started.

The preceding range-based kernel is preserved by `d2680e27` and its ancestors.
At `c3d052a4` it passes all six still-reference comparisons and both cut controls.
At `3f896417` it passes the paired 361-frame 640×360 timing run: resolve p50/p95
0.046709/0.049792 ms. At `959ada11` it passes all 1,966 exact comparisons in
939.25 seconds. The reports and target captures are committed under
`docs/evidence/APPLE-PRESENTATION/m3/`. Those measurements do not qualify the new
kernel. The independent Chrome opinion is
`m3/motion/chrome-review-independent.md` (GPT-6.1 Sol, medium).

Both readers see a fine ragged/speckled fringe around newly revealed small
blocks and the moving cube's lower/trailing border. Chrome reveal ordinals
144–155 show it; Mac has the same imperfection. This is a concrete quality issue,
not a demonstrated Chrome-only fault. Spawn appears solid immediately in the
inspected frames; all-frame star-core traces are stable, and both targets' scrub
sky diagnostics fit zero lag. These diagnostics have no acceptance thresholds.
Neither reader claimed direct continuous 1× visual perception; decoder execution
is kept distinct from source-frame inspection.

The new shader interpolates reciprocal previous view depth rather than accepting
the entire four-texel depth interval. Its causal connection to the fringe is still
a hypothesis. `m3/depth-reprojection/README.md` gives the independent projected-plane
argument, the unchanged cost/resources, and the target probe's old-range negative
control. Production/probe WGSL validation, strict renderer Clippy and
`./dev check-web` pass. The Metal probe and new still/motion/timing results remain
owed. Restore the range candidate only if the new attempt is worse; keep its
visible defect open rather than treating a rollback as a motion pass.

After committing this checkpoint, the builder queues
`python3 docs/evidence/APPLE-PRESENTATION/m3/depth-reprojection/run_candidate.py --output tmp/apple-presentation-m3/depth-candidate-1`
under the script's `background -- ... lock gpu -- ...`. Use `agent_loop.py jobs
--worktree .` to find that job and wait for it; do not restart a live job because
a wait times out. It stops at the first failing phase and saves `run.json` plus
phase logs. It runs the Metal probe, held reconstruction/cut oracle, web build,
fresh cook, exact verifier, both full motion campaigns, timing pair and moving
route. It expects one clean source throughout. If no such job exists, the turn
may have ended between commit and launch; launch it once on the clean checkpoint.

The separate job `20261114-091901-13629` has passed the moving command on the
development build at `tmp/g7-showcase/runs/moving-20261114-095335/run.json`.
Its source was dirty, so treat it as diagnostic and retain the bundled clean
rerun. All four inherited jobs from the session prompt have now exited 0.

The failed range candidate's original bulk frames were reverified and released
to recover 5.099 GiB. Exact paths and retained hashes are in
`m3/motion/source-release.json.gz`. Selected clips, native crops, all-frame
diagnostic atlases, manifests/runtime records and recipes remain committed.
Full original frame recovery is not claimed. Do not rerun audits against the
old source directories expecting those bulk files. Retained audit/staging tools
accept `--source` and `--output` for the new candidate paths. Scratch notes in
`tmp/apple-presentation-m3/README.md` also name the current job once launched.
No temporary environment or recorder outside the repository was created here.
Keep this worktree's target while the queued probe/builds need it.

After a successful candidate run, inspect the new reveal sequence first, then all
five pairs and the full native-size crops on both targets; get a fresh opinion
as the build skill requires. Promote selected new evidence before final checks.
Preload item memos with `agent_loop.py check --module APPLE-PRESENTATION
--milestone M3 --worktree . --items-only`; only after all memos match the clean
tree may the full checker run under an outer GPU lock. Otherwise its item locks
would deadlock. Non-plan evidence edits invalidate those memos; plan LOG/DONE
edits do not. Run the required full checker and merge, inspect `upstream_files`,
and repeat affected proofs. A rebase may require another exact content-key proof.
Never bypass the baseline guard or set `ready_for_review`/`done`.
