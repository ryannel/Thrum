# SEA M2 recovery note

Finish M2 with every item proved on its target and the work merged through
`agent_loop.py merge --worktree .`. Leave its status `in_progress`; the lead's
script owns the later status and review. M2.1–M2.7 and M2.9 are passed. M2.8,
the complete exact native/Chrome baseline verification, remains open.

The integration source is now based on main's CLOUDS M1 work. Rebase conflicts
in `Cargo.toml` and `.config/architecture.toml` were resolved by retaining both
modules' registrations. The resulting source checkpoint is `9bcfed37`. Cargo
metadata includes `thrum-sea`, `thrum-sea-model` and `thrum-cloud-model` together;
documentation checks pass. A comparison with pre-rebase `327546b8` shows no
change to the sea crates or renderer. SEA has not printed `MERGED` or pushed.

Use the latest live job for [the qualification helper](qualify_m2_baselines.py),
listed by `agent_loop.py jobs --worktree .`, rather than launch another duplicate.
The helper runs under `lock gpu`, updates only this branch to current main, and
uses the repository's own baseline gate. It verifies when a valid proof is
missing. Below 18 GiB it inspects and cleans only this worktree's private target
directory, refusing a linked target. That leaves build headroom above the native
observer's unchanged 16 GiB requirement. Leave the worktree unchanged
while it rebases or verifies. At `AWAIT_EVIDENCE`, make and commit the evidence
changes described below; the helper no longer changes the branch in that phase.

The helper now keeps its GPU slot for the final evidence and integration phase.
Wait with `wait <job> --worktree . --until AWAIT_EVIDENCE:` to observe that phase
without waiting for the whole job to end. On that marker, read
`tmp/sea-m2/qualification-ready.json`. Inspect and copy `proof_file` exactly to
`retained_proof_file`, update M2.8 and the evidence/log, and commit those changes.
Then write JSON to `ticket_file` with the ready file's `token` and the current
Git HEAD as `commit`. This is a builder phase handshake, not a human approval.
The helper waits at most 900 seconds, verifies the committed record and builder
fields, then invokes the official gated merge under the same lock. It releases
the resource on an error or timeout. Its terminal merge result must still be
checked before ending with DONE.

These earlier jobs are terminal:

- `20261114-054946-45357`, the old complete check, was stopped while waiting with
  no child so it could not compile a partially resolved rebase. The eight passed
  items retain their target proofs; final repository checks remain required.
- `20261114-074918-88657` refused the verifier's disk preflight before rendering.
  Space recovered to 21 GiB afterward. It produced no baseline pass.
- `20261114-094322-25542` safely aborted on the registration conflicts above.
  Those conflicts are resolved; rerun the helper from the current clean source.
- `20261114-101644-42314` rebased and built successfully, then the first native
  capture refused its disk preflight. It began with 16.0136 GiB free and fell
  below 16 GiB after the builds. No images were captured. The
  [diagnostic metadata](M2-disk-attempt.json.gz) is retained; its cooked scratch
  and this worktree's 2.3 GiB of build output were removed. Space recovered to
  about 20 GiB. The helper now reserves build headroom before qualification.

The [M2 record](M2.md), selected Metal records, [cost record](M2-cost.md) and
[CPU loop record](M2-loop.txt) retain the passed proofs. All three Metal targets
and three library tests passed at `4b1fbfa8`; ambient also passed at `4372f847`
and again after the later documentation-only checkpoints. No product code has
changed since those proofs. The new upstream cloud crate changes the exact
verifier's covered key, which is why a fresh integration proof is needed.

The [first baseline attempt](M2-baseline-attempt.json) stopped on an incomplete
native route-motion interval: 153 readback-ring omissions, zero writer omissions,
Metal backend. The cause is unconfirmed. Its complete diagnostic metadata is in
`tmp/sea-m2/baseline-first-failure.json.gz`; disposable pixels were removed.
No baseline reference or other module's test was changed.

After a successful current-source proof, retain its compact record, update M2.8,
the pending baseline line in M2's evidence and the final log, then complete the
repository gate and integration. The user authorizes verified pushes through
the integration script; the lead starts the reviewer. A rebase that changes
covered source needs a matching proof. Keep work in this worktree, and clean
owned scratch/build outputs at final closeout. No access request was refused.
