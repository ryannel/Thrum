# Diagnose a fault at its system boundary

Use this when an artifact, mismatch, weak cue or unstable interaction might come from the model, the representation,
ownership or integration rather than from a local bug. The outcome is a correction at the responsible layer that
restores the intended behaviour, verified across its consumers. A local bug can still have a local fix.

## Find the cause before changing parameters

- **Reproduce first.** Use known inputs and a known build. Hold the camera, lighting or presentation constant where
  that helps, and compare intermediate state, not only the final image. Separate a local appearance jump from an
  application-wide hitch.
- **Ask whether the model can produce the missing behaviour at all.** Reproduce the core equations outside the engine,
  for example in a short CPU script with the shipped constants, and compare the result with the cue and a reference.
  This often settles in minutes what visual iteration doesn't settle in hours.
- **Trace the path.** Follow it from source through state, transforms, reconstruction and composition to every
  important consumer. Ask who owns the quantity and whether another path derives a conflicting version. A visible
  fault can start upstream in support, timing, encoding, sampling or resource lifecycle.
- **Look for one shared cause.** When several symptoms appear together, such as a weak wake, no wake when turning,
  sudden starts and stops and missing foam, look for a common cause before fixing each one: a frozen input, a pinned
  domain, a filter cancelling the signal, or state that stops updating after a hitch.
- **Classify the failure:**
  - an implementation mistake;
  - a missing or incompatible mechanism;
  - missing input or content;
  - calibration;
  - a representation or scale error;
  - a presentation fault;
  - a lifecycle fault (hitch, reset, resize, device loss).

  Correct it at that layer. Damping a response, turning a term off or adding noise can isolate a defect or hide it;
  keep such a change only if it restores the intended contract.
- **Bound the next experiment.** After a negative result, choose a bounded experiment that can change the responsible
  decision. Don't run a more expensive version of the same test. More resolution can't supply an absent mechanism or
  fix another quantity's calibration.

## Make shared representations explicit

For each quantity that crosses a boundary, state:
- its source or band;
- units and coordinate frame;
- spatial and temporal support;
- physical time;
- encoding;
- composition rule;
- each consumer's fidelity.

Where consumers must agree, use one authority. Others may approximate it within a stated bound.

- **Derived quantities follow their source.** Displacement and its shading differential need compatible derivatives.
  A changing frequency needs an integrated phase.
- **Detail is resolved, filtered or statistical; never counted twice.** Filtering one representation may need a
  matching statistical response in another. Never count a contribution twice, or amplify unresolved detail until it
  resembles the target.
- **Shared resources live where they're reused,** for example as a view or frame service, not allocated by every
  instance.

## Verify the correction

- **Test against a reference.** Write tests that expose the broken relationship, with an analytic or independent
  reference where one is meaningful. Two evaluators that share an omission can't bound it.
- **Exercise the conditions.** Cover transforms, physical time, source tails, mixed contributions, resolution
  changes, hitches and fallback paths.
- **Look again in motion.** Re-inspect the ordinary output in motion and the adjacent consumers, and measure the
  cost.
- **Record what you learnt.** Record what the ablation established, why the repair preserves the design, and what
  remains unproved.
