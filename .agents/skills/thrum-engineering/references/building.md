# Building in Thrum

What `/build-milestone` needs to know that is particular to this engine: what counts as proof, how capture works
here, what to verify, and how to leave the machine. The generic build skill covers the plan, the stop rule, review
and integration.

## A milestone is proven in the game

A milestone is proven when the V1 test game (`examples/v1-game`) uses the module the way a game will:

- in the game's own scene;
- through the module's public interface only;
- with shipped content and configuration;
- with the existing neighbours running alongside (sky, clouds, water, shadows);
- under the game's own controls.

It is then captured while running and watched in motion. Anything less, such as a solver test, a geometry
inspector or an offline high-sample render, is a diagnostic. A diagnostic is useful, but it proves nothing about
the product.

- **The first visual checkpoint renders the interaction itself.** If the milestone promises a wake behind a boat,
  the checkpoint shows a boat making a wake, not a plot of the wave field.
- **If the game can't host the milestone yet, making it able to is part of the work.** For example, there might be
  no sea for a shadow over water. Add the scene through public APIs, and use simple labelled stand-ins for
  neighbours that don't exist. A change that would alter another module's design goes to `/plan-module`.
- **Drive behaviour with its real inputs.** A correct solver fed invented inputs, or an amplitude set by a
  hand-tuned gain, still gives the wrong experience. An authored effect is fine when its role and limits are stated.
- **Test the joins.** Faults hide at transitions between representations, bands and levels of detail, not inside
  each part.
- **Rebuild before reusing evidence.** When a source or recipe changes, rebuild the affected products and recheck the
  loaded artifact before you carry evidence forward.

## Prove once, on main

After you merge and end with `DONE`, the script runs `./dev check` and every item's check on main's tip, which
holds your work. That run is the proof of record. The rules are in `ORCHESTRATE.md`, and
[the ruling](../../../../docs/plans/ARCHITECTURE.md#proofs-run-once-on-main) says why: in one day builders spent
most of the GPU's time proving again what a rebase had just made stale.

- **Run a check on your branch once, for your own confidence, then merge.** Don't run it again because main
  moved. If the script's run fails, it tells you which check and why, and that one is what you run again.
- **Write no script that rebases, proves and merges in one go.** `merge` already rebases and runs `./dev check`.
  A script of your own that holds a lock across those steps makes every builder wait through all of them, and
  71 of 84 such runs failed.
- **The exact images are not yours to prove.** An item whose check is `python3 scripts/exact_baselines.py verify`
  passes from one shared run on main after your merge. Set it `pass` with the evidence `main-branch check
  exact-images`. On your branch the command answers that the commit isn't on main yet, which is expected. If
  it later says main fails at a commit that isn't yours, say so in your report and don't work around it.
- **One capture pass for a visual milestone, after your last merge.** A capture from before a rebase shows a
  different build from the one under review.

## Capture

Engine capture renders the running game's frames through the shipped renderer offscreen, at the shipped simulation
rate and settings. It needs no display, no unlocked session and no Screen Recording grant, so it is how motion is
watched. The runbook is `docs/CAPTURE.md`. The capture modes each target supports are in `ORCHESTRATE.md`.

Keep the encoded clip, the per-frame GPU timings and a few cited stills. Delete raw frames once they are encoded.

Presentation (cadence and latency on the real display) is a separate measurement, made in the owner's window. It
gates only done-when items about pacing.

## Verify

- **Tests** cover behaviour and plausible failure. Authority work covers rejected, stale and duplicate input.
  Lifecycle work covers interruption and recovery.
- **Timing claims** come from `benches/` (divan, `=0.1.21`) with a stored baseline.
- **Allocation claims** come from a counting-allocator test, such as `examples/v1-game/tests/frame_alloc.rs`.
- **Parsers of untrusted bytes** (wire input, a remote product store) get a fuzz target. None exists yet, so the
  first sets the pattern.
- **Device, frame-time and visual claims** come only from runs on the targets in `ORCHESTRATE.md`: this M3 Max,
  Chrome, and the iPhone simulator. The simulator shows function only, never speed. Label a Mac standing in for an
  iPhone `proxy:iphone`.
- **Locks are about capacity.** `LOOP lock gpu -- <command>` shares the Mac's GPU between two runs, `lock chrome`
  gives Chrome to one run and `lock simulator` gives the iPhone simulator to one. Take the lock of the device the
  command loads most, around the part that needs it, and take only one: a command that holds one lock never
  asks for another and never runs `merge`. Start a GPU run with the script's `background`, carry on with CPU
  work, and collect it with `wait`. Waiting for a lock is progress, not a blocker.
- **`gpu.alone` is for a run that needs the whole GPU,** such as all its memory. It doesn't give a quiet machine,
  because other builders' builds and tests take no lock.
- **Timings inside a milestone are tripwires.** Record them with the load on the machine. They fail only on a
  collapse or a missing measurement. A module's qualified numbers are taken in its last milestone in one
  sitting: do every other item, then end `BLOCKED` asking the lead for a quiet machine and naming the items.
- **Attended runs check first, then lock.** Before a run that needs the console unlocked or the panel awake,
  check both. If either fails, end `BLOCKED needs_access` at once. Never wait for a person while holding a lock:
  one check held the GPU for over 30 minutes that way.
- **A change to a covered path** needs no proof before it merges. `./dev check` prints a warning for it, and the
  run on main verifies it. `CONTRIBUTING.md` has the details, and `.config/exact-baselines.json` lists the
  covered paths.
- **Regressions:** investigate a visible hitch, a regression of about 25% from a comparable baseline, or unexpected
  scaling.
- **Status** goes in `docs/design/maturity.json`, with implementation, correctness, visuals and device recorded
  separately. A check passing doesn't move any of them.
- **Evidence** is at most one page per milestone, linked from the done-when items. Bulk output follows
  `docs/ASSET-LIFECYCLE.md`.

## What to deliver

- the result;
- what was verified, with the evidence;
- the measured cost;
- what remains unqualified or failed.

An unavailable path, such as a fallback, a device or a resource-loss case, stays unqualified. It doesn't silently
pass. A bounded prototype is not full-range delivery, and an unrun check is not a failed one.

## Leaving the machine clean

- Run `cargo clean` in your worktree when work stops. A daily sweep deletes stale `target` folders, and builders
  stall when the disk falls below about 16 GiB.
- Remove the worktree once its commit has landed, that is, once `git merge-base --is-ancestor <commit> origin/main`
  passes after a fetch.
- In a fan-out, each stream keeps one target folder across its milestones.
