# Independent review

Review exists to catch what the author can't see. Design, code correctness, visual output and performance claims are
different questions; say which one a review answered and which artifact revision it inspected. Choose depth by
consequence and uncertainty, not by a quota.

## Brief reviewers so they stay independent

Give a fresh reviewer the original requirements, the use cases and the raw artifacts (code, design, captures,
measurements). Don't give it your preferred technique, expected verdict or research conversation: an agent handed a
hypothesis tends to confirm it. Ask neutral questions. For blue-sky design review, keep the reviewer away from
existing module code and old plans, or disclose that it saw them. Self-checks are not independent review. If
delegation is unavailable, say so. If a reviewer contributes a new mechanism, another reader verifies it. Reviewers
don't claim a full read of scope they didn't inspect.

Ask reviewers to mark each finding as verified, with file and line or a reproduction, or as suspected. Then act on
verified findings; investigate suspected ones before relying on them. Ask for everything that fails the criteria,
ranked; don't ask only for blocking issues.

## The two reviews that matter most

- **The design and plan, before building on them.** Give the reviewer the brief (the commission: what the module
  is for, its consumers, the required experiences and constraints, without your design) first and have it derive what the
  module must do, then compare with the design and plan: omissions, unsupported additions, mechanisms not shown to
  make their cues, milestones that wouldn't show the behaviour, a milestone too large to reach the game soon, and proof
  that pursues accuracy beyond what changes what the player sees.
- **Each milestone, in the game.** Give the reviewer how the milestone is judged and the raw captures and logs for
  one build, not your summary. It says, per check, pass, fail or can't tell, and whether it watched motion.

The reviewer's verdict goes in the design or plan with the revision or build it saw, together with the brief it was
given; it applies only to that revision. A consequential technique chosen after the design review gets the same kind
of review before it's built on.

## What to challenge

- **Technique choice.** Review it when the choice is made, not after hours of building on it. Can the mechanism
  produce every defining cue across the operating range? A numerical reproduction of the core maths often settles this
  quickly. Which modern alternatives were missed? Is the method heavy or dated by design?
- **Coverage.** Have the reviewer derive the capabilities and interactions from the brief first, then compare them with
  the design. Keep omissions, unsupported additions and incompatible couplings separate. An explicit, safe deferral
  is not an omission.
- **Composition.** Walk a normal frame and a coupled interaction through the actual producers and consumers. Where
  would an implementer still have to invent a mechanism, query or update order? A rigorous component doesn't support
  an unbuilt link elsewhere.
- **Plausible wrong substitutes.** Does the mechanism produce the requested state change, or only animate its
  presentation (moving texture for growth, a fade for breakup)? Could it pass the author's checks while missing the
  goal?
- **Range and shortcuts.** Look for fixed domains, constant depths, hard-coded origins, wrapping boundaries,
  single-regime constants, or a local field that replaces, rather than composes with, the module's other regimes.
- **Code across boundaries.** Read callers, shared data, shaders, fixtures, lifecycle and tools together. Look for
  duplicate producers, mismatched clocks or encodings, dead alternative paths, unbounded catch-up loops, hot-path
  allocation, unnecessary readbacks and redundant transforms.
- **Visual output.** Reviewers inspect captured images and sequences directly. They state whether they watched motion
  or sampled stills, at native size or downscaled, and which build they saw. Judge each indispensable cue separately;
  good lighting or performance doesn't compensate for failed dynamics. Motion review covers the transitions as well as the steady state: the start from rest, stops, speed changes, turns and camera cuts. Onset faults hide in steady-speed footage. Reviews diagnose the responsible layer
  ([system diagnosis](system-diagnosis.md)) rather than prescribing cosmetic fixes.
- **Performance claims.** Challenge benchmark identity and fairness, tails, equivalence claims, actual saved work,
  memory accounting and contention on a shared GPU ([performance](performance.md)).

## Close the loop

A finding names:
- the affected decision or location;
- the defect or unsupported claim;
- the evidence;
- the consequence;
- what would resolve it.

Don't invent objections or demand a paper without showing why it changes the decision.

The author then does one of these:
- revises the model or contracts;
- supplies evidence;
- rejects the finding with reasons;
- records a genuine open decision with a discriminating experiment.

Fix the class a finding belongs to, not only the instance the reviewer named: search the change for the same kind of
fault before bringing it back. When you resubmit a held change, answer each point of the latest hold in turn: how it's now met, or why it doesn't apply. A revision that skips a held point comes back held. Narrowing a claim removes an overclaim; it doesn't deliver the excluded experience. A review report alone doesn't
close the gap it found.

Verify consequential fixes against the revised assembly. Old reviews apply only to the revision they inspected. Agreement
on a design is not evidence of running behaviour, visual quality or cost.

## Calibration: failures this engine has already paid for

Reviewers look for these first; each one passed an author's own checks.

- **Tuning a mechanism that can't make the cue.** Hours of gain, resolution, foam and shading work went into a wake
  whose wave model couldn't form a Kelvin wedge at varying speed. A short CPU reproduction showed it.
- **Correct but invisible.** Physically calibrated wakes and foam were true to scale and unreadable at the game camera.
- **Judged in the lab.** A cloud look judged at seconds per frame shipped as a 288 ms fog. Critics gated on close
  views and offline-paced clips while the real run stuttered.
- **Trials that hold the suspect fixed.** Six rounds of horizon-shelf fixes all kept base height, profile and
  lighting constant.
- **A plan that only grows.** A cloud plan grew to 399 lines of amendments, and a water plan was a list of effects in
  build order with the speed range never designed. Neither had a criterion that moved.
- **Method families without a governing question.** Nine or more cloud method families were trialled, none with
  a question it answered or a point at which to stop.
- **Swinging between cost-only and look-only work.** Chasing invented budgets before the look existed, then ignoring
  cost, produced designs that were heavy by design.
- **Copying the better-looking prototype.** An earlier implementation looked better but was too heavy; it was a look
  bar, not a technique.
- **Attractive frames without the behaviour.** Moving texture passed for growth and breakup. A fade or age label is
  not structural change.
- **Paperwork displacing the defect.** Notes and qualification grew while the visible defect stayed.
- **Verdicts from descriptions.** Techniques were endorsed from their write-ups and critics' passes were relayed as
  fact, then proved wrong.
