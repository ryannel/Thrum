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
verified findings; investigate suspected ones before relying on them.

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
  good lighting or performance doesn't compensate for failed dynamics. Reviews diagnose the responsible layer
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

Narrowing a claim removes an overclaim; it doesn't deliver the excluded experience. A review report alone doesn't
close the gap it found.

Verify consequential fixes against the revised assembly. Old reviews apply only to the revision they inspected. Agreement
on a design is not evidence of running behaviour, visual quality or cost.
