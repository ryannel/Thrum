# Build and prove

The outcome is the commissioned behaviour running in the engine on the shipped path, judged in motion against its
cues, with measured cost and honest limits.

## Work from the plan

Work through the plan: keep its status current, tick work off as it's done, and revise it as you learn.
A cue that won't come follows the stay-on-course rule in `/build-milestone`.

## Let the design choose the next slice

- **Choose by risk and dependency.** Pick the next slice by architectural risk and dependency, not by what extends
  the latest demo most easily. Name the requirement or decision it tests, the affected consumers and what would reject
  it.
- **Update the design after each proof.** Do this before treating the code as a foundation. If evidence overturns a
  structural assumption, revise the design and affected contracts before building dependent features. Small local
  fixes stay local.
- **Build the core behaviour and its consumers together.** Build the defining behaviour together with its important
  consumers before widening regimes or generalizing infrastructure. A diagnostic stub may exercise plumbing, but its
  real replacement stays on the critical path.

## Make the first visual checkpoint real

A milestone is proven by the game using the module as a game will: the test game's scene, the module's public
interface only, shipped content and configuration, whichever of its neighbours exist (sky, clouds, water, shadows) running alongside,
the game's own controls, and capture of the running game watched in motion. Anything less is a diagnostic.

- **Capture without the screen.** Render the running game's frames through the shipped renderer offscreen, at the
  shipped simulation rate and settings. Encode them at 1×, record each frame's GPU time, and review the clip; keep
  the clip, timings and a few cited stills, and delete raw frames once encoded. This needs no display or unlocked
  session, so it is how motion is watched. Presentation (cadence and latency on the real display) is measured
  separately, in the owner window, and gates only done-when items about pacing.

- **Render the interaction itself.** The first promised visual checkpoint is a rendered interaction with the required
  cue visible. A solver or geometry inspector is a diagnostic.
- **Exercise the shipped path.** Use the selected runtime path, configuration and budget. An offline or high-sample
  look shows what's possible; it doesn't show the product.
- **Make the game able to host the milestone.** If the test game can't yet show the scene a milestone needs (no sea
  for a shadow over water, say), adding that scene through public interfaces is work in your plan. Use simple stand-ins
  for neighbours that don't exist yet, label them, and take anything that would change another module's design to
  `/plan-project`.
- **Supply what the judgement needs.** Provide the content and surroundings needed to judge at the
  intended distances and speeds, and reject inadequate content before expensive preparation.
- **Rebuild before reusing evidence.** When a source or recipe changes, rebuild the affected products and recheck the
  loaded artifact before carrying evidence forward.

## Make the model produce the behaviour

- **Drive simulated behaviour with its real drivers.** Use established relationships and real inputs. A correct
  solver with invented inputs, or amplitudes set by hand-tuned gains, still gives the wrong experience. Authored
  effects are legitimate when their role and limits are stated.
- **Ask whether the model can produce a cue before tuning it.** When a cue stays missing or weak, check the model
  ([system diagnosis](system-diagnosis.md)). If it can't produce the cue, replace the mechanism and keep the
  independently qualified parts. Raise resolution, sample counts or preparation cost only when they address the
  identified failure.
- **Test the joins.** Test transitions and overlaps between representations, bands and levels of detail, not only
  each part's resolution. Use [system diagnosis](system-diagnosis.md) to separate an implementation error from a
  missing or incompatible model.
- **Keep the code where it belongs.** Keep pure calculation, GPU work and orchestration with their owners. Share
  invariant data and contracts rather than duplicating formulas or keeping competing producers alive.

## Look, measure, review

- **Look at the output yourself.** Watch stills and motion against the inspection list, including normal motion,
  starts and stops, turns, camera and scale changes, and quality transitions. Judge at the player's camera, distance and
  real frame rate, against the milestone's inspection list and its shipped-game bar; close views and offline-paced clips are diagnostics.
  Tests and source review don't replace looking.
- **Vary the suspects.** When a defect persists, vary the variables that could plausibly cause it; don't hold them
  fixed across trials. Give each experiment a question, a prediction and a stopping point.
- **Independent visual review is the evaluator's.** For milestones with `review: true`, the clean-context evaluator
  judges the sequences and records `agent_reviewed`. Say plainly when output is poor. Owner feedback steers later slices; it is never a pause.
- **Tie evidence to the build.** Tie captures and timings to the actual loaded build, inputs and quality state.
- **Measure cost on the assembled path.** For performance-sensitive work, follow [performance](performance.md),
  ending with an assembled measurement. Label timings taken on a shared, contended GPU as such.
- **Review, fix, recheck.** Use [review](review.md) for design, code, output and performance claims. Resolve
  actionable findings, then check whether a fix weakened another cue.

Deliver:
- the result;
- what was verified, with evidence;
- the measured cost;
- what remains unqualified or failed.

Keep a bounded prototype distinct from full-range delivery, and keep unrun qualification distinct from a known failure.
Unavailable paths (a fallback, a device, a resource-loss case) stay unqualified; they don't silently pass. For
evolving structure, check local development and disappearance, not only persistent identity. Commit and push verified checkpoints according to the
repository's rules, and preserve others' work. Using this skill grants no extra permission for external actions.
